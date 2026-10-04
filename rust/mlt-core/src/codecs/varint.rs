use integer_encoding::VarInt;
use usize_cast::IntoUsize as _;

use crate::{Decoder, MltError, MltRefResult, MltResult};

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
            u32::try_from(input.len().saturating_add(1)).unwrap_or(u32::MAX),
            input.len(),
        )),
    }
}

/// Parse `size` varints of wire type `T` into a `Vec<T>`, charging `dec` for
/// the output allocation.
pub fn parse_varint_vec<'a, T: VarInt + From<u8>>(
    mut input: &'a [u8],
    size: u32,
    dec: &mut Decoder,
) -> MltRefResult<'a, Vec<T>> {
    const CONTINUATION_BITS: u64 = 0x8080_8080_8080_8080;
    let mut remaining = size.into_usize();
    let mut values = dec.alloc::<T>(remaining)?;
    while remaining > 0 {
        if remaining >= 8
            && let Some((chunk, rest)) = input.split_first_chunk::<8>()
            && u64::from_le_bytes(*chunk) & CONTINUATION_BITS == 0
        {
            values.extend(chunk.iter().map(|&b| T::from(b)));
            input = rest;
            remaining -= 8;
            continue;
        }
        let val;
        (input, val) = parse_varint::<T>(input)?;
        values.push(val);
        remaining -= 1;
    }
    Ok((input, values))
}

/// Parse varints of wire type `T` until `input` is exhausted, charging `dec`
/// for the output allocation.
///
/// Used for v2 interleaved-RLE streams, whose `(run, value)` pair count is not
/// stored on the wire - the data is scanned to the stream's `byte_length`.
/// Every varint occupies at least one byte, so `input.len()` bounds the
/// element count for the budget pre-charge.
pub fn parse_varint_vec_all<T: VarInt>(mut input: &[u8], dec: &mut Decoder) -> MltResult<Vec<T>> {
    let alloc_size = input.len();
    let mut values = dec.alloc::<T>(alloc_size)?;
    while !input.is_empty() {
        let val;
        (input, val) = parse_varint::<T>(input)?;
        values.push(val);
    }
    dec.adjust_alloc(&values, alloc_size).expect(
        "infallible: every varint consumes at least one byte, so values.len() <= alloc_size",
    );
    Ok(values)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::MltResult;
    use crate::test_helpers::{dec, starved_dec};

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
        match (actual, expected) {
            (Ok((v1, s1)), Ok((v2, s2))) => assert_eq!((v1, s1), (v2.as_slice(), s2)),
            (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_parse_varint_vec() {
        let mut buf = Vec::new();
        let mut buf_tmp = vec![0u8; 10];
        for v in [1u32, 2, 3] {
            let written = v.encode_var(&mut buf_tmp);
            buf.extend_from_slice(&buf_tmp[0..written]);
        }
        let (remaining, values) =
            parse_varint_vec::<u32>(&buf, 3, &mut dec()).expect("parse_varint_vec failed");
        assert_eq!(remaining, [] as [u8; 0]);
        assert_eq!(values, [1, 2, 3]);
    }

    #[rstest]
    #[case::all_single_byte_aligned(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])]
    #[case::multi_byte_first(&[300, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])]
    #[case::multi_byte_inside_a_window(&[1, 2, 3, 70_000, 5, 6, 7, 8, 9, 10, 11, 12])]
    #[case::multi_byte_last(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, u32::MAX])]
    #[case::shorter_than_a_window(&[1, 2, 3])]
    fn parse_varint_vec_matches_value_by_value_parsing(#[case] values: &[u32]) {
        let mut buf = Vec::new();
        for v in values {
            let mut tmp = [0u8; 10];
            let written = v.encode_var(&mut tmp);
            buf.extend_from_slice(&tmp[..written]);
        }
        buf.extend_from_slice(&[0x7F, 0x7F]);
        let count = u32::try_from(values.len()).unwrap();
        let (rest, parsed) = parse_varint_vec::<u32>(&buf, count, &mut dec()).unwrap();
        assert_eq!(parsed, values);
        assert_eq!(rest, [0x7F, 0x7F]);
    }

    #[test]
    fn parse_varint_vec_all_reads_every_varint_to_the_end() {
        let mut buf = Vec::new();
        let mut tmp = vec![0u8; 10];
        for v in [1u32, 300, 70_000] {
            let written = v.encode_var(&mut tmp);
            buf.extend_from_slice(&tmp[..written]);
        }
        assert_eq!(
            parse_varint_vec_all::<u32>(&buf, &mut dec()).unwrap(),
            [1, 300, 70_000]
        );
    }

    #[test]
    fn parse_varint_vec_all_accepts_an_empty_stream() {
        assert_eq!(
            parse_varint_vec_all::<u32>(&[], &mut dec()).unwrap(),
            [] as [u32; 0]
        );
    }

    #[test]
    fn a_truncated_varint_in_a_counted_vec_is_rejected() {
        let err = parse_varint_vec::<u32>(&[0x80], 1, &mut dec()).unwrap_err();
        assert!(matches!(err, MltError::BufferUnderflow(2, 1)), "{err:?}");
    }

    #[test]
    fn a_truncated_varint_in_an_unbounded_stream_is_rejected() {
        let err = parse_varint_vec_all::<u32>(&[0x80], &mut dec()).unwrap_err();
        assert!(matches!(err, MltError::BufferUnderflow(2, 1)), "{err:?}");
    }

    #[test]
    fn a_counted_vec_past_the_memory_budget_is_rejected() {
        let err = parse_varint_vec::<u32>(&[1, 2], 2, &mut starved_dec()).unwrap_err();
        assert!(
            matches!(err, MltError::MemoryLimitExceeded { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn an_unbounded_stream_past_the_memory_budget_is_rejected() {
        let err = parse_varint_vec_all::<u32>(&[1, 2], &mut starved_dec()).unwrap_err();
        assert!(
            matches!(err, MltError::MemoryLimitExceeded { .. }),
            "{err:?}"
        );
    }
}
