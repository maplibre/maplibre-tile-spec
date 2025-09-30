use bytes::Buf;

use crate::decoder::tracked_bytes::TrackedBytes;
use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::{
    LogicalLevelTechnique, PhysicalLevelTechnique, PhysicalStreamType,
};
use crate::{MltError, MltResult};

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

/// Decodes a boolean stream based on the stream metadata.
/// This is the main entry point for boolean decoding, similar to `decode_int_stream`.
pub fn decode_boolean_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<bool>> {
    // First check if this is actually a boolean stream
    if metadata.physical.r#type != PhysicalStreamType::Present {
        return Err(MltError::InvalidPhysicalStreamType(
            metadata.physical.r#type as u8,
        ));
    }

    let values = decode_boolean_physical(tile, metadata)?;
    decode_boolean_logical(&values, metadata)
}

/// Physical-level decoding for boolean streams.
/// Handles the byte-level compression techniques.
pub fn decode_boolean_physical(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<Vec<u8>> {
    match metadata.physical.technique {
        PhysicalLevelTechnique::None => {
            // For boolean streams with no physical compression,
            // we expect RLE-encoded bytes
            if metadata.logical.technique1 == Some(LogicalLevelTechnique::Rle) {
                Ok(decode_boolean_rle(tile, metadata.num_values as usize))
            } else {
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
        }
        PhysicalLevelTechnique::Varint => {
            // Varint is not typically used for boolean streams, but handle it
            Err(MltError::UnsupportedPhysicalTechnique(
                PhysicalLevelTechnique::Varint,
            ))
        }
        PhysicalLevelTechnique::FastPfor => {
            // FastPfor is not typically used for boolean streams, but handle it
            Err(MltError::UnsupportedPhysicalTechnique(
                PhysicalLevelTechnique::FastPfor,
            ))
        }
        PhysicalLevelTechnique::Alp => Err(MltError::UnsupportedPhysicalTechnique(
            PhysicalLevelTechnique::Alp,
        )),
    }
}

/// Logical-level decoding for boolean streams.
/// Converts the byte array to a boolean array based on logical techniques.
fn decode_boolean_logical(values: &[u8], metadata: &StreamMetadata) -> MltResult<Vec<bool>> {
    let num_booleans = metadata.num_values as usize;

    match metadata.logical.technique1 {
        Some(LogicalLevelTechnique::Rle) => {
            // RLE is already handled in physical decoding for boolean streams
            // Just convert bytes to booleans
            Ok(bytes_to_booleans(values, num_booleans))
        }
        Some(LogicalLevelTechnique::None) => {
            // No logical transformation, just convert bytes to booleans
            Ok(bytes_to_booleans(values, num_booleans))
        }
        Some(LogicalLevelTechnique::Delta) => {
            // Delta encoding doesn't make sense for boolean streams
            Err(MltError::UnsupportedLogicalTechnique(
                LogicalLevelTechnique::Delta,
            ))
        }
        Some(LogicalLevelTechnique::Morton) => {
            // Morton encoding doesn't make sense for boolean streams
            Err(MltError::UnsupportedLogicalTechnique(
                LogicalLevelTechnique::Morton,
            ))
        }
        Some(LogicalLevelTechnique::ComponentwiseDelta) => {
            // Componentwise delta doesn't make sense for boolean streams
            Err(MltError::UnsupportedLogicalTechnique(
                LogicalLevelTechnique::ComponentwiseDelta,
            ))
        }
        Some(LogicalLevelTechnique::Pde) => {
            // PDE doesn't make sense for boolean streams
            Err(MltError::UnsupportedLogicalTechnique(
                LogicalLevelTechnique::Pde,
            ))
        }
        None => {
            // No logical technique specified, just convert bytes to booleans
            Ok(bytes_to_booleans(values, num_booleans))
        }
    }
}

/// Converts a byte array to a boolean array.
/// Each byte contains up to 8 boolean values (bits).
fn bytes_to_booleans(bytes: &[u8], num_booleans: usize) -> Vec<bool> {
    let mut result = Vec::with_capacity(num_booleans);

    for (byte_index, &byte) in bytes.iter().enumerate() {
        for bit_index in 0..8 {
            let global_index = byte_index * 8 + bit_index;
            if global_index >= num_booleans {
                break;
            }
            result.push((byte & (1 << bit_index)) != 0);
        }
    }

    // Ensure we have exactly the expected number of booleans
    result.truncate(num_booleans);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::stream_encoding::{Logical, Physical};

    #[test]
    fn test_bytes_to_booleans() {
        // Test case: [0b10101010, 0b11001100] with 12 booleans
        let bytes = vec![0b10101010, 0b11001100];
        let result = bytes_to_booleans(&bytes, 12);
        // Expected: [false, true, false, true, false, true, false, true, false, false, true, true]
        assert_eq!(
            result,
            vec![
                false, true, false, true, false, true, false, true, false, false, true, true
            ]
        );
    }

    #[test]
    fn test_bytes_to_booleans_exact_byte_boundary() {
        // Test case: [0b11110000] with 8 booleans
        let bytes = vec![0b11110000];
        let result = bytes_to_booleans(&bytes, 8);
        assert_eq!(
            result,
            vec![false, false, false, false, true, true, true, true]
        );
    }

    #[test]
    fn test_bytes_to_booleans_partial_byte() {
        // Test case: [0b11111111] with 5 booleans
        let bytes = vec![0b11111111];
        let result = bytes_to_booleans(&bytes, 5);
        assert_eq!(result, vec![true, true, true, true, true]);
    }

    #[test]
    fn test_decode_boolean_stream_rle() {
        // Create test data: RLE encoded bytes representing [true, true, true, true, true]
        // RLE encoding: 0x02 (2 runs + 3 = 5 runs), 0xFF (all bits set)
        let mut tile: TrackedBytes = vec![0x02, 0xFF].into();

        let metadata = StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::Rle,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 5,
            byte_length: 2,
            morton: None,
            rle: None,
        };

        let result = decode_boolean_stream(&mut tile, &metadata).unwrap();
        assert_eq!(result, vec![true, true, true, true, true]);
    }

    #[test]
    fn test_decode_boolean_stream_no_compression() {
        // Create test data: [0b10101010] representing [false, true, false, true, false, true, false, true]
        let mut tile: TrackedBytes = vec![0b10101010].into();

        let metadata = StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::None),
            num_values: 8,
            byte_length: 1,
            morton: None,
            rle: None,
        };

        let result = decode_boolean_stream(&mut tile, &metadata).unwrap();
        assert_eq!(
            result,
            vec![false, true, false, true, false, true, false, true]
        );
    }

    #[test]
    fn test_decode_boolean_stream_wrong_physical_type() {
        let mut tile: TrackedBytes = vec![0b10101010].into();

        let metadata = StreamMetadata {
            logical: Logical::new(
                None,
                LogicalLevelTechnique::None,
                LogicalLevelTechnique::None,
            ),
            physical: Physical::new(
                PhysicalStreamType::Data, // Wrong type
                PhysicalLevelTechnique::None,
            ),
            num_values: 8,
            byte_length: 1,
            morton: None,
            rle: None,
        };

        let result = decode_boolean_stream(&mut tile, &metadata);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MltError::InvalidPhysicalStreamType(_)
        ));
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
