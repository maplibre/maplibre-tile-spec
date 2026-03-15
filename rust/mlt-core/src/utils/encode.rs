use num_traits::{PrimInt, WrappingSub};
use zigzag::ZigZag;

use crate::MltError;

#[must_use]
pub fn encode_zigzag<T: ZigZag>(data: &[T]) -> Vec<T::UInt> {
    data.iter().map(|&v| T::encode(v)).collect()
}

fn encode_delta<T: Copy + WrappingSub>(data: &[T]) -> Vec<T> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(data.len());
    result.push(data[0]);
    for i in 1..data.len() {
        result.push(data[i].wrapping_sub(&data[i - 1]));
    }
    result
}

#[must_use]
pub fn encode_zigzag_delta<T: Copy + ZigZag + WrappingSub<Output = T>>(data: &[T]) -> Vec<T::UInt> {
    encode_zigzag(&encode_delta(data))
}

#[must_use]
pub fn encode_rle<T: PrimInt>(data: &[T]) -> (Vec<T>, Vec<T>) {
    if data.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut runs = Vec::new();
    let mut values = Vec::new();

    let mut current_val = data[0];
    let mut current_run = T::one();

    for &val in &data[1..] {
        if val == current_val {
            current_run = current_run.saturating_add(T::one());
        } else {
            runs.push(current_run);
            values.push(current_val);
            current_val = val;
            current_run = T::one();
        }
    }
    runs.push(current_run);
    values.push(current_val);

    (runs, values)
}

/// Encode byte-level RLE as used in ORC for boolean and present streams.
///
/// Format: control byte determines the run type:
/// - `control >= 128`: literal run of `(256 - control)` bytes follow
/// - `control < 128`: repeating run of `(control + 3)` copies of the next byte
#[must_use]
pub fn encode_byte_rle(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Look ahead for repeating run
        let mut repeat_count = 1;
        while pos + repeat_count < data.len()
            && data[pos + repeat_count] == data[pos]
            && repeat_count < 130
        {
            repeat_count += 1;
        }

        if repeat_count >= 3 {
            // Encode repeating run
            #[expect(clippy::cast_possible_truncation, reason = "3 <= repeat_count < 130")]
            let control = (repeat_count - 3) as u8;
            output.push(control);
            output.push(data[pos]);
            pos += repeat_count;
        } else {
            // Encode literal run
            let mut literal_count = 0;
            // Scan ahead to see where the next repeating run starts
            while pos + literal_count < data.len() && literal_count < 128 {
                let mut inner_repeat = 1;
                while let next_idx = pos + literal_count
                    && next_idx + inner_repeat < data.len()
                    && data[next_idx + inner_repeat] == data[next_idx]
                    && inner_repeat < 3
                {
                    inner_repeat += 1;
                }

                if inner_repeat >= 3 {
                    break;
                }
                literal_count += 1;
            }

            #[expect(
                clippy::cast_possible_truncation,
                reason = "literal_count is always smaller than 128"
            )]
            let control = (256 - literal_count) as u8;
            output.push(control);
            output.extend_from_slice(&data[pos..pos + literal_count]);
            pos += literal_count;
        }
    }
    output
}

/// Encode a `u32` sequence using `FastPFOR256` (composite codec).
///
/// This is the inverse of `decode_fastpfor_composite`
pub fn encode_fastpfor(values: &[u32]) -> Result<Vec<u8>, MltError> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(feature = "fastpfor-cpp")]
    {
        use fastpfor::cpp::{Codec32 as _, FastPFor256Codec};
        let codec = FastPFor256Codec::new();
        // Over-allocate: FastPFOR may write a header and padding beyond the input length.
        let mut compressed = vec![0u32; values.len() + 1024];
        let out = codec.encode32(values, &mut compressed)?;

        // Convert u32 words to big-endian bytes to match the wire format.
        let mut data = Vec::with_capacity(out.len() * 4);
        for word in out.iter() {
            data.extend_from_slice(&word.to_be_bytes());
        }
        Ok(data)
    }
    #[cfg(all(feature = "fastpfor-rust", not(feature = "fastpfor-cpp")))]
    {
        use fastpfor::rust::{Composition, FastPFOR, Integer as _, VariableByte};

        // Over-allocate: FastPFOR may write a header and padding beyond the input length.
        let mut compressed = vec![0u32; values.len() + 1024];
        let mut comp = Composition::new(FastPFOR::default(), VariableByte::new());
        let mut output_offset = std::io::Cursor::new(0u32);

        comp.compress(
            values,
            u32::try_from(values.len())?,
            &mut std::io::Cursor::new(0u32),
            &mut compressed,
            &mut output_offset,
        )?;

        // FIXME: handle usize casting to be within u32?
        let written = usize::try_from(output_offset.position())?;

        // Convert u32 words to big-endian bytes to match the wire format.
        let mut data = Vec::with_capacity(written * 4);
        for word in &compressed[..written] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        Ok(data)
    }
}

#[must_use]
pub fn encode_u32s_to_bytes(data: &[u32]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() * 4);
    for &val in data {
        output.extend_from_slice(&val.to_le_bytes());
    }
    output
}

#[must_use]
pub fn encode_u64s_to_bytes(data: &[u64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() * 8);
    for &val in data {
        output.extend_from_slice(&val.to_le_bytes());
    }
    output
}

/// Encode signed integer vec2 values using componentwise delta + zigzag encoding.
///
/// Input: `[x0, y0, x1, y1, ...]`
/// Output: `[zigzag(x0-0), zigzag(y0-0), zigzag(x1-x0), zigzag(y1-y0), ...]`
///
/// This is the inverse of `decode_componentwise_delta_vec2s`.
#[must_use]
pub fn encode_componentwise_delta_vec2s<T>(data: &[T]) -> Vec<T::UInt>
where
    T: ZigZag + WrappingSub,
{
    let mut result = Vec::with_capacity(data.len());
    let mut prev_x = T::zero();
    let mut prev_y = T::zero();
    for chunk in data.chunks_exact(2) {
        let (x, y) = (chunk[0], chunk[1]);
        result.push(T::encode(x.wrapping_sub(&prev_x)));
        result.push(T::encode(y.wrapping_sub(&prev_y)));
        (prev_x, prev_y) = (x, y);
    }
    result
}

/// Interleave `x` and `y` into a single Morton code using `num_bits` bits per component.
///
/// Even bit positions encode `x`, odd positions encode `y`.
/// This is the inverse of the Morton decode in `decode_morton_codes` / `decode_morton_delta`.
#[must_use]
#[inline]
pub fn encode_morton_15(x: u32, y: u32) -> u32 {
    let mut code = 0u32;
    for bit in 0..15 {
        code |= ((x >> bit) & 1) << (2 * bit);
        code |= ((y >> bit) & 1) << (2 * bit + 1);
    }
    code
}

/// Helper to pack a `Vec<bool>` into `Vec<u8>` where each byte represents 8 booleans.
#[must_use]
pub fn encode_bools_to_bytes(bools: &[bool]) -> Vec<u8> {
    let num_bytes = bools.len().div_ceil(8);
    let mut bytes = vec![0u8; num_bytes];
    for (i, _) in bools.iter().enumerate().filter(|(_, bit)| **bit) {
        bytes[i / 8] |= 1 << (i % 8);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::test_helpers::dec;
    use crate::utils::{
        decode_byte_rle, decode_bytes_to_bools, decode_bytes_to_u32s, decode_bytes_to_u64s,
        decode_componentwise_delta_vec2s, decode_fastpfor_composite, decode_zigzag,
        decode_zigzag_delta,
    };
    use crate::v01::RleMeta;

    proptest! {
        #[test]
        fn encode_bools_to_bytes_roundtrip(bools: Vec<bool>) {
            let bools_rountrip = decode_bytes_to_bools(&encode_bools_to_bytes(&bools), bools.len());
            prop_assert_eq!(bools_rountrip, bools);
        }

        #[test]
        fn test_zigzag_roundtrip_i64(data: Vec<i64>) {
            let encoded = encode_zigzag(&data);
            let decoded = decode_zigzag::<i64>(&encoded);
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_delta_roundtrip_i32(data: Vec<i32>) {
            if data.is_empty() { return Ok(()); }
            let encoded = encode_zigzag_delta(&data);
            let decoded: Vec<i32> = decode_zigzag_delta::<i32, i32>(&encoded);
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_rle_roundtrip_u32(data: Vec<u32>) {
            let (runs, vals) = encode_rle(&data);
            let mut combined = runs.clone();
            combined.extend(vals);
            let runs = u32::try_from(runs.len()).unwrap();
            let num_rle_values = u32::try_from(data.len()).unwrap();
            let rle = RleMeta { runs, num_rle_values };
            let decoded = rle.decode(&combined, &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_byte_rle_roundtrip(data: Vec<u8>) {
            let encoded = encode_byte_rle(&data);
            let decoded = decode_byte_rle(&encoded, data.len());
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_u32_bytes_roundtrip(data: Vec<u32>) {
            let encoded = encode_u32s_to_bytes(&data);
            let (rem, decoded) = decode_bytes_to_u32s(&encoded, u32::try_from(data.len()).unwrap()).unwrap();
            prop_assert_eq!(data, decoded);
            prop_assert!(rem.is_empty());
        }

        #[test]
        fn test_componentwise_delta_vec2s(data: Vec<i32>) {
            if data.len() <= 1 {
                return Err(TestCaseError::reject("data not valid vertices"))
            }
            // done this way to not have to reject less
            let data_slice = if data.len().is_multiple_of(2) {
                &data
            } else {
                &data[.. data.len()-1]
            };
            let encoded = encode_componentwise_delta_vec2s(data_slice);
            let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded).unwrap();
            prop_assert_eq!(data_slice, &decoded);
        }

        #[test]
        fn test_u64_bytes_roundtrip(data: Vec<u64>) {
            let encoded = encode_u64s_to_bytes(&data);
            let (rem, decoded) = decode_bytes_to_u64s(&encoded, u32::try_from(data.len()).unwrap()).unwrap();
            prop_assert_eq!(data, decoded);
            prop_assert!(rem.is_empty());
        }

        #[test]
        fn test_fastpfor_roundtrip(data: Vec<u32>) {
            let encoded = encode_fastpfor(&data).unwrap();
            let decoded = decode_fastpfor_composite(&encoded, data.len()).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }

    #[test]
    fn test_encode_fastpfor_empty() {
        let encoded = encode_fastpfor(&[]).unwrap();
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_encode_zigzag_empty() {
        assert!(encode_zigzag::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_encode_delta_empty() {
        assert!(encode_delta::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_encode_zigzag_delta_empty() {
        assert!(encode_zigzag_delta::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_encode_rle_empty() {
        let (runs, vals) = encode_rle::<u8>(&[]);
        assert!(runs.is_empty());
        assert!(vals.is_empty());
    }

    #[test]
    fn test_encode_byte_rle_empty() {
        assert!(encode_byte_rle(&[]).is_empty());
    }
}
