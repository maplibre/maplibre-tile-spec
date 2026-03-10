use num_traits::{AsPrimitive, WrappingAdd};
use wide::u32x8;
use zigzag::ZigZag;

use crate::MltError::{BufferUnderflow, InvalidPairStreamSize};
use crate::utils::{AsUsize as _, take};
use crate::{MltError, MltRefResult};

/// Decode ([`ZigZag`] + delta) for Vec2s
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag + WrappingAdd>(
    data: &[T::UInt],
) -> Result<Vec<T>, MltError> {
    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(InvalidPairStreamSize(data.len()));
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
#[must_use]
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

/// Decode a slice of bytes into a vector of u64 values assuming little-endian encoding
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

pub fn decode_zigzag<T: ZigZag>(data: &[T::UInt]) -> Vec<T> {
    data.iter().map(|&v| T::decode(v)).collect()
}

/// Decode a single Morton code to (x, y) as i32, applying `coordinate_shift`.
fn decode_morton_one(morton_code: u32, num_bits: u32, coordinate_shift: u32) -> (i32, i32) {
    let mut x = 0u32;
    let mut y = 0u32;
    for i in 0..num_bits {
        let bit_mask = 1u32 << (2 * i);
        x |= (morton_code & bit_mask) >> i;
        y |= ((morton_code >> 1) & bit_mask) >> i;
    }
    (
        x as i32 - coordinate_shift as i32,
        y as i32 - coordinate_shift as i32,
    )
}

/// Decode delta-encoded Morton codes to flat `[x0, y0, x1, y1, ...]`.
///
/// Each input value is a signed delta (stored as u32 with wrapping arithmetic)
/// relative to the previous Morton code. The sequential prefix sum is computed
/// in chunks of 8 into a stack-allocated buffer, which is then SIMD-decoded.
/// This keeps the working set in registers / L1 cache.
#[must_use]
pub fn decode_morton_delta(data: &[u32], num_bits: u32, coordinate_shift: u32) -> Vec<i32> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let shift_vec = u32x8::splat(coordinate_shift);

    let mut prev = 0i32;
    let mut chunks = data.chunks_exact(8);

    for chunk in chunks.by_ref() {
        // Sequential prefix sum into a stack buffer — no heap allocation.
        let mut buf = [0u32; 8];
        for (b, &d) in buf.iter_mut().zip(chunk.iter()) {
            prev = prev.wrapping_add(d as i32);
            *b = prev as u32;
        }
        decode_morton_chunk(buf, num_bits, shift_vec, &mut out);
    }

    // Scalar tail for any codes that didn't fill a full SIMD chunk.
    for &d in chunks.remainder() {
        prev = prev.wrapping_add(d as i32);
        let (x, y) = decode_morton_one(prev as u32, num_bits, coordinate_shift);
        out.push(x);
        out.push(y);
    }

    out
}

/// Decode Morton codes (no delta) to flat `[x0, y0, x1, y1, ...]`.
///
/// Processes 8 codes at a time with `wide::u32x8`. Each lane extracts the
/// compacted even-bit (x) and odd-bit (y) components in parallel, then applies
/// the coordinate shift. A scalar tail handles any remaining codes.
#[must_use]
pub fn decode_morton_codes(data: &[u32], num_bits: u32, coordinate_shift: u32) -> Vec<i32> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let shift_vec = u32x8::splat(coordinate_shift);

    let mut chunks = data.chunks_exact(8);

    for chunk in chunks.by_ref() {
        let buf = [
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ];
        decode_morton_chunk(buf, num_bits, shift_vec, &mut out);
    }

    // Scalar tail for any codes that didn't fill a full SIMD chunk.
    for &code in chunks.remainder() {
        let (x, y) = decode_morton_one(code, num_bits, coordinate_shift);
        out.push(x);
        out.push(y);
    }

    out
}

/// SIMD-decode a chunk of exactly 8 resolved Morton codes into the output buffer.
///
/// Each code has already been resolved to its absolute value (no delta pending).
/// Even-indexed bits encode x, odd-indexed bits encode y.
#[inline]
fn decode_morton_chunk(buf: [u32; 8], num_bits: u32, shift_vec: u32x8, out: &mut Vec<i32>) {
    let codes = u32x8::from(buf);
    // Odd bits become even after shifting right by 1, giving the y component.
    let codes_y = codes >> 1;

    let mut x_vec = u32x8::ZERO;
    let mut y_vec = u32x8::ZERO;

    for i in 0..num_bits {
        // Mask for the bit position 2*i in the original Morton code.
        let bit_mask = u32x8::splat(1u32 << (2 * i));
        x_vec |= (codes & bit_mask) >> i;
        y_vec |= (codes_y & bit_mask) >> i;
    }

    let xs: [u32; 8] = (x_vec - shift_vec).into();
    let ys: [u32; 8] = (y_vec - shift_vec).into();

    for lane in 0..8 {
        out.push(xs[lane] as i32);
        out.push(ys[lane] as i32);
    }
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

    #[test]
    fn test_bytes_to_u32s_valid() {
        // Little-endian representation:
        // [0x04, 0x03, 0x02, 0x01] -> 0x01020304
        // [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
        let bytes: [u8; 8] = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let res = decode_bytes_to_u32s(&bytes, 2);
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
        let res = decode_bytes_to_u32s(&bytes, 0);
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
        let decoded = decode_componentwise_delta_vec2s::<i32>(values).unwrap();
        assert_eq!(&decoded, &[-1_i32, 1, -3, 3]);
    }

    #[test]
    fn test_decode_zigzag_i32() {
        let encoded_u32 = [0u32, 1, 2, 3, 4, 5, u32::MAX];
        let expected_i32 = [0i32, -1, 1, -2, 2, -3, i32::MIN];
        let decoded_i32 = decode_zigzag::<i32>(&encoded_u32);
        assert_eq!(decoded_i32, expected_i32);
    }

    #[test]
    fn test_decode_zigzag_i64() {
        let encoded_u64 = [0u64, 1, 2, 3, 4, 5, u64::MAX];
        let expected_i64 = [0i64, -1, 1, -2, 2, -3, i64::MIN];
        let decoded_i64 = decode_zigzag::<i64>(&encoded_u64);
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
        assert!(decode_zigzag::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_decode_zigzag_delta_empty() {
        assert!(decode_zigzag_delta::<i32, i32>(&[]).is_empty());
    }

    #[test]
    fn test_decode_byte_rle_empty() {
        assert!(decode_byte_rle(&[], 0).is_empty());
    }

    #[test]
    fn test_decode_bytes_to_u32s_empty() {
        let (input, decoded) = decode_bytes_to_u32s(&[], 0).unwrap();
        assert!(decoded.is_empty());
        assert!(input.is_empty());
    }

    #[test]
    fn test_decode_fastpfor_empty() {
        let decoded = decode_fastpfor_composite(&[], 0).unwrap();
        assert!(decoded.is_empty());
    }

    // --- Morton helpers used across multiple tests ---

    /// Interleave two `NUM_BITS`-wide values into a Morton code.
    fn encode_morton(x: u32, y: u32) -> u32 {
        let mut code = 0u32;
        for bit in 0..NUM_BITS {
            code |= ((x >> bit) & 1) << (2 * bit);
            code |= ((y >> bit) & 1) << (2 * bit + 1);
        }
        code
    }

    const NUM_BITS: u32 = 15;
    const COORD_SHIFT: u32 = 1 << (NUM_BITS - 1); // 16384

    // --- decode_morton_codes tests ---

    #[test]
    fn test_decode_morton_codes_empty() {
        assert!(decode_morton_codes(&[], NUM_BITS, COORD_SHIFT).is_empty());
    }

    #[test]
    fn test_decode_morton_codes_origin() {
        // Morton code for (COORD_SHIFT, COORD_SHIFT) should decode to (0, 0).
        let code = encode_morton(COORD_SHIFT, COORD_SHIFT);
        assert_eq!(decode_morton_codes(&[code], NUM_BITS, COORD_SHIFT), [0, 0]);
    }

    #[test]
    fn test_decode_morton_codes_known_values() {
        // x=1, y=2 (pre-shift) → decoded (1 - COORD_SHIFT, 2 - COORD_SHIFT)
        let x: u32 = 1;
        let y: u32 = 2;
        let code = encode_morton(x, y);
        let expected_x = x.cast_signed() - COORD_SHIFT.cast_signed();
        let expected_y = y.cast_signed() - COORD_SHIFT.cast_signed();
        assert_eq!(
            decode_morton_codes(&[code], NUM_BITS, COORD_SHIFT),
            [expected_x, expected_y]
        );
    }

    #[test]
    fn test_decode_morton_codes_scalar_tail() {
        // 3 codes — exercises the scalar tail path (< 8 codes).
        let pairs = [(0u32, 1u32), (2, 3), (4, 5)];
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton(x, y)).collect();
        let result = decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT);
        let expected = test_morton(&pairs);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_morton_codes_full_simd_chunk() {
        // 8 codes — exercises exactly one SIMD chunk, no scalar tail.
        let pairs: [(u32, u32); 8] = [
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (2, 3),
            (7, 5),
            (10, 9),
            (15, 15),
        ];
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton(x, y)).collect();
        let result = decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT);
        let expected = test_morton(&pairs);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_morton_codes_simd_plus_tail() {
        // 11 codes — one full SIMD chunk of 8 plus a scalar tail of 3.
        let pairs: Vec<(u32, u32)> = (0..11u32).map(|i| (i * 3 % 100, i * 7 % 100)).collect();
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton(x, y)).collect();
        let result = decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT);
        let expected = test_morton(&pairs);
        assert_eq!(result, expected);
    }

    // --- decode_morton_delta tests ---

    #[test]
    fn test_decode_morton_delta_empty() {
        assert!(decode_morton_delta(&[], NUM_BITS, COORD_SHIFT).is_empty());
    }

    #[test]
    fn test_decode_morton_delta_identity_with_zero_deltas() {
        // All-zero deltas: every resolved code is 0, which decodes to (-COORD_SHIFT, -COORD_SHIFT).
        let deltas = vec![0u32; 3];
        let result = decode_morton_delta(&deltas, NUM_BITS, COORD_SHIFT);
        let shift = -COORD_SHIFT.cast_signed();
        assert_eq!(result, vec![shift, shift, shift, shift, shift, shift]);
    }

    #[test]
    fn test_decode_morton_delta_matches_codes_after_prefix_sum() {
        // Build a sequence of absolute codes, compute their deltas, then verify that
        // decode_morton_delta produces the same output as decode_morton_codes on the
        // original absolute codes.
        let pairs: Vec<(u32, u32)> = (0..11u32).map(|i| (i * 5 % 200, i * 9 % 200)).collect();
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton(x, y)).collect();
        let deltas: Vec<u32> = test_delta_morton(&codes);

        let from_codes = decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT);
        let from_deltas = decode_morton_delta(&deltas, NUM_BITS, COORD_SHIFT);
        assert_eq!(from_codes, from_deltas);
    }

    #[test]
    fn test_decode_morton_delta_scalar_tail() {
        // 3 codes via deltas — scalar tail path only.
        let codes: Vec<u32> = vec![
            encode_morton(10, 20),
            encode_morton(30, 40),
            encode_morton(50, 60),
        ];
        let deltas: Vec<u32> = test_delta_morton(&codes);
        let from_codes = decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT);
        let from_deltas = decode_morton_delta(&deltas, NUM_BITS, COORD_SHIFT);
        assert_eq!(from_codes, from_deltas);
    }

    #[test]
    fn test_decode_morton_delta_wrapping() {
        // A single wrapping delta: start from a large code, subtract more than it — should
        // still round-trip correctly via wrapping arithmetic.
        let code_a = encode_morton(500, 300);
        let code_b = encode_morton(10, 10); // numerically smaller than code_a
        let delta_b = code_b.cast_signed().wrapping_sub(code_a.cast_signed()) as u32;
        let deltas = vec![code_a, delta_b];
        let codes = vec![code_a, code_b];
        assert_eq!(
            decode_morton_delta(&deltas, NUM_BITS, COORD_SHIFT),
            decode_morton_codes(&codes, NUM_BITS, COORD_SHIFT)
        );
    }

    fn test_morton(pairs: &[(u32, u32)]) -> Vec<i32> {
        pairs
            .iter()
            .flat_map(|&(x, y)| {
                [
                    x.cast_signed() - COORD_SHIFT.cast_signed(),
                    y.cast_signed() - COORD_SHIFT.cast_signed(),
                ]
            })
            .collect()
    }

    /// Compute signed deltas (wrapping).
    fn test_delta_morton(codes: &[u32]) -> Vec<u32> {
        let mut prev = 0i32;
        codes
            .iter()
            .map(|&c| {
                let delta = c.cast_signed().wrapping_sub(prev) as u32;
                prev = c.cast_signed();
                delta
            })
            .collect()
    }
}
