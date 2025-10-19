use std::fmt::{Debug, Display, Formatter};

use integer_encoding::VarInt;
use num_traits::{AsPrimitive, PrimInt};
use zigzag::ZigZag;

use crate::{MltError, MltRefResult};

/// Parse a varint (variable-length integer) from the input
pub fn parse_varint<T: VarInt>(input: &[u8]) -> MltRefResult<'_, T> {
    match VarInt::decode_var(input) {
        Some((value, consumed)) => Ok((&input[consumed..], value)),
        None => Err(MltError::ParsingVarInt),
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

/// Helper function to encode a varint using integer-encoding
pub fn encode_varint(data: &mut Vec<u8>, value: u64) {
    data.extend_from_slice(&value.encode_var_vec());
}

pub fn encode_str(data: &mut Vec<u8>, value: &[u8]) {
    encode_varint(data, value.len() as u64);
    data.extend_from_slice(value);
}

#[inline]
pub fn take(input: &[u8], size: usize) -> MltRefResult<'_, &[u8]> {
    let (value, input) = input
        .split_at_checked(size)
        .ok_or(MltError::UnableToTake(size))?;
    Ok((input, value))
}

/// Decode ([`ZigZag`] + delta) for Vec2s
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag>(data: &[T::UInt]) -> Result<Vec<T>, MltError> {
    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(MltError::InvalidPairStreamSize(data.len()));
    }

    let mut result = Vec::with_capacity(data.len());
    let mut last1 = T::zero();
    let mut last2 = T::zero();

    for i in (0..data.len()).step_by(2) {
        last1 = T::decode(data[i]) + last1;
        last2 = T::decode(data[i + 1]) + last2;
        result.push(last1);
        result.push(last2);
    }

    Ok(result)
}

/// Decode a vector of ZigZag-encoded unsigned deltas.
pub fn decode_zigzag_delta<T: Copy + ZigZag + AsPrimitive<U>, U: 'static + Copy>(
    data: &[T::UInt],
) -> Vec<U> {
    data.iter()
        .scan(T::zero(), |state, &v| {
            let decoded_delta = T::decode(v);
            *state = *state + decoded_delta;
            Some((*state).as_())
        })
        .collect()
}

/// Decode RLE (Run-Length Encoding) data
/// It serves the same purpose as the `decodeUnsignedRLE` and `decodeRLE` methods in the Java code.
pub fn decode_rle<T: PrimInt + Debug>(
    data: &[T],
    runs: usize,
    num_rle_values: usize,
) -> Result<Vec<T>, MltError> {
    let (run_lens, values) = data.split_at(runs);
    let mut result = Vec::with_capacity(num_rle_values);
    for (&run, &val) in run_lens.iter().zip(values.iter()) {
        let run_len = run
            .to_usize()
            .ok_or_else(|| MltError::RleRunLenInvalid(run.to_i128().unwrap_or_default()))?;
        result.extend(std::iter::repeat_n(val, run_len));
    }
    Ok(result)
}

pub trait SetOptionOnce<T> {
    fn set_once(&mut self, value: T) -> Result<(), MltError>;
}

impl<T> SetOptionOnce<T> for Option<T> {
    fn set_once(&mut self, value: T) -> Result<(), MltError> {
        if self.replace(value).is_some() {
            Err(MltError::DuplicateValue)
        } else {
            Ok(())
        }
    }
}

/// Wrapper type for optional slices to provide a custom Debug implementation
pub struct OptSeq<'a, T>(pub Option<&'a [T]>);

impl<T: Display + Debug> Debug for OptSeq<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = self.0 {
            write!(
                f,
                "[{}{}; {}]",
                v.iter()
                    .take(8)
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                if v.len() > 8 { ", ..." } else { "" },
                v.len()
            )
        } else {
            write!(f, "None")
        }
    }
}
