use crate::decoder::tracked_bytes::TrackedBytes;
use crate::decoder::varint;
use crate::decoder::vectorized::helpers::decode_componentwise_delta_vec2s;
use crate::encoder::integer::encoded_u32s_to_bytes;
use crate::metadata::stream::{Morton, Rle, StreamMetadata};
use crate::metadata::stream_encoding::{
    Logical, LogicalLevelTechnique, LogicalStreamType, Physical, PhysicalLevelTechnique,
    PhysicalStreamType,
};
use crate::MltError;
use fastpfor::cpp::Codec32 as _;
use fastpfor::cpp::FastPFor128Codec;

use bytes::Buf;
use morton_encoding::{morton_decode, morton_encode};
use num_traits::PrimInt;
use std::fmt::Debug;
use zigzag::ZigZag;

/// decode_long_stream is a placeholder for future implementation
/// For some reasons, the Java code has a method that decodes long streams,
/// but it has different logic than the integer stream decoding.
/// It is not clear what the purpose of this method is, so it is left unimplemented
pub fn decode_long_stream(
    _tile: &mut TrackedBytes,
    _metadata: &StreamMetadata,
    _is_signed: bool,
) -> Result<Vec<u64>, MltError> {
    todo!()
}

/// decode_int_stream can handle multiple decoding techniques,
/// some of which do represent signed integers (like varint with ZigZag)
/// so returning Vec<i32>
pub fn decode_int_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
    is_signed: bool,
) -> Result<Vec<i32>, MltError> {
    let values = decode_physical(tile, metadata)?;
    decode_logical(values, metadata, is_signed)
}

/// Byte-level decoding based on the physical technique and stream type
fn decode_physical(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> Result<Vec<u32>, MltError> {
    match metadata.physical.technique {
        PhysicalLevelTechnique::FastPfor => decode_fast_pfor(tile, metadata),
        PhysicalLevelTechnique::Varint => varint::decode::<u32>(tile, metadata.num_values as usize),
        _ => Err(MltError::UnsupportedIntStreamTechnique(format!(
            "{:?}",
            metadata.physical.technique
        ))),
    }
}

/// Logical-level decoding based on the logical technique
fn decode_logical(
    values: Vec<u32>,
    metadata: &StreamMetadata,
    is_signed: bool,
) -> Result<Vec<i32>, MltError> {
    match metadata.logical.technique1 {
        Some(LogicalLevelTechnique::Delta) => {
            if metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle) {
                let rle_metadata = metadata.rle.as_ref().ok_or_else(|| {
                    MltError::DecodeError("RLE metadata is required for Delta + RLE".to_string())
                })?;
                let values = decode_rle(&values, rle_metadata)?;
                return Ok(decode_zigzag_delta(&values));
            }
            Ok(decode_zigzag_delta(&values))
        }
        Some(LogicalLevelTechnique::Rle) => {
            let rle_metadata = metadata.rle.as_ref().ok_or_else(|| {
                MltError::DecodeError("RLE metadata is required for Delta + RLE".to_string())
            })?;
            let values = decode_rle(&values, rle_metadata)?;
            if is_signed {
                Ok(decode_zigzag(&values))
            } else {
                Ok(convert_u32_to_i32(&values)?)
            }
        }
        Some(LogicalLevelTechnique::None) => {
            if is_signed {
                return Ok(decode_zigzag(&values));
            }
            Ok(convert_u32_to_i32(&values)?)
        }
        Some(LogicalLevelTechnique::Morton) => {
            let morton_metadata = metadata.morton.as_ref().ok_or_else(|| {
                MltError::DecodeError("Morton metadata is required for Morton encoding".to_string())
            })?;
            decode_morton_u64_to_i32_vec2s_flat(&values, morton_metadata.coordinate_shift)
        }
        Some(LogicalLevelTechnique::ComponentwiseDelta) => {
            decode_componentwise_delta_vec2s(&values)
        }
        _ => Err(MltError::UnsupportedIntStreamTechnique(format!(
            "{:?}",
            metadata.logical.technique1
        ))),
    }
}

fn decode_fast_pfor(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> Result<Vec<u32>, MltError> {
    let codec = FastPFor128Codec::new();
    let mut decoded = vec![0; metadata.num_values as usize];
    // Regardless of u32 or u64 fastpfor, the encoded data is always u32
    let encoded_u32s: Vec<u32> = bytes_to_encoded_u32s(tile, metadata.byte_length as usize)?;
    let _ = codec.decode32(&encoded_u32s, &mut decoded);
    Ok(decoded)
}

fn bytes_to_encoded_u32s(tile: &mut TrackedBytes, num_bytes: usize) -> Result<Vec<u32>, MltError> {
    let num_bytes = num_bytes / 4;
    let encoded_u32s = (0..num_bytes)
        .map(|_| {
            let b1 = tile.get_u8();
            let b2 = tile.get_u8();
            let b3 = tile.get_u8();
            let b4 = tile.get_u8();
            u32::from_be_bytes([b1, b2, b3, b4])
        })
        .collect();
    Ok(encoded_u32s)
}

/// Decode RLE (Run-Length Encoding) data
/// It serves the same purpose as the `decodeUnsignedRLE` and `decodeRLE` methods in the Java code.
fn decode_rle<T: PrimInt + Debug>(data: &[T], rle_meta: &Rle) -> Result<Vec<T>, MltError> {
    let runs = rle_meta.runs as usize;
    let total = rle_meta.num_rle_values as usize;
    let (run_lens, values) = data.split_at(runs);
    let mut result = Vec::with_capacity(total);
    for (&run, &val) in run_lens.iter().zip(values.iter()) {
        let run_len = run.to_usize().ok_or_else(|| {
            MltError::DecodeError(format!("Failed to convert run length to usize: {run:?}"))
        })?;
        result.extend(std::iter::repeat_n(val, run_len));
    }
    Ok(result)
}

/// Decode a vector of ZigZag-encoded unsigned integers.
fn decode_zigzag<T: ZigZag>(data: &[T::UInt]) -> Vec<T> {
    data.iter().map(|&v| T::decode(v)).collect()
}

/// Decode a vector of ZigZag-encoded unsigned deltas.
fn decode_zigzag_delta<T: ZigZag>(data: &[T::UInt]) -> Vec<T> {
    data.iter()
        .scan(T::zero(), |state, &v| {
            let decoded_delta = T::decode(v);
            *state = *state + decoded_delta;
            Some(*state)
        })
        .collect()
}

fn decode_morton_u64_to_i32_vec2<T: Into<u64>>(
    morton_code: T,
    coordinate_shift: u32,
) -> Result<[i32; 2], MltError> {
    let encoded: u64 = morton_code.into();
    let [xu, yu]: [u32; 2] = morton_decode(encoded);

    // Convert u32 to i32, checking for overflow
    let shift = i32::try_from(coordinate_shift).map_err(|_| {
        MltError::DecodeError(format!("Shift value {coordinate_shift} too large for i32"))
    })?;
    let x_i32 = i32::try_from(xu)
        .map_err(|_| MltError::DecodeError(format!("coordinate {xu} too large for i32")))?;
    let y_i32 = i32::try_from(yu)
        .map_err(|_| MltError::DecodeError(format!("coordinate {yu} too large for i32")))?;

    let x = x_i32
        .checked_sub(shift)
        .ok_or_else(|| MltError::DecodeError(format!("subtract overflow: {x_i32} - {shift}")))?;
    let y = y_i32
        .checked_sub(shift)
        .ok_or_else(|| MltError::DecodeError(format!("subtract overflow: {y_i32} - {shift}")))?;

    Ok([x, y])
}

fn decode_morton_u64_to_i32_vec2s_flat<T: Into<u64> + Copy>(
    data: &[T],
    coordinate_shift: u32,
) -> Result<Vec<i32>, MltError> {
    let mut vectices: Vec<i32> = Vec::with_capacity(data.len() * 2);
    for &code in data {
        let [x, y] = decode_morton_u64_to_i32_vec2(code, coordinate_shift)?;
        vectices.push(x);
        vectices.push(y);
    }
    Ok(vectices)
}

fn convert_u32_to_i32(values: &[u32]) -> Result<Vec<i32>, MltError> {
    values
        .iter()
        .map(|&v| {
            i32::try_from(v).map_err(|e| MltError::DecodeError(format!("Conversion failed: {e}")))
        })
        .collect()
}

struct PhysicalDecodeCase {
    name: &'static str,
    encoded_bytes: Vec<u8>,
    metadata: StreamMetadata,
    expected: Vec<u32>,
}

fn generate_physical_decode_cases() -> Vec<PhysicalDecodeCase> {
    // For FastPFor-encoded example
    let input = vec![5, 10, 15, 20, 25, 30, 35];
    let codec = FastPFor128Codec::new();
    let mut tmp = vec![0; input.len()];
    let encoded = codec.encode32(&input, &mut tmp).unwrap();
    let encoded_bytes = encoded_u32s_to_bytes(encoded);

    vec![
        // FastPFor-encoded example
        PhysicalDecodeCase {
            name: "fastpfor_basic",
            encoded_bytes,
            metadata: StreamMetadata {
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::FastPfor,
                ),
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::None,
                    LogicalLevelTechnique::None,
                ),
                num_values: input.len() as u32,
                byte_length: std::mem::size_of_val(encoded) as u32,
                morton: None,
                rle: None,
            },
            expected: input,
        },
        // Varint-encoded value 300 -> [0b10101100, 0b00000010]
        PhysicalDecodeCase {
            name: "varint_single_value",
            encoded_bytes: vec![0b10101100, 0b00000010],
            metadata: StreamMetadata {
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::None,
                    LogicalLevelTechnique::None,
                ),
                num_values: 1,
                byte_length: 2,
                morton: None,
                rle: None,
            },
            expected: vec![300],
        },
    ]
}

struct LogicalDecodeCase {
    name: &'static str,
    values: Vec<u32>,
    metadata: StreamMetadata,
    is_signed: bool,
    expected: Vec<i32>,
}

fn generate_logical_decode_cases() -> Vec<LogicalDecodeCase> {
    // for morton encoding, the original coordinates are shifted by a constant value
    let shift = 2;
    let original = [3 + shift, 1 + shift];
    let encoded = morton_encode(original) as u32;

    vec![
        // Morton
        LogicalDecodeCase {
            name: "morton",
            values: vec![encoded],
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::Morton,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 1,
                byte_length: 0,
                morton: Some(Morton {
                    num_bits: 32, // Or however many bits your app uses
                    coordinate_shift: shift,
                }),
                rle: None,
            },
            is_signed: false,
            expected: vec![3, 1],
        },
        // Logical::None unsigned
        LogicalDecodeCase {
            name: "none_unsigned",
            values: vec![1, 2, 3],
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::None,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 3,
                byte_length: 0,
                morton: None,
                rle: None,
            },
            is_signed: false,
            expected: vec![1, 2, 3],
        },
        // Logical::None signed (ZigZag)
        LogicalDecodeCase {
            name: "none_signed_zigzag",
            values: vec![0, 1, 2],
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::None,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 3,
                byte_length: 0,
                morton: None,
                rle: None,
            },
            is_signed: true,
            expected: vec![0, -1, 1],
        },
        // Logical::Delta + No-RLE + ZigZag
        LogicalDecodeCase {
            name: "delta_zigzag",
            values: vec![0u32, 1, 2, 3, 4, 5],
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::Delta,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 6,
                byte_length: 0,
                morton: None,
                rle: None,
            },
            is_signed: true,
            expected: vec![0i32, -1, 0, -2, 0, -3],
        },
        // Logical::Delta + RLE + ZigZag
        LogicalDecodeCase {
            name: "delta_rle_zigzag",
            values: vec![3, 2, 1, 0, 1, 2], // RLE: [0, 0, 0, 1, 1, 2] → ZigZag: [0, 0, 0, -1, -1, 1] → Accum: [0, 0, 0, -1, -2, -1]
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::Delta,
                    LogicalLevelTechnique::Rle,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 3,
                byte_length: 0,
                morton: None,
                rle: Some(Rle {
                    runs: 3,
                    num_rle_values: 6,
                }),
            },
            is_signed: true,
            expected: vec![0, 0, 0, -1, -2, -1],
        },
        // Logical::RLE only
        LogicalDecodeCase {
            name: "rle_unsigned",
            values: vec![3, 2, 1, 10, 20, 30], // RLE → [10,10,10,20,20,30]
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::Rle,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 6,
                byte_length: 0,
                morton: None,
                rle: Some(Rle {
                    runs: 3,
                    num_rle_values: 6,
                }),
            },
            is_signed: false,
            expected: vec![10, 10, 10, 20, 20, 30],
        },
        // Componentwise Delta
        LogicalDecodeCase {
            name: "componentwise_delta",
            values: vec![6, 10, 8, 2, 10, 3], // Encoded from [(3, 5), (7, 6), (12, 4)]
            metadata: StreamMetadata {
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::ComponentwiseDelta,
                    LogicalLevelTechnique::None,
                ),
                physical: Physical::new(
                    PhysicalStreamType::Present,
                    PhysicalLevelTechnique::Varint,
                ),
                num_values: 6, // 3 Vec2s = 6 values
                byte_length: 0,
                morton: None,
                rle: None,
            },
            is_signed: false,
            expected: vec![3, 5, 7, 6, 12, 4],
        },
    ]
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_decode_physical_all_cases() {
        for case in generate_physical_decode_cases() {
            let mut tile: TrackedBytes = case.encoded_bytes.clone().into();
            let result = decode_physical(&mut tile, &case.metadata).unwrap();
            assert_eq!(
                result, case.expected,
                "Test '{}' failed: expected {:?}, got {:?}",
                case.name, case.expected, result
            );
        }
    }

    #[test]
    fn test_decode_logical_all_cases() {
        for case in generate_logical_decode_cases() {
            let result =
                decode_logical(case.values.clone(), &case.metadata, case.is_signed).unwrap();
            assert_eq!(
                result, case.expected,
                "Test '{}' failed: expected {:?}, got {:?}",
                case.name, case.expected, result
            );
        }
    }

    #[test]
    fn test_decode_fast_pfor() {
        // Encode a sample input using FastPFor128Codec
        let codec = FastPFor128Codec::new();
        let input = vec![5, 10, 15, 20, 25, 30, 35];
        let mut tmp = vec![0; input.len()];
        let encoded = codec.encode32(&input, &mut tmp).unwrap();
        let byte_length = std::mem::size_of_val(encoded) as u32;
        let num_values = input.len() as u32;

        // Prepare the tile as a TrackedBytes instance
        let encoded_bytes: Vec<u8> = encoded_u32s_to_bytes(encoded);
        let mut tile: TrackedBytes = encoded_bytes.into();

        // Create a StreamMetadata instance
        let metadata = StreamMetadata {
            logical: Logical::new(
                Some(LogicalStreamType::Dictionary(None)),
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(
                PhysicalStreamType::Present,
                PhysicalLevelTechnique::FastPfor,
            ),
            num_values,
            byte_length,
            morton: None,
            rle: None,
        };
        let result = decode_fast_pfor(&mut tile, &metadata).unwrap();
        assert_eq!(input, result);
        assert_eq!(tile.offset(), byte_length as usize);
    }

    #[test]
    fn test_bytes_to_encoded_u32s() {
        let mut tile: TrackedBytes = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]
            .as_slice()
            .into();
        let result = bytes_to_encoded_u32s(&mut tile, 8).unwrap();
        assert_eq!(result, [0x12345678, 0x90abcdef]);
    }

    #[test]
    fn test_decode_rle() {
        let rle_meta = Rle {
            runs: 3,
            num_rle_values: 6,
        };

        assert_eq!(
            decode_rle::<u32>(&[3, 2, 1, 10, 20, 30], &rle_meta).unwrap(),
            [10, 10, 10, 20, 20, 30]
        );

        assert_eq!(
            decode_rle::<i32>(&[3, 2, 1, -10, 20, 30], &rle_meta).unwrap(),
            [-10, -10, -10, 20, 20, 30]
        );

        let rle_zero = Rle {
            runs: 3,
            num_rle_values: 3,
        };

        assert_eq!(
            decode_rle::<u32>(&[0, 2, 1, 10, 20, 30], &rle_zero).unwrap(),
            [20, 20, 30] // 0 * 10 skipped, 2 * 20, 1 * 30
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

    #[test]
    fn test_decode_zigzag_delta() {
        let encoded_deltas_u32 = [0u32, 1, 2, 3, 4, 5];
        let expected_deltas_i32 = [0i32, -1, 0, -2, 0, -3];
        let decoded_deltas_i32 = decode_zigzag_delta::<i32>(&encoded_deltas_u32);
        assert_eq!(decoded_deltas_i32, expected_deltas_i32);

        let edge_encoded = [u32::MAX];
        let decoded = decode_zigzag_delta::<i32>(&edge_encoded);
        assert_eq!(decoded, [i32::MIN]);

        let overflow_encoded = [0u32, u32::MAX];
        let decoded = decode_zigzag_delta::<i32>(&overflow_encoded);
        assert_eq!(decoded, [0, i32::MIN]);
    }

    #[test]
    fn test_decode_morton_u64_to_i32_vec2() {
        // Original coordinates: [3, 1]
        // Shift: 2
        // shifted_x, shifted_y: [u32; 2] = [5, 3]; [0101, 0011]
        // It goes x3:1, y3:0, x2:0, y2:1, x1:1, y1:1
        // Encoded Morton code: 39 (binary: 100111)
        // Encoded [5, 3] = 39
        let encoded: u64 = 39;
        let coordinate_shift: u32 = 2;
        let decoded = decode_morton_u64_to_i32_vec2(encoded, coordinate_shift).unwrap();
        assert_eq!(decoded, [3i32, 1]);

        // Max coordinates the function can handle
        let original = [1 << (31 - 1), 1 << (31 - 1)];
        let encoded_large: u64 = morton_encode(original);
        let coordinate_shift: u32 = 0;
        let decoded_large = decode_morton_u64_to_i32_vec2(encoded_large, coordinate_shift).unwrap();
        let decoded_u32: [u32; 2] = decoded_large.map(|x| x.try_into().unwrap());
        assert_eq!(decoded_u32, original);
    }

    #[test]
    fn test_decode_morton_u64_to_i32_vec2s_flat() {
        // Use the same example as above
        let input: [u64; 1] = [39];
        let result = decode_morton_u64_to_i32_vec2s_flat(&input, 2).unwrap();
        assert_eq!(result, vec![3, 1]);
    }
}
