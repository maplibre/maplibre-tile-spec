use crate::codecs::varint::parse_varint;
use crate::{MltError, MltRefResult};

#[inline]
pub fn take(input: &[u8], size: u32) -> MltRefResult<'_, &[u8]> {
    let (value, input) = input
        .split_at_checked(size.try_into()?)
        .ok_or(MltError::UnableToTake(size))?;
    Ok((input, value))
}

/// Parse a length-prefixed UTF-8 string from the input
pub fn parse_string(input: &[u8]) -> MltRefResult<'_, &str> {
    let (input, length) = parse_varint::<u32>(input)?;
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
