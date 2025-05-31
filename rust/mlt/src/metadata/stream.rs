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
            PhysicalStreamType::Data => LogicalStreamType::Dictionary(Some(
                DictionaryType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid dictionary type".into()))?,
            )),
            PhysicalStreamType::Offset => LogicalStreamType::Offset(
                OffsetType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid offset type".into()))?,
            ),
            PhysicalStreamType::Length => LogicalStreamType::Length(
                LengthType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid length type".into()))?,
            ),
            _ => {
                return Err(MltError::DecodeError(
                    "Unexpected physical stream type for logical stream type".into(),
                ));
            }
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
            && metadata.physical.technique.is_some()
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
