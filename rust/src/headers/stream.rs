mod morton;
mod rle;

use crate::decoding::decoding_utils::DecodingUtils;
use crate::types::compression_types::{LogicalLevelCompressionTechnique, PhysicalLevelCompressionTechnique};
use crate::types::stream_types::{LogicalStreamType, PhysicalStreamType};
pub use morton::*;
pub use rle::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HasStreamMetadata {
    pub stream_metadata: StreamMetadata,
    pub morton_stream_metadata: Option<MortonEncodedStreamMetadata>,
    pub rle_stream_metadata: Option<RLEEncodedStreamMetadata>,
}
impl HasStreamMetadata {
    pub fn new_stream_metadata(stream_metadata: StreamMetadata) -> Self {
        Self {
            stream_metadata,
            morton_stream_metadata: None,
            rle_stream_metadata: None,
        }
    }

    pub fn new_morton_metadata(stream_metadata: StreamMetadata, morton_stream_metadata: MortonEncodedStreamMetadata) -> Self {
        Self {
            stream_metadata,
            morton_stream_metadata: Some(morton_stream_metadata),
            rle_stream_metadata: None,
        }
    }

    pub fn new_rle_metadata(stream_metadata: StreamMetadata, rle_stream_metadata: RLEEncodedStreamMetadata) -> Self {
        Self {
            stream_metadata,
            morton_stream_metadata: None,
            rle_stream_metadata: Some(rle_stream_metadata),
        }
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
// Size = 10
pub struct StreamMetadata {
    pub physical_stream_type: PhysicalStreamType,
    pub logical_stream_type: LogicalStreamType,
    pub logical_level_technique1: LogicalLevelCompressionTechnique,
    pub logical_level_technique2: LogicalLevelCompressionTechnique,
    pub physical_level_technique: PhysicalLevelCompressionTechnique,
    pub num_values: u32,
    pub byte_length: u32,
}
impl StreamMetadata {
    pub fn decode(tile: &[u8], offset: &mut usize) -> StreamMetadata {
        let stream_type = tile[*offset];
        let physical_stream_type = (stream_type >> 4).into();
        
        let logical_stream_type = match physical_stream_type {
            PhysicalStreamType::DATA => LogicalStreamType::new_dictionary((stream_type & 0xF).into()),
            PhysicalStreamType::OFFSET => LogicalStreamType::new_offset((stream_type & 0xF).into()),
            PhysicalStreamType::LENGTH => LogicalStreamType::new_length((stream_type & 0xF).into()),
            PhysicalStreamType::PRESENT => LogicalStreamType::new_none(),
        };

        *offset += 1;

        let encodings_header = tile[*offset] & 0xFF;
        let logical_level_technique1 = (encodings_header >> 5).into();
        let logical_level_technique2 = (encodings_header >> 2 & 0x7).into();
        let physical_level_technique = (encodings_header & 0x3).into();
        *offset += 1;

        let size_info = DecodingUtils::decode_varint(tile, offset, 2);
        let num_values = size_info[0];
        let byte_length = size_info[1];

        StreamMetadata {
            physical_stream_type,
            logical_stream_type,
            logical_level_technique1,
            logical_level_technique2,
            physical_level_technique,
            num_values,
            byte_length,
        }
    }
}


// impl Stream {
//     pub fn load(data: &[u8], offset: &mut usize) -> Self {
//
//         let ps_type = PhysicalStreamType::from(data[*offset] >> 4);
//         let ls_type_raw = data[*offset] & 0x0F;
//         let ls_type = match ps_type {
//             PhysicalStreamType::PRESENT => { StreamType::new_none() }
//             PhysicalStreamType::DATA => { StreamType::new_dictionary(StreamType_Dictionary::from(ls_type_raw)) }
//             PhysicalStreamType::OFFSET => { StreamType::new_offset(StreamType_OffsetIntoDictionary::from(ls_type_raw)) }
//             PhysicalStreamType::LENGTH => { StreamType::new_length(StreamType_VariableSizedItems::from(ls_type_raw)) }
//         };
//         *offset += 1;
//
//         let llt1 = LogicalLevelCompressionTechnique::from(data[*offset] >> 5);
//         let llt2 = LogicalLevelCompressionTechnique::from((data[*offset] >> 2) & 0b00000111);
//         let plt = PhysicalLevelCompressionTechnique::from(data[*offset] & 0b00000011);
//         *offset += 1;
//
//         let mut infos = [0; 2];
//         *offset += VarInt::decode_varint(&data[1..], 2, &mut infos);
//
//         let mut result = Self {
//             physical_stream_type: ps_type,
//             logical_stream_type: ls_type,
//             logical_level_technique1: llt1,
//             logical_level_technique2: llt2,
//             physical_level_technique: plt,
//             num_values: infos[0],
//             byte_length: infos[1],
//             data: Vec::new(),
//         };
//
//         result.data.copy_from_slice(&data[*offset..*offset + result.byte_length as usize]);
//
//         *offset += result.byte_length as usize;
//
//         result
//     }
// }