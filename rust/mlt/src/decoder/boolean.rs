use bytes::Buf;

use crate::decoder::tracked_bytes::TrackedBytes;
use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::PhysicalStreamType;
use crate::{MltError, MltResult};

pub fn decode_boolean_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<u8>> {
    if metadata.physical.r#type != PhysicalStreamType::Present {
        return Err(MltError::InvalidPhysicalStreamType(
            metadata.physical.r#type as u8,
        ));
    }
    Ok(decode_boolean_rle(tile, metadata.num_values as usize))
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
pub fn bytes_to_booleans(bytes: &[u8], num_booleans: usize) -> Vec<bool> {
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
    use crate::metadata::stream_encoding::{
        Logical, LogicalLevelTechnique, LogicalStreamType, Physical, PhysicalLevelTechnique,
    };

    #[test]
    fn test_decode_boolean_stream() {
        let mut tile: TrackedBytes = [0x03, 0x01].as_slice().into();
        let dummy_metadata = StreamMetadata {
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            logical: Logical::new(
                Some(LogicalStreamType::Dictionary(None)),
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            num_values: 5,
            byte_length: 2,
            morton: None,
            rle: None,
        };

        let result = decode_boolean_stream(&mut tile, &dummy_metadata).unwrap();
        assert_eq!(result, vec![1]);
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
