use std::fmt::{Debug, Display, Formatter};

use hex::ToHex as _;
use integer_encoding::VarInt;
use num_traits::{AsPrimitive, PrimInt, WrappingAdd};
use zigzag::ZigZag;

use crate::{MltError, MltRefResult};

/// Parse a varint (variable-length integer) from the input
pub fn parse_varint<T: VarInt>(input: &[u8]) -> MltRefResult<'_, T> {
    match T::decode_var(input) {
        Some((value, consumed)) => {
            // Validate canonical encoding:
            // Check that the value couldn't fit in fewer bytes
            //
            // A varint is canonical if its last byte is non-zero (for multi-byte encodings).
            // Value 0 must be encoded as a single 0x00 byte.
            // For multi-byte varints, the last byte (without continuation bit) must be non-zero.
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
pub fn decode_componentwise_delta_vec2s<T: ZigZag + WrappingAdd>(
    data: &[T::UInt],
) -> Result<Vec<T>, MltError> {
    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(MltError::InvalidPairStreamSize(data.len()));
    }

    let mut result = Vec::with_capacity(data.len());
    let mut last1 = T::zero();
    let mut last2 = T::zero();

    for i in (0..data.len()).step_by(2) {
        last1 = last1.wrapping_add(&T::decode(data[i]));
        last2 = last2.wrapping_add(&T::decode(data[i + 1]));
        result.push(last1);
        result.push(last2);
    }

    Ok(result)
}

/// Decode a vector of ZigZag-encoded unsigned deltas.
pub fn decode_zigzag_delta<T: Copy + ZigZag + WrappingAdd + AsPrimitive<U>, U: 'static + Copy>(
    data: &[T::UInt],
) -> Vec<U> {
    data.iter()
        .scan(T::zero(), |state, &v| {
            *state = state.wrapping_add(&T::decode(v));
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

/// Decode a slice of bytes into a vector of u32 values assuming little-endian encoding
pub fn bytes_to_u32s(mut input: &[u8], num_values: u32) -> MltRefResult<'_, Vec<u32>> {
    let expected_bytes = num_values as usize * 4;
    if input.len() < expected_bytes {
        return Err(MltError::BufferUnderflow {
            needed: expected_bytes,
            remaining: input.len(),
        });
    }

    let mut values = Vec::with_capacity(num_values as usize);
    for _ in 0..num_values {
        let (new_input, bytes) = take(input, 4)?;
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        values.push(value);
        input = new_input;
    }
    Ok((input, values))
}

pub fn decode_zigzag<T: ZigZag>(data: &[T::UInt]) -> Vec<T> {
    data.iter().map(|&v| T::decode(v)).collect()
}

/// Decode byte-level RLE as used in ORC for boolean and present streams.
///
/// Format: control byte determines the run type:
/// - `control >= 128`: literal run of `(256 - control)` bytes follow
/// - `control < 128`: repeating run of `(control + 3)` copies of the next byte
pub fn decode_byte_rle(input: &[u8], num_bytes: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(num_bytes);
    let mut pos = 0;
    while output.len() < num_bytes && pos < input.len() {
        let control = input[pos];
        pos += 1;
        if control >= 128 {
            let count = usize::from(control ^ 0xFF) + 1;
            output.extend_from_slice(&input[pos..pos + count]);
            pos += count;
        } else {
            let count = usize::from(control) + 3;
            let value = input[pos];
            pos += 1;
            output.extend(std::iter::repeat_n(value, count));
        }
    }
    output
}

/// Wrapper type for optional slices to provide a custom Debug implementation
pub struct OptSeq<'a, T>(pub Option<&'a [T]>);

impl<T: Display + Debug> Debug for OptSeq<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_seq(f, self.0, ToString::to_string)
    }
}

pub struct OptSeqOpt<'a, T>(pub Option<&'a [Option<T>]>);

impl<T: Display + Debug> Debug for OptSeqOpt<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_seq(f, self.0, |opt| match opt {
            Some(val) => val.to_string(),
            None => "None".to_string(),
        })
    }
}

fn write_seq<T>(
    f: &mut Formatter,
    value: Option<&[T]>,
    to_str: fn(&T) -> String,
) -> std::fmt::Result {
    if let Some(v) = value {
        if f.alternate() {
            let items = v.iter().map(to_str).collect::<Vec<_>>().join(",");
            write!(f, "[{items}; {}]", v.len())
        } else {
            let items = v.iter().take(8).map(to_str).collect::<Vec<_>>().join(",");
            write!(f, "[{items}")?;
            if v.len() > 8 {
                write!(f, ", ...; {}]", v.len())
            } else {
                write!(f, "]")
            }
        }
    } else {
        write!(f, "None")
    }
}

pub(crate) fn fmt_byte_array(data: &[u8], f: &mut Formatter<'_>) -> std::fmt::Result {
    if f.alternate() {
        let vals = data.encode_hex_upper::<String>();
        write!(f, "[0x{vals}; {}]", data.len())
    } else {
        let vals = (&data[..8.min(data.len())]).encode_hex_upper::<String>();
        write!(
            f,
            "[0x{vals}{}; {}]",
            if data.len() <= 8 { "" } else { "..." },
            data.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_bytes_to_u32s_valid() {
        // Little-endian representation:
        // [0x04, 0x03, 0x02, 0x01] -> 0x01020304
        // [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
        let bytes: [u8; 8] = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let res = bytes_to_u32s(&bytes, 2);
        assert!(res.is_ok(), "Should decode valid buffer with 2 values");
        let (remaining, u32s) = res.unwrap();
        assert!(remaining.is_empty(), "All input should be consumed");
        assert_eq!(
            u32s,
            vec![0x0102_0304, 0xAABB_CCDD],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_bytes_to_u32s_empty() {
        let bytes: [u8; 0] = [];
        let res = bytes_to_u32s(&bytes, 0);
        assert!(res.is_ok(), "Empty slice with 0 values is valid");
        let (remaining, u32s) = res.unwrap();
        assert!(remaining.is_empty(), "All input should be consumed");
        assert!(
            u32s.is_empty(),
            "Output should be an empty Vec for 0 values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_buffer_underflow() {
        // Only 4 bytes but requesting 2 values (8 bytes needed)
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let res = bytes_to_u32s(&bytes, 2);
        assert!(
            res.is_err(),
            "Should error if not enough bytes for requested values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_partial_consumption() {
        // 12 bytes (3 values) but only requesting 2 values
        let bytes: [u8; 12] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let res = bytes_to_u32s(&bytes, 2);
        assert!(res.is_ok(), "Should decode 2 values from larger buffer");
        let (remaining, u32s) = res.unwrap();
        assert_eq!(remaining.len(), 4, "Should have 4 bytes remaining");
        assert_eq!(u32s.len(), 2, "Should have exactly 2 values");
        assert_eq!(
            u32s,
            vec![0x0403_0201, 0x0807_0605],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_decode_zigzag() {
        let encoded_u32 = [0u32, 1, 2, 3, 4, 5, u32::MAX];
        let expected_i32 = [0i32, -1, 1, -2, 2, -3, i32::MIN];
        let decoded_i32 = decode_zigzag::<i32>(&encoded_u32);
        assert_eq!(decoded_i32, expected_i32);

        let encoded_u64 = [0u64, 1, 2, 3, 4, 5, u64::MAX];
        let expected_i64 = [0i64, -1, 1, -2, 2, -3, i64::MIN];
        let decoded_i64 = decode_zigzag::<i64>(&encoded_u64);
        assert_eq!(decoded_i64, expected_i64);
    }

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
