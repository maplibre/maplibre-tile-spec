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
            Self::Bool(..) | Self::BoolOpt(..) => T::Bool,
            Self::I8(..)
            | Self::I8Opt(..)
            | Self::I32(..)
            | Self::I32Opt(..)
            | Self::I64(..)
            | Self::I64Opt(..)
            | Self::U8(..)
            | Self::U8Opt(..)
            | Self::U32(..)
            | Self::U32Opt(..)
            | Self::U64(..)
            | Self::U64Opt(..) => T::Integer,
            Self::F32(..) | Self::F32Opt(..) | Self::F64(..) | Self::F64Opt(..) => T::Float,
            Self::Str(..) => T::String,
            Self::SharedDict(..) => T::SharedDict,
        }
    }

    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let col_type = match self {
            Self::Bool(..) => ColumnType::Bool,
            Self::BoolOpt(..) => ColumnType::OptBool,
            Self::I8(..) => ColumnType::I8,
            Self::I8Opt(..) => ColumnType::OptI8,
            Self::U8(..) => ColumnType::U8,
            Self::U8Opt(..) => ColumnType::OptU8,
            Self::I32(..) => ColumnType::I32,
            Self::I32Opt(..) => ColumnType::OptI32,
            Self::U32(..) => ColumnType::U32,
            Self::U32Opt(..) => ColumnType::OptU32,
            Self::I64(..) => ColumnType::I64,
            Self::I64Opt(..) => ColumnType::OptI64,
            Self::U64(..) => ColumnType::U64,
            Self::U64Opt(..) => ColumnType::OptU64,
            Self::F32(..) => ColumnType::F32,
            Self::F32Opt(..) => ColumnType::OptF32,
            Self::F64(..) => ColumnType::F64,
            Self::F64Opt(..) => ColumnType::OptF64,
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

        #[allow(clippy::match_same_arms)]
        let name = match self {
            Self::Bool(name, _)
            | Self::I8(name, _)
            | Self::U8(name, _)
            | Self::I32(name, _)
            | Self::U32(name, _)
            | Self::I64(name, _)
            | Self::U64(name, _)
            | Self::F32(name, _)
            | Self::F64(name, _) => &name.0,
            Self::BoolOpt(name, _, _)
            | Self::I8Opt(name, _, _)
            | Self::U8Opt(name, _, _)
            | Self::I32Opt(name, _, _)
            | Self::U32Opt(name, _, _)
            | Self::I64Opt(name, _, _)
            | Self::U64Opt(name, _, _)
            | Self::F32Opt(name, _, _)
            | Self::F64Opt(name, _, _)
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
            Self::Bool(_, data) => {
                writer.write_boolean_stream(data)?;
            }
            Self::BoolOpt(_, presence, data) => {
                writer.write_optional_stream(presence.0.as_ref())?;
                writer.write_boolean_stream(data)?;
            }
            Self::I8(_, data)
            | Self::U8(_, data)
            | Self::I32(_, data)
            | Self::U32(_, data)
            | Self::I64(_, data)
            | Self::U64(_, data)
            | Self::F32(_, data)
            | Self::F64(_, data) => {
                writer.write_stream(data)?;
            }
            Self::I8Opt(_, presence, data)
            | Self::U8Opt(_, presence, data)
            | Self::I32Opt(_, presence, data)
            | Self::U32Opt(_, presence, data)
            | Self::I64Opt(_, presence, data)
            | Self::U64Opt(_, presence, data)
            | Self::F32Opt(_, presence, data)
            | Self::F64Opt(_, presence, data) => {
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
