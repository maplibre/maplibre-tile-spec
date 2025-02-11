use crate::decoding::decoding_utils::DecodingUtils;
use crate::headers::stream::StreamMetadata;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RLEEncodedStreamMetadata {
    pub base: StreamMetadata,
    pub run_count: u32,
    pub num_rle_values: u32,
}
impl RLEEncodedStreamMetadata {
    pub fn decode(data: &[u8], offset: &mut usize) -> Self {
        let stream_metadata = StreamMetadata::decode(data, offset);
        let rle_metadata = DecodingUtils::decode_varint(data, offset, 2);
        Self {
            base: stream_metadata,
            run_count: rle_metadata[0],
            num_rle_values: rle_metadata[1],
        }
    }
    pub fn decode_partial(stream_metadata: StreamMetadata, data: &[u8], offset: &mut usize) -> Self {
        let rle_metadata = DecodingUtils::decode_varint(data, offset, 2);
        Self {
            base: stream_metadata,
            run_count: rle_metadata[0],
            num_rle_values: rle_metadata[1],
        }
    }
}

