use bytes::{Buf, Bytes};

use crate::MltResult;

/// Decodes boolean RLE from the buffer.
/// - `num_booleans` is the total number of booleans (bits).
/// - `byte_size` is inferred as `ceil(num_booleans / 8)`.
pub fn decode_boolean_rle(tile: &mut Bytes, num_booleans: usize) -> MltResult<Vec<u8>> {
    let num_bytes = num_booleans.div_ceil(8);
    decode_byte_rle(tile, num_bytes)
}

/// Decodes byte RLE from the buffer.
/// - `num_bytes` is how many decoded bytes we expect.
pub fn decode_byte_rle(tile: &mut Bytes, num_bytes: usize) -> MltResult<Vec<u8>> {
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

#[test]
fn test_decode_byte_rle() -> MltResult<()> {
    let mut tile = Bytes::from_static(&[0x03, 0x01]);
    let result = decode_byte_rle(&mut tile, 5)?;
    assert_eq!(result, vec![1, 1, 1, 1, 1]);
    Ok(())
}

#[test]
fn test_decode_boolean_rle() -> MltResult<()> {
    let mut tile = Bytes::from_static(&[0x03, 0x01]);
    let result = decode_boolean_rle(&mut tile, 5)?;
    assert_eq!(result, vec![1]);
    Ok(())
}
