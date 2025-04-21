use std::io::Cursor;

use bytes::Bytes;
use bytes_varint::*;

pub fn decode(input: &mut Bytes, num_values: usize) -> Vec<u32> {
    let mut values = Vec::with_capacity(num_values);

    for _ in 0..num_values {
        let val = input.try_get_u32_varint().expect("Failed to decode varint");
        values.push(val);
    }

    values
}
