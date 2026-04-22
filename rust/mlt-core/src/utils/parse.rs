use crate::codecs::varint::parse_varint;
use crate::utils::Presence;
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

/// Decode an optional presence stream, validating it against a dense value count.
///
/// Returns [`Presence::AllPresent`] when `presence.0` is `None` (non-optional column).
/// Otherwise decodes the bitvector and checks that the number of set bits equals
/// `dense_count` (the number of non-null values already decoded).
pub fn decode_presence<'a>(
    presence: RawPresence<'a>,
    dense_count: usize,
    dec: &mut Decoder,
) -> MltResult<Presence<'a>> {
    let Some(raw) = presence.0 else {
        return Ok(Presence::AllPresent);
    };
    let bits = raw.decode_bitvec(dec)?;
    let set_count = bits.count_ones();
    if set_count != dense_count {
        return Err(MltError::PresenceValueCountMismatch(set_count, dense_count));
    }
    Ok(Presence::Bits(bits))
}
