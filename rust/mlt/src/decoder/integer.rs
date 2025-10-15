use std::fmt::Debug;

use bytes::Buf;
use fastpfor::cpp::{Codec32 as _, FastPFor128Codec};
use morton_encoding::{morton_decode, morton_encode};
use num_traits::PrimInt;
use zigzag::ZigZag;

use crate::decoder::integer_stream::decode_componentwise_delta_vec2s;
use crate::decoder::tracked_bytes::TrackedBytes;
use crate::decoder::varint;
use crate::encoder::integer::u32s_to_le_bytes;
use crate::metadata::stream::{Morton, Rle, StreamMetadata};
use crate::metadata::stream_encoding::{
    Logical, LogicalLevelTechnique, LogicalStreamType, Physical, PhysicalLevelTechnique,
    PhysicalStreamType,
};
use crate::{MltError, MltResult};

pub fn decode_long_stream(
    _tile: &mut TrackedBytes,
    _metadata: &StreamMetadata,
    _is_signed: bool,
) -> MltResult<Vec<u64>> {
    todo!()
}

/// `decode_int_stream` can handle multiple decoding techniques,
/// some of which do represent signed integers (like varint with [`ZigZag`])
/// so returning `Vec<i32>`
pub fn decode_int_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
    is_signed: bool,
) -> MltResult<Vec<i32>> {
    let values = decode_physical(tile, metadata)?;
    decode_logical(&values, metadata, is_signed)
}

/// Byte-level decoding based on the physical technique and stream type
pub fn decode_physical(tile: &mut TrackedBytes, metadata: &StreamMetadata) -> MltResult<Vec<u32>> {
    match &metadata.physical.technique {
        PhysicalLevelTechnique::FastPfor => decode_fast_pfor(tile, metadata),
        PhysicalLevelTechnique::Varint => varint::decode::<u32>(tile, metadata.num_values as usize),
        PhysicalLevelTechnique::None => le_bytes_to_u32s(tile, metadata.byte_length as usize),
        PhysicalLevelTechnique::Alp => Err(MltError::UnsupportedPhysicalTechnique(
            PhysicalLevelTechnique::Alp,
        )),
    }
}

/// Logical-level decoding based on the logical technique
fn decode_logical(
    values: &[u32],
    metadata: &StreamMetadata,
    is_signed: bool,
) -> MltResult<Vec<i32>> {
    match metadata.logical.technique1 {
        Some(LogicalLevelTechnique::Delta) => {
            if metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle) {
                let rle_metadata = metadata
                    .rle
                    .as_ref()
                    .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?;
                let values = decode_rle(values, rle_metadata)?;
                Ok(decode_zigzag_delta(&values))
            } else {
                Ok(decode_zigzag_delta(values))
            }
        }
        Some(LogicalLevelTechnique::Rle) => {
            let rle_metadata = metadata
                .rle
                .as_ref()
                .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?;
            let values = decode_rle(values, rle_metadata)?;
            if is_signed {
                Ok(decode_zigzag(&values))
            } else {
                convert_u32_to_i32(&values)
            }
        }
        Some(LogicalLevelTechnique::None) => {
            if is_signed {
                Ok(decode_zigzag(values))
            } else {
                convert_u32_to_i32(values)
            }
        }
        Some(LogicalLevelTechnique::Morton) => {
            let morton_metadata = metadata
                .morton
                .as_ref()
                .ok_or(MltError::MissingLogicalMetadata { which: "morton" })?;
            decode_morton_u64_to_i32_vec2s_flat(values, morton_metadata.coordinate_shift)
        }
        Some(LogicalLevelTechnique::ComponentwiseDelta) => decode_componentwise_delta_vec2s(values),
        Some(LogicalLevelTechnique::Pde) => Err(MltError::UnsupportedLogicalTechnique(
            LogicalLevelTechnique::Pde,
        )),
        None => Err(MltError::MissingField("logical.technique1")),
    }
}

fn decode_fast_pfor(tile: &mut TrackedBytes, metadata: &StreamMetadata) -> MltResult<Vec<u32>> {
    let codec = FastPFor128Codec::new();
    let expected_len = metadata.num_values as usize;
    let mut decoded = vec![0; expected_len];
    // Regardless of u32 or u64 fastpfor, the encoded data is always u32
    let encoded_u32s: Vec<u32> = le_bytes_to_u32s(tile, metadata.byte_length as usize)?;
    let decoded_slice = codec.decode32(&encoded_u32s, &mut decoded)?;
    let actual_len = decoded_slice.len();
    if actual_len != expected_len {
        return Err(MltError::FastPforDecode {
            expected: expected_len,
            got: actual_len,
        });
    }
    Ok(decoded)
}

/// Convert a byte stream (little-endian, LE) to a vector of u32 integers.
fn le_bytes_to_u32s(tile: &mut TrackedBytes, num_bytes: usize) -> MltResult<Vec<u32>> {
    if !num_bytes.is_multiple_of(4) {
        return Err(MltError::InvalidByteMultiple {
            ctx: "bytes-to-be-encoded-u32 stream",
            multiple_of: 4,
            got: num_bytes,
        });
    }
    if tile.remaining() < num_bytes {
        return Err(MltError::BufferUnderflow {
            needed: num_bytes,
            remaining: tile.remaining(),
        });
    }

    let num_ints = num_bytes / 4;
    let encoded_u32s = (0..num_ints).map(|_| tile.get_u32_le()).collect();
    Ok(encoded_u32s)
}

/// Convert a byte stream (little-endian, LE) to a vector of u64 integers.
fn le_bytes_to_u64s(tile: &mut TrackedBytes, num_bytes: usize) -> MltResult<Vec<u64>> {
    if !num_bytes.is_multiple_of(8) {
        return Err(MltError::InvalidByteMultiple {
            ctx: "bytes-to-be-encoded-u64 stream",
            multiple_of: 8,
            got: num_bytes,
        });
    }
    if tile.remaining() < num_bytes {
        return Err(MltError::BufferUnderflow {
            needed: num_bytes,
            remaining: tile.remaining(),
        });
    }

    let num_ints = num_bytes / 8;
    let encoded_u64s = (0..num_ints).map(|_| tile.get_u64_le()).collect();
    Ok(encoded_u64s)
}

/// Decode RLE (Run-Length Encoding) data
/// It serves the same purpose as the `decodeUnsignedRLE` and `decodeRLE` methods in the Java code.
fn decode_rle<T: PrimInt + Debug>(data: &[T], rle_meta: &Rle) -> MltResult<Vec<T>> {
    let runs = rle_meta.runs as usize;
    let total = rle_meta.num_rle_values as usize;
    let (run_lens, values) = data.split_at(runs);
    let mut result = Vec::with_capacity(total);
    for (&run, &val) in run_lens.iter().zip(values.iter()) {
        let run_len = run
            .to_usize()
            .ok_or_else(|| MltError::RleRunLenInvalid(run.to_i128().unwrap_or_default()))?;
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
) -> MltResult<[i32; 2]> {
    let encoded: u64 = morton_code.into();
    let [xu, yu]: [u32; 2] = morton_decode(encoded);

    // Convert u32 to i32, checking for overflow
    let shift =
        i32::try_from(coordinate_shift).map_err(|_| MltError::ShiftTooLarge(coordinate_shift))?;
    let x_i32 = i32::try_from(xu).map_err(|_| MltError::CoordinateOverflow {
        coordinate: xu,
        shift: coordinate_shift,
    })?;
    let y_i32 = i32::try_from(yu).map_err(|_| MltError::CoordinateOverflow {
        coordinate: yu,
        shift: coordinate_shift,
    })?;

    let x = x_i32.checked_sub(shift).ok_or(MltError::SubtractOverflow {
        left_val: x_i32,
        right_val: shift,
    })?;
    let y = y_i32.checked_sub(shift).ok_or(MltError::SubtractOverflow {
        left_val: y_i32,
        right_val: shift,
    })?;

    Ok([x, y])
}

fn decode_morton_u64_to_i32_vec2s_flat<T: Into<u64> + Copy>(
    data: &[T],
    coordinate_shift: u32,
) -> MltResult<Vec<i32>> {
    let mut vertices: Vec<i32> = Vec::with_capacity(data.len() * 2);
    for &code in data {
        let [x, y] = decode_morton_u64_to_i32_vec2(code, coordinate_shift)?;
        vertices.push(x);
        vertices.push(y);
    }
    Ok(vertices)
}

fn convert_u32_to_i32(values: &[u32]) -> MltResult<Vec<i32>> {
    values
        .iter()
        .map(|&v| {
            i32::try_from(v).map_err(|_| MltError::ConversionOverflow {
                from: "u32",
                to: "i32",
                value: u64::from(v),
            })
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
    let encoded_bytes = u32s_to_le_bytes(encoded);

    vec![
        // PhysicalLevelTechnique::FastPfor example
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
                byte_length: size_of_val(encoded) as u32,
                morton: None,
                rle: None,
            },
            expected: input,
        },
        // PhysicalLevelTechnique::Varint example
        // Varint-encoded value 300 -> [0b10101100, 0b00000010]
        PhysicalDecodeCase {
            name: "varint_single_value",
            encoded_bytes: vec![0b1010_1100, 0b0000_0010],
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
        // PyhsicalLevelTechnique::None example
        // Little-endian bytes for [0x01,0x00,0x00,0x00, 0x00,0x01,0x00,0x00, 0x00,0x00,0x01,0x00] -> [1u32, 256u32, 65536u32]
        PhysicalDecodeCase {
            name: "le_bytes_multiple_values",
            encoded_bytes: vec![
                0x01, 0x00, 0x00, 0x00, // 1
                0x00, 0x01, 0x00, 0x00, // 256
                0x00, 0x00, 0x01, 0x00, // 65536
            ],
            metadata: StreamMetadata {
                physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
                logical: Logical::new(
                    Some(LogicalStreamType::Dictionary(None)),
                    LogicalLevelTechnique::None,
                    LogicalLevelTechnique::None,
                ),
                num_values: 3,
                byte_length: 12,
                morton: None,
                rle: None,
            },
            expected: vec![1, 256, 65536],
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
            let result = decode_logical(&case.values, &case.metadata, case.is_signed).unwrap();
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
        let byte_length = size_of_val(encoded) as u32;
        let num_values = input.len() as u32;

        // Prepare the tile as a TrackedBytes instance
        let encoded_bytes: Vec<u8> = u32s_to_le_bytes(encoded);
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
    fn test_le_bytes_to_u32s() {
        let mut tile: TrackedBytes = [0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90]
            .as_slice()
            .into();
        let result = le_bytes_to_u32s(&mut tile, 8).unwrap();
        assert_eq!(result, [0x1234_5678, 0x90ab_cdef]);
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
