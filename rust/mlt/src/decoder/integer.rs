use std::fmt::Debug;

use bytes::Buf;
use fastpfor::cpp::{Codec32 as _, FastPFor128Codec};
use morton_encoding::{morton_decode, morton_encode};
use num_traits::PrimInt;
use zigzag::ZigZag;

use crate::MltError;
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

/// a placeholder for future implementation
/// For some reason, the Java code has a method that decodes long streams,
/// but it has different logic than the integer stream decoding.
/// It is not clear what the purpose of this method is, so it is left unimplemented
pub fn decode_long_stream(
    _tile: &mut TrackedBytes,
    _metadata: &StreamMetadata,
    _is_signed: bool,
) -> Result<Vec<u64>, MltError> {
    todo!()
}

/// `decode_int_stream` can handle multiple decoding techniques,
/// some of which do represent signed integers (like varint with [`ZigZag`])
/// so returning Vec<i32>
pub fn decode_int_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
    is_signed: bool,
) -> Result<Vec<i32>, MltError> {
    let values = decode_physical(tile, metadata)?;
    decode_logical(&values, metadata, is_signed)
}

/// Byte-level decoding based on the physical technique and stream type
pub fn decode_physical(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> Result<Vec<u32>, MltError> {
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
) -> Result<Vec<i32>, MltError> {
    match metadata.logical.technique1 {
        Some(LogicalLevelTechnique::Delta) => {
            if metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle) {
                let rle_metadata = metadata
                    .rle
                    .as_ref()
                    .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?;
                let values = decode_rle(values, rle_metadata)?;
                return Ok(decode_zigzag_delta(&values));
            }
            Ok(decode_zigzag_delta(values))
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
                Ok(convert_u32_to_i32(&values)?)
            }
        }
        Some(LogicalLevelTechnique::None) => {
            if is_signed {
                return Ok(decode_zigzag(values));
            }
            Ok(convert_u32_to_i32(values)?)
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

fn decode_fast_pfor(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> Result<Vec<u32>, MltError> {
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
fn le_bytes_to_u32s(tile: &mut TrackedBytes, num_bytes: usize) -> Result<Vec<u32>, MltError> {
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

    let num_bytes = num_bytes / 4;
    let encoded_u32s = (0..num_bytes)
        .map(|_| {
            let b1 = tile.get_u8();
            let b2 = tile.get_u8();
            let b3 = tile.get_u8();
            let b4 = tile.get_u8();
            u32::from_le_bytes([b1, b2, b3, b4])
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
) -> Result<[i32; 2], MltError> {
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

/// Decode a physical level technique.
fn decode_physical_level_technique(
    data: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<u32>> {
    match metadata.physical.technique {
        PhysicalLevelTechnique::Varint => varint::decode::<u32>(data, metadata.num_values as usize),
        PhysicalLevelTechnique::None => {
            let byte_length = metadata.byte_length as usize;
            let mut values = Vec::with_capacity(byte_length / 4);

            // Read the raw bytes directly from the TrackedBytes
            for _ in 0..(byte_length / 4) {
                if data.remaining() < 4 {
                    return Err(MltError::InsufficientData);
                }
                let value = data.get_u32_le(); // Read u32 in little-endian format
                values.push(value);
            }

            Ok(values)
        }
        _ => Err(MltError::UnsupportedPhysicalTechnique(
            metadata.physical.technique,
        )),
    }
}

// Decode a length stream into an offset buffer.
pub fn decode_length_stream_to_offset_buffer(
    data: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<i32>> {
    #[expect(unused)]
    let values = decode_physical_level_technique(data, metadata)?;
    todo!("Implement length stream to offset buffer decoding");
}

// Decode a length stream into an offset buffer based on logical techniques.
fn decode_length_to_offset_buffer(
    values: &mut [u32],
    stream_metadata: &StreamMetadata,
) -> MltResult<Vec<i32>> {
    if stream_metadata.logical.technique1 == Some(LogicalLevelTechnique::Delta)
        && stream_metadata.logical.technique2 == Some(LogicalLevelTechnique::None)
    {
        let decoded = zigzag_delta_of_delta_decoding(values);
        return Ok(decoded);
    }

    if stream_metadata.logical.technique1 == Some(LogicalLevelTechnique::Rle)
        && stream_metadata.logical.technique2 == Some(LogicalLevelTechnique::None)
    {
        let runs = stream_metadata
            .rle
            .as_ref()
            .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?
            .runs as usize;
        let num_total_values = stream_metadata
            .rle
            .as_ref()
            .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?
            .num_rle_values as usize;
        let decoded = rle_delta_decoding(values, runs, num_total_values);
        return Ok(decoded);
    }

    if stream_metadata.logical.technique1 == Some(LogicalLevelTechnique::None)
        && stream_metadata.logical.technique2 == Some(LogicalLevelTechnique::None)
    {
        inverse_delta(values);
        let mut offsets = Vec::with_capacity(values.len() + 1);
        offsets.push(0);
        offsets.extend(values.iter().map(|&x| x as i32));
        return Ok(offsets);
    }

    if stream_metadata.logical.technique1 == Some(LogicalLevelTechnique::Delta)
        && stream_metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle)
    {
        let runs = stream_metadata
            .rle
            .as_ref()
            .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?
            .runs as usize;
        let num_total_values = stream_metadata
            .rle
            .as_ref()
            .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?
            .num_rle_values as usize;
        let mut decoded = zigzag_rle_delta_decoding(values, runs, num_total_values);
        fast_inverse_delta(&mut decoded);
        return Ok(decoded);
    }
    Err(MltError::UnsupportedLogicalTechniqueCombination {
        technique1: stream_metadata.logical.technique1,
        technique2: stream_metadata.logical.technique2,
    })
}

// Delta-of-delta decoding with ZigZag decoding
fn zigzag_delta_of_delta_decoding(data: &[u32]) -> Vec<i32> {
    if data.is_empty() {
        return vec![0];
    }

    let mut decoded_data = Vec::with_capacity(data.len() + 1);
    decoded_data.push(0);
    decoded_data.push(ZigZag::decode(data[0]));

    let mut delta_sum = decoded_data[1];

    for &zig_zag_value in &data[1..] {
        let delta: i32 = ZigZag::decode(zig_zag_value);
        delta_sum += delta;
        decoded_data.push(decoded_data.last().unwrap() + delta_sum);
    }

    decoded_data
}

/// RLE delta decoding
fn rle_delta_decoding(data: &[u32], num_runs: usize, num_total_values: usize) -> Vec<i32> {
    let mut decoded_values = vec![0; num_total_values + 1];
    let mut offset = 1;
    let mut previous_value = 0;

    for i in 0..num_runs {
        let run_length = data[i] as usize;
        let value = data[i + num_runs] as i32;

        for decoded_value in decoded_values.iter_mut().skip(offset).take(run_length) {
            *decoded_value = value + previous_value;
            previous_value = *decoded_value;
        }

        offset += run_length;
    }

    decoded_values
}

/// Inverse delta decoding for unsigned integers
fn inverse_delta(data: &mut [u32]) {
    let mut prev_value = 0u32;
    for value in data.iter_mut() {
        *value = value.wrapping_add(prev_value);
        prev_value = *value;
    }
}

/// RLE delta decoding with ZigZag decoding
fn zigzag_rle_delta_decoding(data: &[u32], num_runs: usize, num_total_values: usize) -> Vec<i32> {
    let mut decoded_values = vec![0i32; num_total_values + 1];
    decoded_values[0] = 0;
    let mut offset = 1;
    let mut previous_value = decoded_values[0];

    for i in 0..num_runs {
        let run_length = data[i] as usize;
        let value = data[i + num_runs];

        let decoded_value = ((value >> 1) ^ (0u32.wrapping_sub(value & 1))) as i32;

        for decoded_val in decoded_values.iter_mut().skip(offset).take(run_length) {
            *decoded_val = decoded_value + previous_value;
            previous_value = *decoded_val;
        }

        offset += run_length;
    }

    decoded_values
}

/// Fast inverse delta decoding for signed integers
fn fast_inverse_delta(data: &mut [i32]) {
    if data.is_empty() {
        return;
    }

    let sz0 = (data.len() / 4) * 4;
    let mut i = 1;

    if sz0 >= 4 {
        let mut a = data[0];
        while i < sz0.saturating_sub(4) {
            a = {
                data[i] = data[i].wrapping_add(a);
                data[i]
            };
            a = {
                data[i + 1] = data[i + 1].wrapping_add(a);
                data[i + 1]
            };
            a = {
                data[i + 2] = data[i + 2].wrapping_add(a);
                data[i + 2]
            };
            a = {
                data[i + 3] = data[i + 3].wrapping_add(a);
                data[i + 3]
            };
            i += 4;
        }
    }

    while i < data.len() {
        data[i] = data[i].wrapping_add(data[i - 1]);
        i += 1;
    }
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

    use bytes::Bytes;

    #[test]
    fn test_decode_physical_level_technique_varint() {
        let bytes: Bytes = Bytes::from(vec![
            0x01, // 1
            0xAC, 0x02, // 300
            0xD0, 0x86, 0x03, // 50000
        ]);
        let mut tile: TrackedBytes = bytes.into();
        let metadata = StreamMetadata {
            logical: Logical::new(
                Some(LogicalStreamType::Dictionary(None)),
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::Varint),
            num_values: 3,
            byte_length: 0,
            morton: None,
            rle: None,
        };

        let decoded = decode_physical_level_technique(&mut tile, &metadata).unwrap();
        assert_eq!(decoded, vec![1, 300, 50000]);
    }

    #[test]
    fn test_decode_physical_level_technique_none() {
        let bytes: Bytes = Bytes::from(vec![
            // 1 in little-endian
            0x01, 0x00, 0x00, 0x00, // 300 in little-endian
            0x2C, 0x01, 0x00, 0x00, // 50000 in little-endian
            0x50, 0xC3, 0x00, 0x00,
        ]);

        let mut tile: TrackedBytes = bytes.into();
        let metadata = StreamMetadata {
            logical: Logical::new(
                Some(LogicalStreamType::Dictionary(None)),
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 3,
            byte_length: 12, // 3 values * 4 bytes each
            morton: None,
            rle: None,
        };

        let decoded = decode_physical_level_technique(&mut tile, &metadata).unwrap();
        assert_eq!(decoded, vec![1, 300, 50000]);
    }

    #[test]
    fn test_decode_physical_level_technique_none_empty() {
        let bytes: Bytes = Bytes::from(vec![]);
        let mut tile: TrackedBytes = bytes.into();

        let metadata = StreamMetadata {
            logical: Logical::new(
                Some(LogicalStreamType::Dictionary(None)),
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 0,
            byte_length: 0,
            morton: None,
            rle: None,
        };

        let decoded = decode_physical_level_technique(&mut tile, &metadata).unwrap();
        assert_eq!(decoded, Vec::<u32>::new());
    }

    #[test]
    fn test_zigzag_delta_of_delta_decoding_simple_sequence() {
        let data = vec![20u32, 10, 12];
        let decoded = zigzag_delta_of_delta_decoding(&data);
        let expected = vec![0, 10, 25, 46];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_zigzag_delta_of_delta_decoding_mixed_values() {
        // Test with mixed positive/negative delta-of-deltas
        let data = vec![16u32, 11, 11, 12];
        let decoded = zigzag_delta_of_delta_decoding(&data);

        let expected = vec![0, 8, 10, 6, 8];
        assert_eq!(decoded, expected);
        assert_eq!(decoded.len(), 5);
    }

    #[test]
    fn test_zigzag_delta_of_delta_decoding_single_value() {
        // Test with just one value
        let data = vec![10u32];
        let decoded = zigzag_delta_of_delta_decoding(&data);

        let expected = vec![0, 5];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_zigzag_delta_of_delta_decoding_empty_data() {
        let data = vec![];
        let decoded = zigzag_delta_of_delta_decoding(&data);

        assert_eq!(decoded, vec![0]);
        assert_eq!(decoded.len(), 1);
    }

    #[test]
    fn test_single_run() {
        let data = [3, 5];
        let result = rle_delta_decoding(&data, 1, 3);
        assert_eq!(result, vec![0, 5, 10, 15]);
    }

    #[test]
    fn test_zigzag_rle_delta_decoding_basic() {
        let data = vec![2u32, 2, 10, 6];
        let num_runs = 2;
        let num_total_values = 4;

        let result = zigzag_rle_delta_decoding(&data, num_runs, num_total_values);

        let expected = vec![0, 5, 10, 13, 16];
        assert_eq!(result, expected);
        assert_eq!(result.len(), num_total_values + 1);
    }

    #[test]
    fn test_zigzag_rle_delta_decoding_with_negative_values() {
        let data = vec![3u32, 1, 7, 4];
        let num_runs = 2;
        let num_total_values = 4;

        let result = zigzag_rle_delta_decoding(&data, num_runs, num_total_values);

        let expected = vec![0, -4, -8, -12, -10];
        assert_eq!(result, expected);
        assert_eq!(result.len(), num_total_values + 1);
    }

    #[test]
    fn test_fast_inverse_delta_vectorized_processing() {
        let mut data = vec![10i32, 5, 3, 2, 1, 4, 6, 2];
        fast_inverse_delta(&mut data);

        let expected = vec![10i32, 15, 18, 20, 21, 25, 31, 33];
        assert_eq!(data, expected);
    }

    #[test]
    fn test_fast_inverse_delta_mixed_processing() {
        let mut data = vec![1i32, 2, 3, 4, 5, 10];
        fast_inverse_delta(&mut data);

        let expected = vec![1i32, 3, 6, 10, 15, 25];
        assert_eq!(data, expected);
    }

    fn create_delta_metadata() -> StreamMetadata {
        StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::Delta,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 0,
            byte_length: 0,
            morton: None,
            rle: None,
        }
    }

    #[test]
    fn test_decode_length_to_offset_buffer_delta_technique() {
        let mut values = vec![20u32, 10, 12];
        let metadata = create_delta_metadata();

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        let expected = vec![0, 10, 25, 46];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_length_to_offset_buffer_delta_empty_values() {
        let mut values = vec![];
        let metadata = create_delta_metadata();

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        assert_eq!(decoded, vec![0]);
    }

    fn create_rle_metadata(runs: u32, num_rle_values: u32) -> StreamMetadata {
        StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::Rle,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 0,
            byte_length: 0,
            morton: None,
            rle: Some(Rle {
                runs,
                num_rle_values,
            }),
        }
    }

    #[test]
    fn test_decode_length_to_offset_buffer_rle_technique() {
        let mut values = vec![2u32, 2, 5, 3];
        let metadata = create_rle_metadata(2, 4);

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        let expected = vec![0, 5, 10, 13, 16];
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_length_to_offset_buffer_rle_missing_metadata() {
        let mut values = vec![2u32, 2, 5, 3];
        let mut metadata = create_rle_metadata(2, 4);
        metadata.rle = None; // Remove RLE metadata

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_err());
        match result {
            Err(MltError::MissingLogicalMetadata { which }) => {
                assert_eq!(which, "rle");
            }
            _ => panic!("Expected MissingLogicalMetadata error"),
        }
    }

    fn create_delta_rle_metadata(runs: u32, num_rle_values: u32) -> StreamMetadata {
        StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::Delta,
                LogicalLevelTechnique::Rle,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 0,
            byte_length: 0,
            morton: None,
            rle: Some(Rle {
                runs,
                num_rle_values,
            }),
        }
    }

    #[test]
    fn test_decode_length_to_offset_buffer_delta_rle_technique() {
        let mut values = vec![2u32, 1, 10, 6];
        let metadata = create_delta_rle_metadata(2, 3);

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        assert!(!decoded.is_empty());
        assert_eq!(decoded[0], 0);
    }

    #[test]
    fn test_decode_length_to_offset_buffer_delta_rle_missing_metadata() {
        let mut values = vec![2u32, 1, 10, 6];
        let mut metadata = create_delta_rle_metadata(2, 3);
        metadata.rle = None;

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_err());
        match result {
            Err(MltError::MissingLogicalMetadata { which }) => {
                assert_eq!(which, "rle");
            }
            _ => panic!("Expected MissingLogicalMetadata error"),
        }
    }

    #[test]
    fn test_decode_length_to_offset_buffer_unsupported_combination() {
        let mut values = vec![1u32, 2, 3];
        // Create an unsupported combination: Delta + Delta
        let metadata = StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::Delta,
                LogicalLevelTechnique::Delta, // Unsupported combination
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 0,
            byte_length: 0,
            morton: None,
            rle: None,
        };

        let result = decode_length_to_offset_buffer(&mut values, &metadata);

        assert!(result.is_err());
        match result {
            Err(MltError::UnsupportedLogicalTechniqueCombination {
                technique1,
                technique2,
            }) => {
                assert_eq!(technique1, Some(LogicalLevelTechnique::Delta));
                assert_eq!(technique2, Some(LogicalLevelTechnique::Delta));
            }
            _ => panic!("Expected UnsupportedLogicalTechniqueCombination error"),
        }
    }
}
