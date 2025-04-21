use bitvec::prelude::*;
use bytes::Bytes;
use parquet::data_type::{BoolType, ByteArrayType};
use parquet::decoding::{Decoder, RleValueDecoder};

use crate::{MltError, MltResult};

pub fn decode_boolean_rle(
    buffer: &Bytes,
    num_booleans: u32,
    #[expect(unused_variables)] byte_size: u32,
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

#[expect(unused_variables)]
pub fn decode_byte_rle(buffer: &Bytes, num_bytes: usize, byte_size: usize) -> MltResult<Vec<u8>> {
    let mut reader = RleValueDecoder::<ByteArrayType>::new();
    reader
        .set_data(buffer.clone(), num_bytes)
        .map_err(|_| MltError::DecodeError("Failed to set data for RLE decoder".to_string()))?;

    for _ in 0..num_bytes {
        // let byte = buffer.get_u8();
        // values.push(byte);

        // let byte = buffer
        //     .get(read_pos)
        //     .ok_or_else(|| MltError::DecodeError("Failed to read byte from buffer".to_string()))?;
        // values.push(*byte);
        // read_pos += 1;
    }

    // pos.add(byte_size as u32);

    // Ok(values)
    todo!("Implement decode_byte_rle");
}
