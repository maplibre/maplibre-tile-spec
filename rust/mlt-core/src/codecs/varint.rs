use integer_encoding::VarInt;

use crate::utils::AsUsize as _;
use crate::{Decoder, MltError, MltRefResult};

/// Parse a single varint (variable-length integer) from the input, returning
/// the remaining bytes and the decoded value.
///
/// Validates canonical encoding: a multibyte varint must not have a trailing
/// zero byte (which would mean it could have been encoded in fewer bytes).
#[inline]
pub fn parse_varint<T: VarInt>(input: &[u8]) -> MltRefResult<'_, T> {
    match T::decode_var(input) {
        Some((value, consumed)) => {
            // A varint is canonical if its last byte is non-zero (for multibyte encodings).
            // Value 0 must be encoded as a single 0x00 byte.
            // For multibyte VarInts, the last byte (without a continuation bit) must be non-zero.
            // Using more bytes than necessary violates roundtrip-ability of MLT.
            if consumed > 1 && input[consumed - 1] == 0 {
                return Err(MltError::NonCanonicalVarInt);
            }
            Ok((&input[consumed..], value))
        }
        None => Err(MltError::BufferUnderflow(
            u32::try_from(input.len().saturating_add(1))?,
            input.len(),
        )),
    }
}

/// Parse `size` varints of wire type `T` into a `Vec<U>`, charging `dec` for
/// the output allocation.
pub fn parse_varint_vec<'a, T, U>(
    mut input: &'a [u8],
    size: u32,
    dec: &mut Decoder,
) -> MltRefResult<'a, Vec<U>>
where
    T: VarInt,
    U: TryFrom<T>,
    MltError: From<<U as TryFrom<T>>::Error>,
{
    let mut values = dec.alloc::<U>(size.as_usize())?;
    let mut val;
    for _ in 0..size {
        (input, val) = parse_varint::<T>(input)?;
        values.push(val.try_into()?);
    }
    Ok((input, values))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::MltResult;
    use crate::test_helpers::dec;

    #[rstest]
    #[case::trailing_bytes(&[0x80, 0x01, 0x42], Ok((vec![0x42_u8], 128)))]
    #[case::zero(&[0x00], Ok((vec![], 0)))]
    #[case::max_single_byte(&[0x7F], Ok((vec![], 127)))]
    #[case::min_two_byte(&[0x80, 0x01], Ok((vec![], 128)))]
    #[case::max_two_byte(&[0xFF, 0x7F], Ok((vec![], 16383)))]
    #[case::min_three_byte(&[0x80, 0x80, 0x01], Ok((vec![], 16384)))]
    #[case::non_canonical_two(&[0x82, 0x00], Err(MltError::NonCanonicalVarInt))]
    #[case::non_canonical_three_byte(&[0x80, 0x80, 0x00], Err(MltError::NonCanonicalVarInt))]
    #[case::single_byte_with_trailing(&[0x01, 0x02, 0x03], Ok((vec![2, 3], 1)))]
    #[case::underflow(&[0x80, 0x80, 0x80], Err(MltError::BufferUnderflow(4, 3)))]
    fn test_varint_parsing(#[case] bytes: &[u8], #[case] expected: MltResult<(Vec<u8>, u32)>) {
        let actual = parse_varint::<u32>(bytes);
        // matching because MltError cannot implement PartialEq
        // effectively assert_eq!(actual, expected);
        match (actual, expected) {
            (Ok((v1, s1)), Ok((v2, s2))) => assert_eq!((v1, s1), (v2.as_slice(), s2)),
            (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_parse_varint_vec() {
        // Encode [1u32, 2, 3] as varints and parse back.
        let mut buf = Vec::new();
        let mut buf_tmp = vec![0u8; 10];
        for v in [1u32, 2, 3] {
            let written = v.encode_var(&mut buf_tmp);
            buf.extend_from_slice(&buf_tmp[0..written]);
        }
        let (remaining, values) =
            parse_varint_vec::<u32, u32>(&buf, 3, &mut dec()).expect("parse_varint_vec failed");
        assert!(remaining.is_empty());
        assert_eq!(values, [1, 2, 3]);
    }
}
