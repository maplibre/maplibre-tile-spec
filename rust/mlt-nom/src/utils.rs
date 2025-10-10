use integer_encoding::VarInt;

use crate::MltError::Fail;
use crate::{MltError, MltResult};

/// Parse a varint (variable-length integer) from the input
pub fn parse_varint<T: VarInt>(input: &[u8]) -> MltResult<'_, T> {
    match VarInt::decode_var(input) {
        Some((value, consumed)) => Ok((&input[consumed..], value)),
        None => Err(Fail),
    }
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
pub fn parse_varint_vec<T, U>(mut input: &[u8], size: usize) -> MltResult<'_, Vec<U>>
where
    T: VarInt,
    U: TryFrom<T>,
    MltError: From<<U as TryFrom<T>>::Error>,
{
    let mut values = Vec::with_capacity(size);
    let mut val;
    for _ in 0..size {
        (input, val) = parse_varint::<T>(input)?;
        values.push(val.try_into()?);
    }
    Ok((input, values))
}

/// Parse a length-prefixed UTF-8 string from the input
pub fn parse_string(input: &[u8]) -> MltResult<'_, &str> {
    let (input, length) = parse_varint::<usize>(input)?;
    let (input, value) = take(input, length)?;
    let value = str::from_utf8(value)?;
    Ok((input, value))
}

/// Parse a single byte from the input
pub fn parse_u8(input: &[u8]) -> MltResult<'_, u8> {
    if input.is_empty() {
        Err(Fail)
    } else {
        Ok((&input[1..], input[0]))
    }
}

/// Parse a single byte from the input when we know the value is less than 128
pub fn parse_u7(input: &[u8]) -> MltResult<'_, u8> {
    let (input, value) = parse_u8(input)?;
    if value < 128 {
        Ok((input, value))
    } else {
        Err(Fail)
    }
}

/// Helper function to encode a varint using integer-encoding
pub fn encode_varint(data: &mut Vec<u8>, value: u64) {
    data.extend_from_slice(&value.encode_var_vec());
}

pub fn encode_str(data: &mut Vec<u8>, value: &[u8]) {
    encode_varint(data, value.len() as u64);
    data.extend_from_slice(value);
}

#[inline]
pub fn take<'a>(input: &'a [u8], size: usize) -> MltResult<'a, &'a [u8]> {
    let (value, input) = input.split_at_checked(size).ok_or(Fail)?;
    Ok((input, value))
}
