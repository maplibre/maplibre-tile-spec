use crate::decoding::decoding_utils::DecodingUtils;
use crate::headers::stream::StreamMetadata;

pub struct DoubleDecoder {}
impl DoubleDecoder {
    pub fn decode_double_stream(data: &[u8], offset: &mut usize, stream_metadata: &StreamMetadata) -> Vec<f64> {
        DecodingUtils::decode_doubles_le(data, offset, stream_metadata.num_values as usize)
    }
}
