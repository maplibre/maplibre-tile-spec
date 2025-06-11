use crate::decoder::tracked_bytes::TrackedBytes;
use bytes_varint::*;

pub fn decode(input: &mut TrackedBytes, num_values: usize) -> Vec<u32> {
    let mut values = Vec::with_capacity(num_values);

    for _ in 0..num_values {
        let val = input.try_get_u32_varint().expect("Failed to decode varint");
        values.push(val);
    }

    values
}
