use std::io::Cursor;

pub fn decode_varint(src: &[u8], pos: &mut Cursor<u32>, num_values: usize) -> Vec<i32> {
    let mut values = vec![0; num_values];
    let mut dst_offset = 0;
    for _ in 0..num_values {
        let offset = decode_varint_internal(src, pos.position() as usize, &mut values, dst_offset);
        dst_offset += 1;
        pos.set_position(offset as u64);
    }
}

fn decode_varint_internal(src: &[u8], offset: usize, dst: &mut [i32], dst_offset: usize) -> usize {
    // Max 4 bytes supported.
    let mut offset = offset;
    let mut b = src[offset];
    offset += 1;

    let mut value = (b & 0x7f) as i32;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset];
    offset += 1;
    value |= ((b & 0x7f) as i32) << 7;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset];
    offset += 1;
    value |= ((b & 0x7f) as i32) << 14;
    if (b & 0x80) == 0 {
        dst[dst_offset] = value;
        return offset;
    }

    b = src[offset];
    offset += 1;
    value |= ((b & 0x7f) as i32) << 21;
    dst[dst_offset] = value;
    offset
}
