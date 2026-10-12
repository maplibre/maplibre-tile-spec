use crate::codecs::varint::parse_varint;
use crate::utils::PresentValues;
use crate::{Decoder, MltError, MltRefResult, MltResult, RawPresence};

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

/// Decode an optional presence stream, combining it with the dense values.
/// A non-optional column has every feature present.
/// Otherwise decodes the bitvector and checks its set-bit count matches `values.len()`.
pub fn decode_presence<'a, T: Copy>(
    presence: RawPresence<'a>,
    values: Vec<T>,
    dec: &mut Decoder,
) -> MltResult<PresentValues<'a, T>> {
    let Some(bits) = presence.decode_bits(dec)? else {
        return Ok(PresentValues::all_present(values));
    };
    let set_count = bits.count_ones();
    let dense_count = values.len();
    if set_count != dense_count {
        return Err(MltError::PresenceValueCountMismatch(set_count, dense_count));
    }
    Ok(PresentValues::with_bits(bits, values))
}
