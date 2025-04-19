use crate::{MltError, MltResult};
use bitvec::prelude::*;
use bytes::Bytes;
use fastpfor::rust::IncrementCursor;
use std::io::Cursor;

#[expect(dead_code, unused_variables)]
pub fn decode_boolean_rle(
    buffer: &Bytes,
    num_booleans: u32,
    byte_size: u32,
    pos: &mut Cursor<u32>,
) -> MltResult<BitVec> {
    let num_bytes = num_booleans.div_ceil(8);
    let byte_stream = decode_byte_rle(buffer, num_bytes as usize, byte_size as usize, pos)?;
    // Java: BitSet.valueOf(byteStream);
    todo!("Implement decode_boolean_rle");
}

pub fn decode_byte_rle(
    buffer: &Bytes,
    num_bytes: usize,
    byte_size: usize,
    pos: &mut Cursor<u32>,
) -> MltResult<Vec<u8>> {
    let mut values = Vec::with_capacity(num_bytes);

    // This emulates the inStream starting from pos.get()
    let mut read_pos = pos.position() as usize;

    // Very basic RLE: assume the reader.next() returns a single byte at a time
    // Here we just read `num_bytes` bytes sequentially from the stream starting at `pos`
    for _ in 0..num_bytes {
        let byte = buffer
            .get(read_pos)
            .ok_or_else(|| MltError::DecodeError("Failed to read byte from buffer".to_string()))?;
        values.push(*byte);
        read_pos += 1;
    }

    pos.add(byte_size as u32);

    Ok(values)
}

// #[expect(unused_variables, unused_mut)]
// fn decode_byte_rle(
//     buffer: &Bytes,
//     num_bytes: u32,
//     byte_size: u32,
//     pos: &mut Cursor<u32>,
// ) -> MltResult<Vec<u8>> {
//
//     let mut remaining_count = 0;
//     let mut current_value: Option<u8> = None;
//
//     let mut values = vec![0; num_bytes as usize];
//     for i in 0..num_bytes {
//         if remaining_count == 0 {
//             let count = buffer[pos.position() as usize];
//             pos.set_position(pos.position() + 1);
//             remaining_count = count & 0x7F;
//             current_value = Some(count >> 7);
//         }
//
//         if let Some(value) = current_value {
//             values[i as usize] = value;
//         }
//
//         remaining_count -= 1;
//     }
//
//
//
//     todo!("Implement decode_byte_rle");
// }

// public static byte[] decodeByteRle(byte[] buffer, int numBytes, int byteSize, IntWrapper pos)
//     throws IOException {
//   var inStream =
//       InStream.create(
//           "test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
//   var reader = new RunLengthByteReader(inStream);
//
//   var values = new byte[numBytes];
//   for (var i = 0; i < numBytes; i++) {
//     values[i] = reader.next();
//   }
//
//   pos.add(byteSize);
//   return values;
// }
