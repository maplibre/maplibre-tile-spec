use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::utils::{BinarySerializer as _, checked_sum3};
use crate::v01::{ColumnType, EncodedProperty, PropertyKind, StagedProperty};
use crate::{MltError, MltResult};

impl StagedProperty {
    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        use PropertyKind as T;
        match self {
            Self::Bool(_) => T::Bool,
            Self::I8(_)
            | Self::I32(_)
            | Self::I64(_)
            | Self::U8(_)
            | Self::U32(_)
            | Self::U64(_) => T::Integer,
            Self::F32(_) | Self::F64(_) => T::Float,
            Self::Str(_) => T::String,
            Self::SharedDict(_) => T::SharedDict,
        }
    }
}

impl EncodedProperty {
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        let col_type = match self {
            Self::Bool(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptBool
                } else {
                    ColumnType::Bool
                }
            }
            Self::I8(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptI8
                } else {
                    ColumnType::I8
                }
            }
            Self::U8(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptU8
                } else {
                    ColumnType::U8
                }
            }
            Self::I32(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptI32
                } else {
                    ColumnType::I32
                }
            }
            Self::U32(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptU32
                } else {
                    ColumnType::U32
                }
            }
            Self::I64(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptI64
                } else {
                    ColumnType::I64
                }
            }
            Self::U64(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptU64
                } else {
                    ColumnType::U64
                }
            }
            Self::F32(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptF32
                } else {
                    ColumnType::F32
                }
            }
            Self::F64(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptF64
                } else {
                    ColumnType::F64
                }
            }
            Self::Str(s) => {
                if s.presence.0.is_some() {
                    ColumnType::OptStr
                } else {
                    ColumnType::Str
                }
            }
            Self::SharedDict(..) => ColumnType::SharedDict,
        };
        col_type.write_to(writer)?;

        let name = match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s) => &s.name.0,
            Self::Str(s) => &s.name.0,
            Self::SharedDict(s) => &s.name.0,
        };
        writer.write_string(name)?;

        // Struct children metadata must be written inline here so subsequent column
        // metadata offsets remain correct.
        if let Self::SharedDict(s) = self {
            writer.write_varint(u32::try_from(s.children.len())?)?;
            for child in &s.children {
                child.write_columns_meta_to(writer)?;
                writer.write_string(&child.name.0)?;
            }
        }
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        match self {
            Self::Bool(s) => {
                writer.write_optional_stream(s.presence.0.as_ref())?;
                writer.write_boolean_stream(&s.data)?;
            }
            Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s) => {
                writer.write_optional_stream(s.presence.0.as_ref())?;
                writer.write_stream(&s.data)?;
            }
            Self::Str(s) => {
                let content = s.encoding.content_streams();
                let stream_count =
                    u32::try_from(content.len() + usize::from(s.presence.0.is_some()))
                        .map_err(MltError::from)?;
                writer.write_varint(stream_count)?;
                writer.write_optional_stream(s.presence.0.as_ref())?;
                for stream in content {
                    writer.write_stream(stream)?;
                }
            }
            Self::SharedDict(s) => {
                let dict_streams = s.encoding.dict_streams();
                let dict_stream_len = u32::try_from(dict_streams.len()).map_err(MltError::from)?;
                let children_len = u32::try_from(s.children.len()).map_err(MltError::from)?;
                let optional_children_count =
                    s.children.iter().filter(|c| c.presence.0.is_some()).count();
                let optional_children_len =
                    u32::try_from(optional_children_count).map_err(MltError::from)?;
                let stream_len =
                    checked_sum3(dict_stream_len, children_len, optional_children_len)?;
                writer.write_varint(stream_len)?;
                for stream in dict_streams {
                    writer.write_stream(stream)?;
                }
                for child in &s.children {
                    writer.write_varint(1 + u32::from(child.presence.0.is_some()))?;
                    writer.write_optional_stream(child.presence.0.as_ref())?;
                    writer.write_stream(&child.data)?;
                }
            }
        }
        Ok(())
    }
}
