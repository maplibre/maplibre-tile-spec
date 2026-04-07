use num_traits::PrimInt;

use crate::utils::AsUsize as _;
use crate::{Decoder, MltError, MltResult};

/// Generic run-length encode: returns `(run_lengths, values)`.
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
pub fn encode_byte_rle(data: &[u8], target: &mut Vec<u8>) {
    target.clear();
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
            target.push(control);
            target.push(data[pos]);
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
            target.push(control);
            target.extend_from_slice(&data[pos..pos + literal_count]);
            pos += literal_count;
        }
    }
}

/// Decode byte-level RLE as used in ORC for boolean and present streams.
///
/// Format: control byte determines the run type:
/// - `control >= 128`: literal run of `(256 - control)` bytes follow
/// - `control < 128`: repeating run of `(control + 3)` copies of the next byte
pub fn decode_byte_rle(input: &[u8], num_bytes: usize, dec: &mut Decoder) -> MltResult<Vec<u8>> {
    let mut output = dec.alloc(num_bytes)?;
    let mut pos = 0;
    while output.len() < num_bytes && pos < input.len() {
        let control = input[pos];
        pos += 1;
        if control >= 128 {
            let count = u32::from(control ^ 0xFF) + 1;
            let end = pos + count.as_usize();
            let slice = input.get(pos..end).ok_or(MltError::BufferUnderflow(
                count,
                input.len().saturating_sub(pos),
            ))?;
            output.extend_from_slice(slice);
            pos = end;
        } else {
            let count = usize::from(control) + 3;
            let &value = input.get(pos).ok_or(MltError::BufferUnderflow(1, 0))?;
            pos += 1;
            output.extend(std::iter::repeat_n(value, count));
        }
    }
    dec.adjust_alloc(&output, num_bytes);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::decoder::RleMeta;
    use crate::test_helpers::dec;

    proptest! {
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
            let mut encoded = Vec::new();
            encode_byte_rle(&data, &mut encoded);
            let decoded = decode_byte_rle(&encoded, data.len(), &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }

    #[test]
    fn test_encode_rle_empty() {
        let (runs, vals) = encode_rle::<u8>(&[]);
        assert!(runs.is_empty());
        assert!(vals.is_empty());
    }

    #[test]
    fn test_encode_byte_rle_empty() {
        let mut buf = Vec::new();
        encode_byte_rle(&[], &mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_decode_byte_rle_empty() {
        assert!(decode_byte_rle(&[], 0, &mut dec()).unwrap().is_empty());
    }
}
