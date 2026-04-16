use crate::codecs::varint::parse_varint;
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

/// Apply an optional present bitmap to a vector of values.
/// If the presence stream is absent (non-optional column), all values are wrapped in Some.
/// If present, values are interleaved with None according to the bitmap.
pub fn apply_present<T>(
    presence: RawPresence<'_>,
    values: Vec<T>,
    dec: &mut Decoder,
) -> MltResult<Vec<Option<T>>> {
    let present: Vec<bool> = if let Some(p) = presence.0 {
        p.decode_bools(dec)?
    } else {
        let mut result = dec.alloc::<Option<T>>(values.len())?;
        result.extend(values.into_iter().map(Some));
        return Ok(result);
    };
    let present_bit_count = present.iter().filter(|&&b| b).count();
    if present_bit_count != values.len() {
        return Err(MltError::PresenceValueCountMismatch(
            present_bit_count,
            values.len(),
        ));
    }
    debug_assert!(
        values.len() <= present.len(),
        "Since the number of present bits is an upper bound on the number of values and equals values.len(), there cannot be more values than entries in the present bitmap"
    );

    let mut result = dec.alloc::<Option<T>>(present.len())?;
    let mut val_iter = values.into_iter();
    for p in present {
        result.push(if p { val_iter.next() } else { None });
    }
    Ok(result)
}
