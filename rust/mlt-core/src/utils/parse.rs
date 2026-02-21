use integer_encoding::VarInt;

use crate::{MltError, MltRefResult};

#[inline]
pub fn take(input: &[u8], size: usize) -> MltRefResult<'_, &[u8]> {
    let (value, input) = input
        .split_at_checked(size)
        .ok_or(MltError::UnableToTake(size))?;
    Ok((input, value))
}

pub fn all<T>((input, value): (&[u8], T)) -> Result<T, MltError> {
    if input.is_empty() {
        Ok(value)
    } else {
        Err(MltError::BufferUnderflow {
            needed: input.len(),
            remaining: 0,
        })
    }
}

/// Parse a varint (variable-length integer) from the input
pub fn parse_varint<T: VarInt>(input: &[u8]) -> MltRefResult<'_, T> {
    match T::decode_var(input) {
        Some((value, consumed)) => {
            // Validate canonical encoding:
            // Check that the value couldn't fit in fewer bytes
            //
            // A varint is canonical if its last byte is non-zero (for multibyte encodings).
            // Value 0 must be encoded as a single 0x00 byte.
            // For multibyte varints, the last byte (without continuation bit) must be non-zero.
            //
            // This ensures we're not using more bytes than necessary.
            // Using more bytes is an issue as it does violate the roundtrip-ability of MLT
            if consumed > 1 && input[consumed - 1] == 0 {
                return Err(MltError::NonCanonicalVarInt);
            }
            Ok((&input[consumed..], value))
        }
        None => Err(MltError::BufferUnderflow {
            needed: input.len() + 1,
            remaining: input.len(),
        }),
    }
}

pub fn parse_varint_vec<T, U>(mut input: &[u8], size: u32) -> MltRefResult<'_, Vec<U>>
where
    T: VarInt,
    U: TryFrom<T>,
    MltError: From<<U as TryFrom<T>>::Error>,
{
    let mut values = Vec::with_capacity(usize::try_from(size)?);
    let mut val;
    for _ in 0..size {
        (input, val) = parse_varint::<T>(input)?;
        values.push(val.try_into()?);
    }
    Ok((input, values))
}

/// Parse a length-prefixed UTF-8 string from the input
pub fn parse_string(input: &[u8]) -> MltRefResult<'_, &str> {
    let (input, length) = parse_varint::<usize>(input)?;
    let (input, value) = take(input, length)?;
    let value = str::from_utf8(value)?;
    Ok((input, value))
}

/// Parse a single byte from the input
pub fn parse_u8(input: &[u8]) -> MltRefResult<'_, u8> {
    if input.is_empty() {
        Err(MltError::UnableToTake(1))
    } else {
        Ok((&input[1..], input[0]))
    }
}

/// Parse a single byte from the input when we know the value is less than 128
pub fn parse_u7(input: &[u8]) -> MltRefResult<'_, u8> {
    let (input, value) = parse_u8(input)?;
    if value < 128 {
        Ok((input, value))
    } else {
        Err(MltError::Parsing7BitInt(value))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
    #[case::underflow(&[0x80, 0x80, 0x80], Err(MltError::BufferUnderflow { needed: 4, remaining: 3 }))]
    fn test_varint_parsing(
        #[case] bytes: &[u8],
        #[case] expected: Result<(Vec<u8>, usize), MltError>,
    ) {
        let actual = parse_varint::<usize>(bytes);
        // matching because MltError cannot implement PartialEq
        // effectively assert_eq!(actual, expected);
        match (actual, expected) {
            (Ok((v1, s1)), Ok((v2, s2))) => assert_eq!((v1, s1), (v2.as_slice(), s2)),
            (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("Unexpected result"),
        }
    }
}
