use std::mem::size_of;

use num_traits::{AsPrimitive, WrappingAdd};
use zigzag::ZigZag;

use crate::MltError::{BufferUnderflow, InvalidPairStreamSize};
use crate::decoder::debug_assert_alloc;
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, take};
use crate::{Decoder, MltError, MltRefResult};

/// Decode ([`ZigZag`] + delta) for Vec2s, charging `dec` for the output allocation.
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag + WrappingAdd>(
    data: &[T::UInt],
    dec: &mut Decoder,
) -> Result<Vec<T>, MltError> {
    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(InvalidPairStreamSize(data.len()));
    }

    let alloc_size = data.len();
    dec.consume(u32::try_from(alloc_size * size_of::<T>()).or_overflow()?)?;
    let mut result = dec.alloc(alloc_size)?;
    let mut last1 = T::zero();
    let mut last2 = T::zero();

    for i in (0..data.len()).step_by(2) {
        last1 = last1.wrapping_add(&T::decode(data[i]));
        last2 = last2.wrapping_add(&T::decode(data[i + 1]));
        result.push(last1);
        result.push(last2);
    }

    debug_assert_alloc(&result, alloc_size);
    Ok(result)
}

/// Decode a vector of ZigZag-encoded unsigned deltas, charging `dec` for the output allocation.
pub fn decode_zigzag_delta<T: Copy + ZigZag + WrappingAdd + AsPrimitive<U>, U: 'static + Copy>(
    data: &[T::UInt],
    dec: &mut Decoder,
) -> Result<Vec<U>, MltError> {
    dec.consume(u32::try_from(data.len() * size_of::<U>()).or_overflow()?)?;
    Ok(data
        .iter()
        .scan(T::zero(), |state, &v| {
            *state = state.wrapping_add(&T::decode(v));
            Some((*state).as_())
        })
        .collect())
}

/// Decode a slice of bytes into a vector of u64 values assuming little-endian encoding
/// TODO: Should this return `MltRefResult`, or should it assert the entire input is consumed?
pub fn decode_bytes_to_u64s(mut input: &[u8], num_values: u32) -> MltRefResult<'_, Vec<u64>> {
    let Some(expected_bytes) = num_values.checked_mul(8) else {
        return Err(BufferUnderflow(u32::MAX, input.len()));
    };
    if input.len() < expected_bytes.as_usize() {
        return Err(BufferUnderflow(expected_bytes, input.len()));
    }

    let mut values = Vec::with_capacity(num_values.as_usize());
    for _ in 0..num_values {
        let (new_input, bytes) = take(input, 8)?;
        let value = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        values.push(value);
        input = new_input;
    }
    Ok((input, values))
}

/// Decode a slice of bytes into a vector of u32 values assuming little-endian encoding
pub fn decode_bytes_to_u32s(mut input: &[u8], num_values: u32) -> MltRefResult<'_, Vec<u32>> {
    let Some(expected_bytes) = num_values.checked_mul(4) else {
        return Err(BufferUnderflow(u32::MAX, input.len()));
    };
    if input.len() < expected_bytes.as_usize() {
        return Err(BufferUnderflow(expected_bytes, input.len()));
    }

    let mut values = Vec::with_capacity(num_values.as_usize());
    for _ in 0..num_values {
        let (new_input, bytes) = take(input, 4)?;
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        values.push(value);
        input = new_input;
    }
    Ok((input, values))
}

/// ZigZag-decode a slice, charging `dec` for the output allocation.
pub fn decode_zigzag<T: ZigZag>(data: &[T::UInt], dec: &mut Decoder) -> Result<Vec<T>, MltError> {
    dec.consume(u32::try_from(data.len() * size_of::<T>()).or_overflow()?)?;
    Ok(data.iter().map(|&v| T::decode(v)).collect())
}

/// Decode byte-level RLE as used in ORC for boolean and present streams.
///
/// Format: control byte determines the run type:
/// - `control >= 128`: literal run of `(256 - control)` bytes follow
/// - `control < 128`: repeating run of `(control + 3)` copies of the next byte
#[must_use]
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

/// Helper to unpack a `Vec<u8>` into `Vec<bool>` where each byte represents 8 booleans.
#[must_use]
pub fn decode_bytes_to_bools(bytes: &[u8], num_bools: usize) -> Vec<bool> {
    debug_assert!(num_bools <= bytes.len() * 8);
    (0..num_bools)
        .map(|i| (bytes[i / 8] >> (i % 8)) & 1 == 1)
        .collect::<Vec<_>>()
}

/// Decode `FastPFOR`-compressed data using the composite codec protocol.
///
/// The Java MLT encoder uses `Composition(FastPFOR(), VariableByte())`, matching
/// the C++ `CompositeCodec<FastPFor<8>, VariableByte>`. The wire format is:
///
/// 1. First u32 = number of compressed u32 words from the primary codec (`FastPFor`)
/// 2. Next N u32 words = primary codec (`FastPFor`) compressed data
/// 3. Remaining u32 words = secondary codec (`VByte`) compressed data
///
/// The compressed bytes are stored as big-endian u32 values by the Java encoder.
pub fn decode_fastpfor_composite(data: &[u8], num_values: usize) -> Result<Vec<u32>, MltError> {
    if num_values == 0 {
        return Ok(vec![]);
    }

    // Convert big-endian bytes to u32 values
    if !data.len().is_multiple_of(4) {
        return Err(MltError::InvalidFastPforByteLength(data.len()));
    }
    // The Java MLT encoder writes compressed int[] → byte[] in big-endian order.
    // We must convert BE bytes → u32 to reconstruct the original integer values
    // that the Composition(FastPFOR, VariableByte) codec produced.
    let num_words = data.len() / 4;
    let input: Vec<u32> = (0..num_words)
        .map(|i| {
            let o = i * 4;
            u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        })
        .collect();

    if input.is_empty() {
        return Err(MltError::FastPforDecode(num_values, 0));
    }

    #[cfg(feature = "fastpfor-cpp")]
    {
        use fastpfor::cpp::{Codec32 as _, FastPFor256Codec};
        // The fastpfor crate's FastPFor256Codec is already a CompositeCodec<FastPFor<8>, VariableByte>.
        // It handles the full Composition protocol internally (FastPFor header + VByte remainder).

        // Over-allocate output buffer — the codec may decode padding beyond num_values.
        let buf_size = num_values + 1024;
        let mut result = vec![0u32; buf_size];

        let decoded = FastPFor256Codec::new().decode32(&input, &mut result)?;

        if decoded.len() < num_values {
            return Err(MltError::FastPforDecode(num_values, decoded.len()));
        }

        result.truncate(num_values);
        Ok(result)
    }
    #[cfg(all(feature = "fastpfor-rust", not(feature = "fastpfor-cpp")))]
    {
        use fastpfor::rust::{Composition, FastPFOR, Integer as _, VariableByte};

        // Over-allocate output buffer - the codec may decode padding beyond num_values.
        let buf_size = num_values + 1024;
        let mut result = vec![0u32; buf_size];

        let mut comp = Composition::new(FastPFOR::default(), VariableByte::new());
        let mut output_offset = std::io::Cursor::new(0u32);

        comp.uncompress(
            &input,
            u32::try_from(input.len())?,
            &mut std::io::Cursor::new(0u32),
            &mut result,
            &mut output_offset,
        )?;

        // FIXME: handle usize casting to be within u32?
        let decoded = usize::try_from(output_offset.position())?;
        if decoded < num_values {
            return Err(MltError::FastPforDecode(num_values, decoded));
        }

        result.truncate(num_values);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_empty, dec};

    #[test]
    fn test_bytes_to_u32s_valid() {
        // Little-endian representation:
        // [0x04, 0x03, 0x02, 0x01] -> 0x01020304
        // [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
        let bytes: [u8; 8] = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let res = decode_bytes_to_u32s(&bytes, 2);
        assert!(res.is_ok(), "Should decode valid buffer with 2 values");
        let (remaining, u32s) = res.unwrap();
        assert_empty(remaining);
        assert_eq!(
            u32s,
            vec![0x0102_0304, 0xAABB_CCDD],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_bytes_to_u32s_empty() {
        let bytes: [u8; 0] = [];
        let res = decode_bytes_to_u32s(&bytes, 0);
        assert!(res.is_ok(), "Empty slice with 0 values is valid");
        let (remaining, u32s) = res.unwrap();
        assert_empty(remaining);
        assert!(
            u32s.is_empty(),
            "Output should be an empty Vec for 0 values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_buffer_underflow() {
        // Only 4 bytes but requesting 2 values (8 bytes needed)
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let res = decode_bytes_to_u32s(&bytes, 2);
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
        let res = decode_bytes_to_u32s(&bytes, 2);
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
    fn test_decode_componentwise_delta_vec2s() {
        let values = &[1_u32, 2, 3, 4];
        let decoded = decode_componentwise_delta_vec2s::<i32>(values, &mut dec()).unwrap();
        assert_eq!(&decoded, &[-1_i32, 1, -3, 3]);
    }

    #[test]
    fn test_decode_zigzag_i32() {
        let encoded_u32 = [0u32, 1, 2, 3, 4, 5, u32::MAX];
        let expected_i32 = [0i32, -1, 1, -2, 2, -3, i32::MIN];
        let decoded_i32 = decode_zigzag::<i32>(&encoded_u32, &mut dec()).unwrap();
        assert_eq!(decoded_i32, expected_i32);
    }

    #[test]
    fn test_decode_zigzag_i64() {
        let encoded_u64 = [0u64, 1, 2, 3, 4, 5, u64::MAX];
        let expected_i64 = [0i64, -1, 1, -2, 2, -3, i64::MIN];
        let decoded_i64 = decode_zigzag::<i64>(&encoded_u64, &mut dec()).unwrap();
        assert_eq!(decoded_i64, expected_i64);
    }

    #[test]
    fn test_decode_u64() {
        let bytes = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        assert_eq!(decode_bytes_to_u64s(&bytes, 2).unwrap(), expected);
    }

    #[test]
    fn test_decode_u32() {
        let bytes = [1, 0, 0, 0, 2, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        assert_eq!(decode_bytes_to_u32s(&bytes, 2).unwrap(), expected);
    }

    #[test]
    fn test_decode_zigzag_empty() {
        assert!(decode_zigzag::<i32>(&[], &mut dec()).unwrap().is_empty());
    }

    #[test]
    fn test_decode_zigzag_delta_empty() {
        assert!(
            decode_zigzag_delta::<i32, i32>(&[], &mut dec())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_decode_byte_rle_empty() {
        assert!(decode_byte_rle(&[], 0).is_empty());
    }

    #[test]
    fn test_decode_bytes_to_u32s_empty() {
        let (input, decoded) = decode_bytes_to_u32s(&[], 0).unwrap();
        assert_empty(input);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_decode_fastpfor_empty() {
        let decoded = decode_fastpfor_composite(&[], 0).unwrap();
        assert!(decoded.is_empty());
    }
}
