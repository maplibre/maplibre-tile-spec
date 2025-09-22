use bytes::Buf;

use crate::decoder::tracked_bytes::TrackedBytes;
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

impl Rle {
    pub fn decode_partial(tile: &mut TrackedBytes) -> MltResult<Self> {
        let rle_info = varint::decode(tile, 2)?;
        Ok(Rle {
            runs: rle_info[0],
            num_rle_values: rle_info[1],
        })
    }
}

#[derive(Debug, Clone)]
pub struct Morton {
    pub num_bits: u32,
    pub coordinate_shift: u32,
}

impl Morton {
    pub fn decode_partial(tile: &mut TrackedBytes) -> MltResult<Self> {
        let morton_info = varint::decode(tile, 2)?;
        Ok(Morton {
            num_bits: morton_info[0],
            coordinate_shift: morton_info[1],
        })
    }
}

#[derive(Debug, Clone)]
pub struct StreamMetadata {
    pub logical: Logical,
    pub physical: Physical,
    pub num_values: u32,
    pub byte_length: u32,
    pub morton: Option<Morton>,
    pub rle: Option<Rle>,
}

impl StreamMetadata {
    pub fn decode(tile: &mut TrackedBytes) -> MltResult<Self> {
        let stream_type = tile.get_u8();
        let physical_code = stream_type >> 4;
        let subtype_code = stream_type & 0x0F;

        let physical_stream_type = PhysicalStreamType::try_from(physical_code)
            .map_err(|_| MltError::InvalidPhysicalStreamType(physical_code))?;

        let logical_stream_type = match physical_stream_type {
            PhysicalStreamType::Data => {
                let dict_type = DictionaryType::try_from(subtype_code)
                    .map_err(|_| MltError::InvalidDictionaryType(subtype_code))?;
                Some(LogicalStreamType::Dictionary(Some(dict_type)))
            }
            PhysicalStreamType::Offset => {
                let offset_type = OffsetType::try_from(subtype_code)
                    .map_err(|_| MltError::InvalidOffsetType(subtype_code))?;
                Some(LogicalStreamType::Offset(offset_type))
            }
            PhysicalStreamType::Length => {
                let length_type = LengthType::try_from(subtype_code)
                    .map_err(|_| MltError::InvalidLengthType(subtype_code))?;
                Some(LogicalStreamType::Length(length_type))
            }
            PhysicalStreamType::Present => None,
        };

        let encoding_header = tile.get_u8();

        let llt1_code = encoding_header >> 5;
        let logical_level_technique1 = LogicalLevelTechnique::try_from(llt1_code)
            .map_err(|_| MltError::InvalidLogicalLevelTechnique(llt1_code))?;

        let llt2_code = (encoding_header >> 2) & 0x07;
        let logical_level_technique2 = LogicalLevelTechnique::try_from(llt2_code)
            .map_err(|_| MltError::InvalidLogicalLevelTechnique(llt2_code))?;

        let plt_code = encoding_header & 0x3;
        let physical_level_technique = PhysicalLevelTechnique::try_from(plt_code)
            .map_err(|_| MltError::InvalidPhysicalLevelTechnique(plt_code))?;

        let size_info = varint::decode(tile, 2)?;

        if size_info.len() != 2 {
            return Err(MltError::ExpectedValues {
                ctx: "StreamMetadata::decode.size_info",
                expected: 2,
                got: size_info.len(),
            });
        }
        let num_values = size_info[0];
        let byte_length = size_info[1];

        let mut metadata = StreamMetadata {
            logical: Logical::new(
                logical_stream_type,
                logical_level_technique1,
                logical_level_technique2,
            ),
            physical: Physical::new(physical_stream_type, physical_level_technique),
            num_values,
            byte_length,
            morton: None,
            rle: None,
        };

        if metadata.logical.technique1 == Some(MORTON) {
            metadata.partial_decode(MORTON, tile)?;
            return Ok(metadata);
        } else if (metadata.logical.technique1 == Some(RLE)
            || metadata.logical.technique2 == Some(RLE))
            && metadata.physical.technique != PhysicalLevelTechnique::None
        {
            metadata.partial_decode(RLE, tile)?;
            return Ok(metadata);
        }

        // Decode any remaining metadata
        if metadata.logical.technique1 == Some(MORTON) {
            let morton = Morton::decode_partial(tile)?;
            metadata.morton = Some(morton);
        } else if (metadata.logical.technique1 == Some(RLE)
            || metadata.logical.technique2 == Some(RLE))
            && metadata.physical.technique != PhysicalLevelTechnique::None
        {
            let rle = Rle::decode_partial(tile)?;
            metadata.rle = Some(rle);
        }

        Ok(metadata)
    }
}

trait Encoding {
    fn partial_decode(
        &mut self,
        r#type: LogicalLevelTechnique,
        tile: &mut TrackedBytes,
    ) -> MltResult<()>;
}

impl Encoding for StreamMetadata {
    fn partial_decode(
        &mut self,
        r#type: LogicalLevelTechnique,
        tile: &mut TrackedBytes,
    ) -> MltResult<()> {
        // let binding = varint::decode(tile, 2, offset);
        let vals = varint::decode(tile, 2)?;
        if vals.len() != 2 {
            return Err(MltError::ExpectedValues {
                ctx: "StreamMetadata::partial_decode",
                expected: 2,
                got: vals.len(),
            });
        }

        match r#type {
            LogicalLevelTechnique::Morton => {
                self.morton = Some(Morton {
                    num_bits: vals[0],
                    coordinate_shift: vals[1],
                });
            }
            LogicalLevelTechnique::Rle => {
                self.rle = Some(Rle {
                    runs: vals[0],
                    num_rle_values: vals[1],
                });
            }
            other => {
                return Err(MltError::PartialDecodeWrongTechnique(other));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_plain_stream_metadata() {
        let mut tile: TrackedBytes = [
            0x00, // stream_type
            0x60, // encoding_header
            0xF4, // varint byte 1 → part of `num_values` for size_info
            0x02, // varint byte 2 → completes `num_values` for size_info
            0x04, // varint byte 3 → single-byte varint for `byte_length` for size_info
        ]
        .as_slice()
        .into();
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

    #[test]
    fn test_decode_morton_stream_metadata() {
        let mut tile: TrackedBytes = [
            0x00, // stream_type: PhysicalStreamType::Present
            0x80, // encoding_header: technique1 = Morton, technique2 = None, physical_level = None
            0xF4, 0x02, // num_values = 372 (varint)
            0x04, // byte_length = 4 (varint)
            0x01, // Morton num_bits = 1
            0x10, // Morton coordinate_shift = 16
        ]
        .as_slice()
        .into();

        let metadata = StreamMetadata::decode(&mut tile).unwrap();

        assert_eq!(metadata.physical.r#type, PhysicalStreamType::Present);
        assert_eq!(metadata.physical.technique, PhysicalLevelTechnique::None);
        assert_eq!(
            metadata.logical.technique1,
            Some(LogicalLevelTechnique::Morton)
        );
        assert_eq!(
            metadata.logical.technique2,
            Some(LogicalLevelTechnique::None)
        );
        assert_eq!(metadata.num_values, 372);
        assert_eq!(metadata.byte_length, 4);

        let morton = metadata
            .morton
            .expect("Morton metadata should be populated");
        assert_eq!(morton.num_bits, 1);
        assert_eq!(morton.coordinate_shift, 16);
        assert!(metadata.rle.is_none());
    }

    #[test]
    fn test_decode_rle_stream_metadata() {
        let mut tile: TrackedBytes = [
            0x00, // stream_type: PhysicalStreamType::Present
            0x61, // encoding_header: technique1 = RLE, technique2 = None, physical_level = FastPfor
            0xF4, 0x02, // num_values = 372 (varint)
            0x04, // byte_length = 4 (varint)
            0x01, // RLE runs = 1
            0xF4, 0x02, // RLE num_rle_values = 372 (varint)
        ]
        .as_slice()
        .into();

        let metadata = StreamMetadata::decode(&mut tile).unwrap();

        assert_eq!(metadata.physical.r#type, PhysicalStreamType::Present);
        assert_eq!(
            metadata.physical.technique,
            PhysicalLevelTechnique::FastPfor // Must have any technique apart from None
        );
        assert_eq!(
            metadata.logical.technique1,
            Some(LogicalLevelTechnique::Rle)
        );
        assert_eq!(
            metadata.logical.technique2,
            Some(LogicalLevelTechnique::None)
        );
        assert_eq!(metadata.num_values, 372);
        assert_eq!(metadata.byte_length, 4);

        let rle = metadata.rle.expect("RLE metadata should be populated");
        assert_eq!(rle.runs, 1);
        assert_eq!(rle.num_rle_values, 372);
        assert!(metadata.morton.is_none());
    }
}
