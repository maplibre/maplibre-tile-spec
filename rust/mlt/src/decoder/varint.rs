use std::io::Cursor;

pub fn decode(src: &[u8], pos: &mut Cursor<u32>, num_values: usize) -> Vec<u32> {
    let mut values = vec![0; num_values];
    let mut dst_offset = 0;
    for _ in 0..num_values {
        let offset = decode_internal(src, pos.position() as i32, &mut values, dst_offset);
        dst_offset += 1;
        pos.set_position(offset as u64);
    }
    values
}

fn decode_internal(src: &[u8], offset: i32, dst: &mut [u32], dst_offset: usize) -> i32 {
    let mut offset = offset;

    // Max 4 bytes supported.
    let mut b = src[offset as usize];
    offset += 1;

    let mut value = (b & 0x7f) as u32;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset as usize];
    offset += 1;
    value |= ((b & 0x7f) as u32) << 7;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset as usize];
    offset += 1;
    value |= ((b & 0x7f) as u32) << 14;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset as usize];
    offset += 1;
    value |= ((b & 0x7f) as u32) << 21;
    dst[dst_offset] = value;
    offset
}
