use std::io::Cursor;

use bitvec::prelude::*;
use bytes::{Buf, Bytes};
use bytes::Bytes;
use fastpfor::rust::IncrementCursor;
use parquet::data_type::BoolType;
use parquet::decoding::Decoder;
use parquet::{data_type::ByteArrayType, decoding::RleValueDecoder};
use std::io::Cursor;

// fn test() {
//     let mut decoder = RleValueDecoder::<ByteArrayType>::new();
//     decoder.set_data(&[0, 1, 2, 3, 4, 5]);
// }

use crate::{MltError, MltResult};

#[expect(dead_code, unused_variables)]
pub fn decode_boolean_rle(
    buffer: &Bytes,
    num_booleans: u32,
    byte_size: u32,
) -> MltResult<BitVec<u8, Lsb0>> {
    let num_bytes = (num_booleans + 7) / 8;

    let mut decoder = RleValueDecoder::<BoolType>::new();
    decoder
        .set_data(buffer.clone(), num_bytes as usize)
        .map_err(|_| MltError::DecodeError("Failed to set data for RLE decoder".to_string()))?;

    let mut bools = vec![false; num_booleans as usize];
    let decoded = decoder
        .get(&mut bools)
        .map_err(|_| MltError::DecodeError("Failed to decode boolean RLE data".to_string()))?;

    // Convert Vec<bool> to BitVec
    let bitvec: BitVec<u8, Lsb0> = bools[..decoded].iter().copied().collect();
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

    Ok(bitvec)
}

// pub fn decode_byte_rle(buffer: &Bytes, num_bytes: usize, byte_size: usize) -> MltResult<Vec<u8>> {
//     let mut reader = RleValueDecoder::<ByteArrayType>::new();
//     reader
//         .set_data(buffer.clone(), num_bytes)
//         .map_err(|_| MltError::DecodeError("Failed to set data for RLE decoder".to_string()))?;
//
//     for _ in 0..num_bytes {
//         // let byte = buffer.get_u8();
//         // values.push(byte);
//
//         // let byte = buffer
//         //     .get(read_pos)
//         //     .ok_or_else(|| MltError::DecodeError("Failed to read byte from buffer".to_string()))?;
//         // values.push(*byte);
//         // read_pos += 1;
//     }
//
//     // pos.add(byte_size as u32);
//
//     // Ok(values)
//     todo!("Implement decode_byte_rle");
// }
