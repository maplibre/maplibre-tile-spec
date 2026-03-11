use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::MltError;
use crate::utils::{BinarySerializer as _, checked_sum3};
use crate::v01::{ColumnType, OwnedEncodedProperty, OwnedProperty, PropertyKind};

impl OwnedProperty {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::Encoded(r) => r.kind(),
            Self::Decoded(r) => r.kind(),
        }
    }
}

impl OwnedEncodedProperty {
    pub(super) fn kind(&self) -> PropertyKind {
        use PropertyKind as T;
        match self {
            Self::Bool(..) => T::Bool,
            Self::I8(..)
            | Self::I32(..)
            | Self::I64(..)
            | Self::U8(..)
            | Self::U32(..)
            | Self::U64(..) => T::Integer,
            Self::F32(..) | Self::F64(..) => T::Float,
            Self::Str(..) => T::String,
            Self::SharedDict(..) => T::SharedDict,
        }
    }

    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let col_type = match self {
            Self::Bool(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptBool
                } else {
                    ColumnType::Bool
                }
            }
            Self::I8(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptI8
                } else {
                    ColumnType::I8
                }
            }
            Self::U8(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptU8
                } else {
                    ColumnType::U8
                }
            }
            Self::I32(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptI32
                } else {
                    ColumnType::I32
                }
            }
            Self::U32(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptU32
                } else {
                    ColumnType::U32
                }
            }
            Self::I64(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptI64
                } else {
                    ColumnType::I64
                }
            }
            Self::U64(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptU64
                } else {
                    ColumnType::U64
                }
            }
            Self::F32(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptF32
                } else {
                    ColumnType::F32
                }
            }
            Self::F64(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptF64
                } else {
                    ColumnType::F64
                }
            }
            Self::Str(_, presence, _) => {
                if presence.0.is_some() {
                    ColumnType::OptStr
                } else {
                    ColumnType::Str
                }
            }
            Self::SharedDict(..) => ColumnType::SharedDict,
        };
        col_type.write_to(writer)?;

        let name = match self {
            Self::Bool(name, _, _)
            | Self::I8(name, _, _)
            | Self::U8(name, _, _)
            | Self::I32(name, _, _)
            | Self::U32(name, _, _)
            | Self::I64(name, _, _)
            | Self::U64(name, _, _)
            | Self::F32(name, _, _)
            | Self::F64(name, _, _)
            | Self::Str(name, _, _)
            | Self::SharedDict(name, _, _) => &name.0,
        };
        writer.write_string(name)?;

        // Struct children metadata must be written inline here so subsequent column
        // metadata offsets remain correct.
        if let Self::SharedDict(_, _, children) = self {
            writer.write_varint(u32::try_from(children.len())?)?;
            for child in children {
                child.write_columns_meta_to(writer)?;
                writer.write_string(&child.name.0)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Bool(_, presence, data) => {
                writer.write_optional_stream(presence.0.as_ref())?;
                writer.write_boolean_stream(data)?;
            }
            Self::I8(_, presence, data)
            | Self::U8(_, presence, data)
            | Self::I32(_, presence, data)
            | Self::U32(_, presence, data)
            | Self::I64(_, presence, data)
            | Self::U64(_, presence, data)
            | Self::F32(_, presence, data)
            | Self::F64(_, presence, data) => {
                writer.write_optional_stream(presence.0.as_ref())?;
                writer.write_stream(data)?;
            }
            Self::Str(_, presence, encoding) => {
                let content = encoding.content_streams();
                let stream_count = u32::try_from(content.len() + usize::from(presence.0.is_some()))
                    .map_err(MltError::from)?;
                writer.write_varint(stream_count)?;
                writer.write_optional_stream(presence.0.as_ref())?;
                for s in content {
                    writer.write_stream(s)?;
                }
            }
            Self::SharedDict(_, s, children) => {
                let dict_streams = s.dict_streams();
                let dict_stream_len = u32::try_from(dict_streams.len()).map_err(MltError::from)?;
                let children_len = u32::try_from(children.len()).map_err(MltError::from)?;
                let optional_children_count =
                    children.iter().filter(|c| c.presence.0.is_some()).count();
                let optional_children_len =
                    u32::try_from(optional_children_count).map_err(MltError::from)?;
                let stream_len =
                    checked_sum3(dict_stream_len, children_len, optional_children_len)?;
                writer.write_varint(stream_len)?;
                for stream in dict_streams {
                    writer.write_stream(stream)?;
                }
                for child in children {
                    // stream_count => data + (0 or 1 for presence stream)
                    // must be u32 because we don't want to zigzag
                    writer.write_varint(1 + u32::from(child.presence.0.is_some()))?;
                    writer.write_optional_stream(child.presence.0.as_ref())?;
                    writer.write_stream(&child.data)?;
                }
            }
        }
        Ok(())
    }
}
