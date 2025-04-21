use crate::{MltError, MltResult};
use bitvec::prelude::*;
use bytes::{Buf, Bytes};
use fastpfor::rust::IncrementCursor;
use parquet::data_type::BoolType;
use parquet::decoding::Decoder;
use parquet::{data_type::ByteArrayType, decoding::RleValueDecoder};
use std::io::Cursor;

// fn test() {
//     let mut decoder = RleValueDecoder::<ByteArrayType>::new();
//     decoder.set_data(&[0, 1, 2, 3, 4, 5]);
// }

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
    // let result = decoder.get(???)

    // let byte_stream = decode_byte_rle(buffer, num_bytes as usize, byte_size as usize)?;
    // let bits = BitVec::<u8, Lsb0>::from_vec(byte_stream);

    // Ok(bits)
    todo!("Implement decode_boolean_rle");
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
