use serde::{Deserialize, Serialize};

use crate::decoder::boolean::{bytes_to_booleans, decode_boolean_stream};
use crate::decoder::integer::decode_int_stream;
use crate::decoder::tracked_bytes::TrackedBytes;
use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::PhysicalStreamType;
use crate::{MltError, MltResult};

#[derive(Serialize, Deserialize)]
pub enum StreamValue {
    Boolean(Vec<bool>),
    Integer(Vec<i32>),
    Data(Vec<u8>),
}

/// Decodes a stream based on its metadata.
pub fn decode_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
    is_signed: bool,
) -> MltResult<StreamValue> {
    match metadata.physical.r#type {
        PhysicalStreamType::Present => {
            let bytes = decode_boolean_stream(tile, metadata)?;
            Ok(StreamValue::Boolean(bytes_to_booleans(
                &bytes,
                metadata.num_values as usize,
            )))
        }
        PhysicalStreamType::Length | PhysicalStreamType::Offset => {
            decode_int_stream(tile, metadata, is_signed).map(StreamValue::Integer)
        }
        PhysicalStreamType::Data => Err(MltError::InvalidPhysicalStreamType(
            PhysicalStreamType::Data as u8,
        )),
    }
}
