use crate::{MltError, MltResult};
use num_enum::TryFromPrimitive;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub enum LogicalStreamType {
    Dictionary(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum LengthType {
    VarBinary,
    Geometries,
    Parts,
    Rings,
    Triangles,
    Symbol,
    Dictionary,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum DictionaryType {
    None,
    Single,
    Shared,
    Vertex,
    Morton,
    Fsst,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex,
    Index,
    String,
    Key,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalStreamType {
    Present,
    Data,
    Offset,
    Length,
}

#[derive(Debug, Clone)]
pub enum LogicalLevelTechnique {
    None,
    Delta,
    ComponentwiseDelta,
    Rle,
    Morton,
    // Pseudodecimal Encoding of floats -> only for the exponent integer part an additional logical level technique is used.
    // Both exponent and significant parts are encoded with the same physical level technique
    Pde,
}

#[derive(Debug, Clone)]
pub enum PhysicalLevelTechnique {
    None,
    FastPfor,
    Varint,
    Alp,
}

#[derive(Debug, Clone)]
pub struct StreamMetadata {
    physical_stream_type: PhysicalStreamType,
    logical_stream_type: LogicalStreamType,
    logical_level_technique1: LogicalLevelTechnique,
    logical_level_technique2: LogicalLevelTechnique,
    physical_level_technique: PhysicalLevelTechnique,
    num_values: usize,
    byte_length: usize,
}

#[expect(dead_code, unused_variables)]
impl StreamMetadata {
    pub fn decode(tile: &[u8], offset: &mut Cursor<usize>) -> MltResult<Self> {
        let stream_type = tile
            .get(offset.position() as usize)
            .ok_or_else(|| MltError::DecodeError("Failed to read stream type".to_string()))?;

        let physical_stream_type = PhysicalStreamType::try_from(stream_type >> 4)
            .map_err(|_| MltError::DecodeError(format!(
                "Invalid physical stream type: {}", stream_type >> 4
            )))?;

        let logical_stream_type = match physical_stream_type {
            PhysicalStreamType::Data => {
                LogicalStreamType::Dictionary(
                    DictionaryType::try_from(stream_type & 0xF)
                        .map_err(|_| MltError::DecodeError("Invalid dictionary type".into()))?,
                )
            }
            PhysicalStreamType::Offset => {
                LogicalStreamType::Offset(OffsetType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid offset type".into()))?)
            }
            PhysicalStreamType::Length => {
                LogicalStreamType::Length(LengthType::try_from(stream_type & 0xF)
                    .map_err(|_| MltError::DecodeError("Invalid length type".into()))?)
            }
            _ => {
                return Err(MltError::DecodeError(
                    "Unexpected physical stream type for logical stream type".into(),
                ));
            }
        };

        todo!("Implement the decoding logic for StreamMetadata");
    }
}
