use std::io::Cursor;

use bytes::Bytes;
use bytes_varint::*;

pub fn decode(input: &Bytes, num_values: usize, offset: &mut Cursor<u32>) -> Vec<u32> {
    let mut values = Vec::with_capacity(num_values);

    for _ in 0..num_values {
        let pos = offset.position() as usize;
        let reader = &input[pos..];

        // Decode using a Cursor over the slice to track how many bytes were consumed
        let mut internal_offset = Cursor::new(reader);
        let val = internal_offset
            .try_get_u32_varint()
            .expect("Failed to decode varint");
        values.push(val);

        // Advance the shared cursor by however many bytes we just read
        let consumed = internal_offset.position();
        offset.set_position(offset.position() + consumed);
    }

    values
}
