use crate::decoding::decoding_utils::DecodingUtils;
use crate::headers::stream::StreamMetadata;

pub struct FloatDecoder {}
impl FloatDecoder {
    pub fn decode_float_stream(data: &[u8], offset: &mut usize, stream_metadata: &StreamMetadata) -> Vec<f32> {
        let values = DecodingUtils::decode_floats_le(data, offset, stream_metadata.num_values as usize);

        Vec::from(&values[0..values.len()])
    }
}
