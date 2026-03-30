use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::MltResult;
use crate::utils::{BinarySerializer as _, checked_sum3};
use crate::v01::property::scalars::Scalar;
use crate::v01::{ColumnType, EncodedProperty, PropertyKind, StagedProperty};

impl StagedProperty {
    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        use PropertyKind as T;
        match self {
            Self::Scalar(s) => match s {
                Scalar::Bool(_) => T::Bool,
                Scalar::I8(_)
                | Scalar::I32(_)
                | Scalar::I64(_)
                | Scalar::U8(_)
                | Scalar::U32(_)
                | Scalar::U64(_) => T::Integer,
                Scalar::F32(_) | Scalar::F64(_) => T::Float,
            },
            Self::Str(_) => T::String,
            Self::SharedDict(_) => T::SharedDict,
        }
    }
}

impl EncodedProperty {
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        let (col_type, name) = match self {
            Self::Scalar(s) => {
                let es = s.encoded_scalar();
                let has_presence = es.presence.0.is_some();
                let col_type = match (s, has_presence) {
                    (Scalar::Bool(_), false) => ColumnType::Bool,
                    (Scalar::Bool(_), true) => ColumnType::OptBool,
                    (Scalar::I8(_), false) => ColumnType::I8,
                    (Scalar::I8(_), true) => ColumnType::OptI8,
                    (Scalar::U8(_), false) => ColumnType::U8,
                    (Scalar::U8(_), true) => ColumnType::OptU8,
                    (Scalar::I32(_), false) => ColumnType::I32,
                    (Scalar::I32(_), true) => ColumnType::OptI32,
                    (Scalar::U32(_), false) => ColumnType::U32,
                    (Scalar::U32(_), true) => ColumnType::OptU32,
                    (Scalar::I64(_), false) => ColumnType::I64,
                    (Scalar::I64(_), true) => ColumnType::OptI64,
                    (Scalar::U64(_), false) => ColumnType::U64,
                    (Scalar::U64(_), true) => ColumnType::OptU64,
                    (Scalar::F32(_), false) => ColumnType::F32,
                    (Scalar::F32(_), true) => ColumnType::OptF32,
                    (Scalar::F64(_), false) => ColumnType::F64,
                    (Scalar::F64(_), true) => ColumnType::OptF64,
                };
                (col_type, &es.name.0)
            }
            Self::Str(s) => {
                let col_type = if s.presence.0.is_some() {
                    ColumnType::OptStr
                } else {
                    ColumnType::Str
                };
                (col_type, &s.name.0)
            }
            Self::SharedDict(s) => (ColumnType::SharedDict, &s.name.0),
        };
        col_type.write_to(writer)?;
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
            Self::Scalar(s) => {
                let es = s.encoded_scalar();
                writer.write_optional_stream(es.presence.0.as_ref())?;
                if matches!(s, Scalar::Bool(_)) {
                    writer.write_boolean_stream(&es.data)?;
                } else {
                    writer.write_stream(&es.data)?;
                }
            }
            Self::Str(s) => {
                let content = s.encoding.streams();
                let stream_count =
                    u32::try_from(content.len() + usize::from(s.presence.0.is_some()))?;
                writer.write_varint(stream_count)?;
                writer.write_optional_stream(s.presence.0.as_ref())?;
                for stream in content {
                    writer.write_stream(stream)?;
                }
            }
            Self::SharedDict(s) => {
                let dict_streams = s.encoding.dict_streams();
                let dict_stream_len = u32::try_from(dict_streams.len())?;
                let children_len = u32::try_from(s.children.len())?;
                let optional_children_count =
                    s.children.iter().filter(|c| c.presence.0.is_some()).count();
                let optional_children_len = u32::try_from(optional_children_count)?;
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
