use crate::decoding::decoding_utils::DecodingUtils;
use crate::headers::stream::StreamMetadata;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MortonEncodedStreamMetadata {
    pub base: StreamMetadata,
    pub num_bits: u32,
    pub coordinate_shift: u32,
}
impl MortonEncodedStreamMetadata {
    pub fn decode(data: &[u8], offset: &mut usize) -> Self {
        let stream_metadata = StreamMetadata::decode(data, offset);
        let morton_info = DecodingUtils::decode_varint(data, offset, 2);
        Self {
            base: stream_metadata,
            num_bits: morton_info[0],
            coordinate_shift: morton_info[1],
        }
    }
    pub fn decode_partial(stream_metadata: StreamMetadata, data: &[u8], offset: &mut usize) -> Self {
        let morton_info = DecodingUtils::decode_varint(data, offset, 2);
        Self {
            base: stream_metadata,
            num_bits: morton_info[0],
            coordinate_shift: morton_info[1],
        }
    }
}
