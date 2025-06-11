use bytes::Buf;

use crate::decoder::tracked_bytes::TrackedBytes;
#[allow(unused_imports)]
use crate::metadata::proto_tileset::{column, scalar_column, Column, ScalarColumn, ScalarType};
use crate::{MltError, MltResult};

/// Decodes boolean RLE from the buffer.
/// - `num_booleans` is the total number of booleans (bits).
/// - `byte_size` is inferred as `ceil(num_booleans / 8)`.
pub fn decode_boolean_rle(tile: &mut TrackedBytes, num_booleans: usize) -> MltResult<Vec<u8>> {
    let num_bytes = num_booleans.div_ceil(8);
    decode_byte_rle(tile, num_bytes)
}

/// Decodes byte RLE from the buffer.
/// - `num_bytes` is how many decoded bytes we expect.
pub fn decode_byte_rle(tile: &mut TrackedBytes, num_bytes: usize) -> MltResult<Vec<u8>> {
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

    Ok(result)
}

/// Get the physical scalarType from a Column metadata.
pub fn get_data_type_from_column(column_metadata: &Column) -> MltResult<ScalarType> {
    match column_metadata.r#type.as_ref() {
        Some(column::Type::ScalarType(scalar_column)) => match scalar_column.r#type {
            Some(scalar_column::Type::PhysicalType(scalar_type)) => {
                ScalarType::try_from(scalar_type)
                    .map_err(|_| MltError::DecodeError("Invalid scalar type value".to_string()))
            }
            _ => Err(MltError::DecodeError(
                "Missing or unsupported scalar type".to_string(),
            )),
        },
        _ => Err(MltError::DecodeError("Missing column type".to_string())),
    }
}

#[test]
fn test_decode_byte_rle() -> MltResult<()> {
    let mut tile: TrackedBytes = [0x03, 0x01].as_slice().into();
    let result = decode_byte_rle(&mut tile, 5)?;
    assert_eq!(result, vec![1, 1, 1, 1, 1]);
    Ok(())
}

#[test]
fn test_decode_boolean_rle() -> MltResult<()> {
    let mut tile: TrackedBytes = [0x03, 0x01].as_slice().into();
    let result = decode_boolean_rle(&mut tile, 5)?;
    assert_eq!(result, vec![1]);
    Ok(())
}

#[test]
fn test_get_data_type_from_column() {
    let column_metadata = Column {
        name: "id".to_string(),
        nullable: false,
        column_scope: 0,
        r#type: Some(column::Type::ScalarType(ScalarColumn {
            r#type: Some(scalar_column::Type::PhysicalType(ScalarType::Uint32 as i32)),
        })),
    };
    let data_type = get_data_type_from_column(&column_metadata).expect("should parse ScalarType");
    assert_eq!(data_type, ScalarType::Uint32);
}
