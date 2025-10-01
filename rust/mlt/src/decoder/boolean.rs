use bytes::Buf;
use num_traits::ToPrimitive;

use crate::decoder::tracked_bytes::TrackedBytes;
use crate::metadata::stream::{Rle, StreamMetadata};
use crate::metadata::stream_encoding::{
    LogicalLevelTechnique, PhysicalLevelTechnique, PhysicalStreamType,
};
use crate::{MltError, MltResult};

pub fn decode_boolean_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<bool>> {
    if metadata.physical.r#type != PhysicalStreamType::Present {
        return Err(MltError::InvalidPhysicalStreamType(
            metadata.physical.r#type as u8,
        ));
    }
    let values = decode_boolean_physical(tile, metadata)?;
    decode_boolean_logical(&values, metadata)
}

/// Physical-level decoding for boolean streams.
pub fn decode_boolean_physical(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<u8>> {
    match metadata.physical.technique {
        PhysicalLevelTechnique::None => {
            // Direct byte reading for uncompressed boolean data
            let num_bytes = (metadata.num_values as usize).div_ceil(8);
            if tile.remaining() < num_bytes {
                return Err(MltError::BufferUnderflow {
                    needed: num_bytes,
                    remaining: tile.remaining(),
                });
            }
            let mut result = Vec::with_capacity(num_bytes);
            for _ in 0..num_bytes {
                result.push(tile.get_u8());
            }
            Ok(result)
        }
        PhysicalLevelTechnique::Varint
        | PhysicalLevelTechnique::FastPfor
        | PhysicalLevelTechnique::Alp => Err(MltError::UnsupportedPhysicalTechnique(
            metadata.physical.technique,
        )),
    }
}

/// Logical-level decoding for boolean streams.
fn decode_boolean_logical(values: &[u8], metadata: &StreamMetadata) -> MltResult<Vec<bool>> {
    match metadata.logical.technique1 {
        Some(LogicalLevelTechnique::Rle) => {
            let rle_metadata = metadata
                .rle
                .as_ref()
                .ok_or(MltError::MissingLogicalMetadata { which: "rle" })?;
            let values = decode_boolean_rle_new(values, rle_metadata)?;
            Ok(bytes_to_booleans(&values, metadata.num_values as usize))
        }
        Some(LogicalLevelTechnique::None) | None => {
            Ok(bytes_to_booleans(values, metadata.num_values as usize))
        }
        Some(technique) => Err(MltError::UnsupportedLogicalTechnique(technique)),
    }
}

fn decode_boolean_rle_new(data: &[u8], rle_meta: &Rle) -> MltResult<Vec<u8>> {
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

/// Decodes boolean RLE from the buffer.
/// - `num_booleans` is the total number of booleans (bits).
/// - `byte_size` is inferred as `ceil(num_booleans / 8)`.
pub fn decode_boolean_rle(tile: &mut TrackedBytes, num_booleans: usize) -> Vec<u8> {
    let num_bytes = num_booleans.div_ceil(8);
    decode_byte_rle(tile, num_bytes)
}

/// Decodes byte RLE from the buffer.
/// - `num_bytes` is how many decoded bytes we expect.
pub fn decode_byte_rle(tile: &mut TrackedBytes, num_bytes: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(num_bytes);
    let mut value_offset = 0;

    while value_offset < num_bytes {
        let header = tile.get_u8();

        if header <= 0x7F {
            // Runs
            let num_runs = header as usize + 3;
            let value = tile.get_u8();
            let end_value_offset = value_offset + num_runs;
            result.resize(end_value_offset.min(num_bytes), value);
            value_offset = end_value_offset.min(num_bytes);
        } else {
            // Literals
            let num_literals = 256 - header as usize;
            for _ in 0..num_literals {
                if value_offset >= num_bytes {
                    break;
                }
                result.push(tile.get_u8());
                value_offset += 1;
            }
        }
    }

    result
}

/// Converts a byte array to a boolean array.
/// Each byte contains up to 8 boolean values (bits).
fn bytes_to_booleans(bytes: &[u8], num_booleans: usize) -> Vec<bool> {
    let mut result = Vec::with_capacity(num_booleans);

    for &byte in bytes.iter() {
        for bit_index in 0..8 {
            let global_index = result.len();
            if global_index >= num_booleans {
                break;
            }
            result.push((byte & (1 << bit_index)) != 0);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::stream::{Rle, StreamMetadata};
    use crate::metadata::stream_encoding::{Logical, Physical};

    #[test]
    fn test_decode_boolean_rle_vs_stream_equivalence() {
        use crate::metadata::stream::{Rle, StreamMetadata};
        use crate::metadata::stream_encoding::{
            Logical, LogicalLevelTechnique, Physical, PhysicalLevelTechnique, PhysicalStreamType,
        };

        // Prepare a dummy tile: 0x03, 0x01 encodes 5 bytes of value 1 (see test_decode_byte_rle)
        let mut tile_rle: TrackedBytes = vec![0x03, 0x01].into();
        let mut tile_stream: TrackedBytes = vec![0x03, 0x01].into();

        // Metadata for decode_boolean_rle (just need num_booleans)
        let num_booleans = 5;

        // Metadata for decode_boolean_stream
        let metadata = StreamMetadata {
            num_values: num_booleans as u32,
            byte_length: 2 as u32,
            physical: Physical {
                r#type: PhysicalStreamType::Present,
                technique: PhysicalLevelTechnique::None,
            },
            logical: Logical {
                r#type: None,
                technique1: Some(LogicalLevelTechnique::Rle),
                technique2: None,
            },
            rle: Some(Rle {
                runs: 1,
                num_rle_values: 5,
            }),
            morton: None,
        };

        // decode_boolean_rle returns Vec<u8> (bytes), convert to bools
        let rle_bytes = decode_boolean_rle(&mut tile_rle, num_booleans);
        let rle_bools = bytes_to_booleans(&rle_bytes, num_booleans);

        // decode_boolean_stream returns Vec<bool>
        let stream_bools = decode_boolean_stream(&mut tile_stream, &metadata).unwrap();

        assert_eq!(
            rle_bools, stream_bools,
            "decode_boolean_rle and decode_boolean_stream should produce the same boolean vector"
        );
    }

    #[test]
    fn test_decode_byte_rle() {
        let mut tile: TrackedBytes = [0x03, 0x01].as_slice().into();
        let result = decode_byte_rle(&mut tile, 5);
        assert_eq!(result, vec![1, 1, 1, 1, 1]);
    }

    #[test]
    fn test_decode_boolean_rle() {
        let mut tile: TrackedBytes = [0x03, 0x01].as_slice().into();
        let result = decode_boolean_rle(&mut tile, 5);
        assert_eq!(result, vec![1]);
    }
}
