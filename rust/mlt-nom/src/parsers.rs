use integer_encoding::VarInt;
use nom::Err::Error as NomError;
use nom::IResult;
use nom::bytes::complete::take;
use nom::error::{Error, ErrorKind};

/// Parse a varint (variable-length integer) from the input
pub fn parse_varint(input: &[u8]) -> IResult<&[u8], u64> {
    match u64::decode_var(input) {
        Some((value, consumed)) => Ok((&input[consumed..], value)),
        None => Err(NomError(Error::new(input, ErrorKind::Fail))),
    }
}

/// Parse a varint (variable-length integer) from the input and convert to usize
pub fn parse_varint_usize(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, value) = parse_varint(input)?;
    let value = usize::try_from(value);
    let value = value.or(Err(NomError(Error::new(input, ErrorKind::TooLarge))))?;
    Ok((input, value))
}

/// Parse a varint (variable-length integer) from the input and convert to u32
pub fn parse_varint_u32(input: &[u8]) -> IResult<&[u8], u32> {
    let (input, value) = parse_varint(input)?;
    let value = u32::try_from(value);
    let value = value.or(Err(NomError(Error::new(input, ErrorKind::TooLarge))))?;
    Ok((input, value))
}

/// Parse a length-prefixed UTF-8 string from the input
pub fn parse_string(input: &[u8]) -> IResult<&[u8], &str> {
    let (input, length) = parse_varint_usize(input)?;
    let (input, value) = take(length)(input)?;
    let value = str::from_utf8(value);
    let value = value.or(Err(NomError(Error::new(input, ErrorKind::Fail))))?;
    Ok((input, value))
}

/// Parse a single byte from the input
pub fn parse_u8(input: &[u8]) -> IResult<&[u8], u8> {
    if input.is_empty() {
        Err(NomError(Error::new(input, ErrorKind::Eof)))
    } else {
        Ok((&input[1..], input[0]))
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
