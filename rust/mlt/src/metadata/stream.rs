use bytes::{Buf, Bytes};

use crate::decoder::varint;
use crate::metadata::stream_encoding::{
    DictionaryType, LengthType, Logical, LogicalLevelTechnique, LogicalStreamType, OffsetType,
    Physical, PhysicalLevelTechnique, PhysicalStreamType,
};
use crate::{MltError, MltResult};

const MORTON: LogicalLevelTechnique = LogicalLevelTechnique::Morton;
const RLE: LogicalLevelTechnique = LogicalLevelTechnique::Rle;

#[derive(Debug, Clone)]
pub struct Rle {
    pub runs: u32,
    pub num_rle_values: u32,
}

#[derive(Debug, Clone)]
pub struct Morton {
    pub num_bits: u32,
    pub coordinate_shift: u32,
}

#[derive(Debug, Clone)]
pub struct StreamMetadata {
    logical: Logical,
    physical: Physical,
    pub num_values: u32,
    pub byte_length: u32,
    morton: Option<Morton>,
    rle: Option<Rle>,
}

impl StreamMetadata {
    pub fn decode(tile: &mut Bytes) -> MltResult<Self> {
        // let stream_type = tile
        //     .get(offset.position() as usize)
        //     .ok_or(MltError::DecodeError("Failed to read...".into()))?;
        let stream_type = tile.get_u8();

        let physical_stream_type = PhysicalStreamType::try_from(stream_type >> 4)
            .map_err(|_| MltError::DecodeError("Invalid physical stream type".into()))?;

        let logical_stream_type = match physical_stream_type {
            PhysicalStreamType::Data => {
                let dict_type = DictionaryType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid dictionary type".into()))?;
                Some(LogicalStreamType::Dictionary(Some(dict_type)))
            }
            PhysicalStreamType::Offset => {
                let offset_type = OffsetType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid offset type".into()))?;
                Some(LogicalStreamType::Offset(offset_type))
            }
            PhysicalStreamType::Length => {
                let length_type = LengthType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid length type".into()))?;
                Some(LogicalStreamType::Length(length_type))
            }
            PhysicalStreamType::Present => None,
        };

        // let encoding_header = *tile
        //     .get(offset.position() as usize)
        //     .ok_or_else(|| MltError::DecodeError("Failed to read encoding header".to_string()))?
        //     & 0xFF;
        let encoding_header = tile.get_u8();

        let logical_level_technique1 = LogicalLevelTechnique::try_from(encoding_header >> 5)
            .map_err(|_| MltError::DecodeError("Invalid logical level technique 1".into()))?;
        let logical_level_technique2 =
            LogicalLevelTechnique::try_from((encoding_header >> 2) & 0x7)
                .map_err(|_| MltError::DecodeError("Invalid logical level technique 2".into()))?;
        let physical_level_technique = PhysicalLevelTechnique::try_from(encoding_header & 0x3)
            .map_err(|_| MltError::DecodeError("Invalid physical level technique".into()))?;

        // offset.increment();

        // let size_info = varint::decode(tile, 2, offset);
        let size_info = varint::decode(tile, 2);
        // new_offset = ???
        let num_values = size_info
            .first()
            .ok_or_else(|| MltError::DecodeError("Failed to read number of values".into()))?;
        let byte_length = size_info
            .get(1)
            .ok_or_else(|| MltError::DecodeError("Failed to read byte length".into()))?;

        let mut metadata = StreamMetadata {
            logical: Logical::new(
                logical_stream_type,
                logical_level_technique1,
                logical_level_technique2,
            ),
            physical: Physical::new(physical_stream_type, physical_level_technique),
            num_values: *num_values,
            byte_length: *byte_length,
            morton: None,
            rle: None,
        };

        if metadata.logical.technique1 == Some(MORTON) {
            metadata.partial_decode(&MORTON, tile)?;
            return Ok(metadata);
        } else if (metadata.logical.technique1 == Some(RLE)
            || metadata.logical.technique2 == Some(RLE))
            && metadata.physical.technique != PhysicalLevelTechnique::None
        {
            metadata.partial_decode(&RLE, tile)?;
            return Ok(metadata);
        }

        Ok(metadata)
    }
}

trait Encoding {
    fn partial_decode(&mut self, r#type: &LogicalLevelTechnique, tile: &mut Bytes)
        -> MltResult<()>;
}

impl Encoding for StreamMetadata {
    fn partial_decode(
        &mut self,
        r#type: &LogicalLevelTechnique,
        tile: &mut Bytes,
    ) -> MltResult<()> {
        // let binding = varint::decode(tile, 2, offset);
        let binding = varint::decode(tile, 2);
        let [val1, val2] = binding.as_slice() else {
            return Err(MltError::DecodeError(
                "Expected 2 values for partial decode".into(),
            ));
        };

        match r#type {
            LogicalLevelTechnique::Morton => {
                self.morton = Some(Morton {
                    num_bits: *val1,
                    coordinate_shift: *val2,
                });
            }
            LogicalLevelTechnique::Rle => {
                self.rle = Some(Rle {
                    runs: *val1,
                    num_rle_values: *val2,
                });
            }
            _ => {
                return Err(MltError::DecodeError(
                    "Invalid logical level technique for partial decode".into(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_decode_stream_metadata() {
        let tile_bytes = vec![
            0x00, // stream_type
            0x60, // encoding_header
            0xF4, // varint byte 1 → part of `num_values` for size_info
            0x02, // varint byte 2 → completes `num_values` for size_info
            0x04, // varint byte 3 → single-byte varint for `byte_length` for size_info
        ];
        let mut tile = Bytes::from(tile_bytes.clone());
        let result = StreamMetadata::decode(&mut tile);
        let metadata = result.unwrap();

        assert_eq!(
            metadata.logical.technique1,
            Some(LogicalLevelTechnique::Rle)
        );
        assert_eq!(
            metadata.logical.technique2,
            Some(LogicalLevelTechnique::None)
        );
        assert_eq!(metadata.physical.r#type, PhysicalStreamType::Present);
        assert_eq!(metadata.physical.technique, PhysicalLevelTechnique::None);
        assert_eq!(metadata.num_values, 372);
        assert_eq!(metadata.byte_length, 4);
    }
}
