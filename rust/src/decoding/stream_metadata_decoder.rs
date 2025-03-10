use crate::headers::stream::{HasStreamMetadata, MortonEncodedStreamMetadata, RLEEncodedStreamMetadata, StreamMetadata};
use crate::types::compression_types::{LogicalLevelCompressionTechnique, PhysicalLevelCompressionTechnique};

pub struct StreamMetadataDecoder {}
impl StreamMetadataDecoder {
    pub fn decode(data: &[u8], offset: &mut usize) -> HasStreamMetadata {
        let stream_metadata = StreamMetadata::decode(data, offset);

        if stream_metadata.logical_level_technique1 == LogicalLevelCompressionTechnique::MORTON {
            let morton_metadata = MortonEncodedStreamMetadata::decode_partial(stream_metadata, data, offset);
            return HasStreamMetadata::new_morton_metadata(stream_metadata, morton_metadata);
        } else if (stream_metadata.logical_level_technique1 == LogicalLevelCompressionTechnique::RLE
                || stream_metadata.logical_level_technique2 == LogicalLevelCompressionTechnique::RLE)
                && stream_metadata.physical_level_technique != PhysicalLevelCompressionTechnique::NONE {
            let rle_metadata = RLEEncodedStreamMetadata::decode_partial(stream_metadata, data, offset);
            return HasStreamMetadata::new_rle_metadata(stream_metadata, rle_metadata);
        }

        HasStreamMetadata::new_stream_metadata(stream_metadata)
    }
}
