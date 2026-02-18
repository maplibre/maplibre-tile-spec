mod decode;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::utils::{BinarySerializer as _, apply_present, f32_to_json};
use crate::v01::property::decode::decode_string_streams;
use crate::v01::{ColumnType, Stream};

/// Property representation, either raw or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Raw(RawProperty<'a>),
    Decoded(DecodedProperty),
}

impl Analyze for Property<'_> {
    fn decoded_statistics_for(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(d) => d.decoded_statistics_for(stat),
            Self::Decoded(d) => d.decoded_statistics_for(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Raw(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

impl OwnedProperty {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Raw(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Raw(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

/// Unparsed property data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawProperty<'a> {
    name: &'a str,
    optional: Option<Stream<'a>>,
    value: RawPropValue<'a>,
}

impl Analyze for RawProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.optional.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl OwnedRawProperty {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        // type
        match (&self.value, &self.optional) {
            (OwnedRawPropValue::Bool(_), Some(_)) => ColumnType::OptBool.write_to(writer)?,
            (OwnedRawPropValue::Bool(_), None) => ColumnType::Bool.write_to(writer)?,
            (OwnedRawPropValue::I8(_), Some(_)) => ColumnType::OptI8.write_to(writer)?,
            (OwnedRawPropValue::I8(_), None) => ColumnType::I8.write_to(writer)?,
            (OwnedRawPropValue::U8(_), Some(_)) => ColumnType::OptU8.write_to(writer)?,
            (OwnedRawPropValue::U8(_), None) => ColumnType::U8.write_to(writer)?,
            (OwnedRawPropValue::I32(_), Some(_)) => ColumnType::OptI32.write_to(writer)?,
            (OwnedRawPropValue::I32(_), None) => ColumnType::I32.write_to(writer)?,
            (OwnedRawPropValue::U32(_), Some(_)) => ColumnType::OptU32.write_to(writer)?,
            (OwnedRawPropValue::U32(_), None) => ColumnType::U32.write_to(writer)?,
            (OwnedRawPropValue::I64(_), Some(_)) => ColumnType::OptI64.write_to(writer)?,
            (OwnedRawPropValue::I64(_), None) => ColumnType::I64.write_to(writer)?,
            (OwnedRawPropValue::U64(_), Some(_)) => ColumnType::OptU64.write_to(writer)?,
            (OwnedRawPropValue::U64(_), None) => ColumnType::U64.write_to(writer)?,
            (OwnedRawPropValue::F32(_), Some(_)) => ColumnType::OptF32.write_to(writer)?,
            (OwnedRawPropValue::F32(_), None) => ColumnType::F32.write_to(writer)?,
            (OwnedRawPropValue::F64(_), Some(_)) => ColumnType::OptF64.write_to(writer)?,
            (OwnedRawPropValue::F64(_), None) => ColumnType::F64.write_to(writer)?,
            (OwnedRawPropValue::Str(_), Some(_)) => ColumnType::OptStr.write_to(writer)?,
            (OwnedRawPropValue::Str(_), None) => ColumnType::Str.write_to(writer)?,
            (OwnedRawPropValue::Struct(_), None) => ColumnType::Struct.write_to(writer)?,
            (OwnedRawPropValue::Struct(_), Some(_)) => {
                return Err(MltError::TriedToEncodeOptionalStruct);
            }
        }

        // name
        writer.write_string(&self.name)?;

        // struct children
        if let OwnedRawPropValue::Struct(_) = &self.value {
            // Yes, we need to write the children right here, otherwise this messes up the next columns metadata
            return Err(MltError::NotImplemented("struct child meta writing"));
        }
        Ok(())
    }

    #[expect(clippy::unused_self)]
    pub(crate) fn write_to<W: Write>(&self, _writer: &mut W) -> Result<(), MltError> {
        Err(MltError::NotImplemented("property write"))
    }
}

/// A sequence of encoded (raw) property values of various types
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawPropValue<'a> {
    Bool(Stream<'a>),
    I8(Stream<'a>),
    U8(Stream<'a>),
    I32(Stream<'a>),
    U32(Stream<'a>),
    I64(Stream<'a>),
    U64(Stream<'a>),
    F32(Stream<'a>),
    F64(Stream<'a>),
    Str(Vec<Stream<'a>>),
    Struct(Stream<'a>),
}

impl Analyze for RawPropValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s)
            | Self::Struct(s) => s.for_each_stream(cb),
            Self::Str(streams) => streams.for_each_stream(cb),
        }
    }
}

/// Decoded property values as a name and a vector of optional typed values
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedProperty {
    pub name: String,
    pub values: PropValue,
}

impl Analyze for DecodedProperty {
    fn decoded_statistics_for(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::MetadataOverheadBytes {
            self.name.len()
        } else {
            0
        };
        meta + self.values.decoded_statistics_for(stat)
    }
}

/// Decoded property value types
#[derive(Clone, Default, PartialEq)]
pub enum PropValue {
    Bool(Vec<Option<bool>>),
    I8(Vec<Option<i8>>),
    U8(Vec<Option<u8>>),
    I32(Vec<Option<i32>>),
    U32(Vec<Option<u32>>),
    I64(Vec<Option<i64>>),
    U64(Vec<Option<u64>>),
    F32(Vec<Option<f32>>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    #[default]
    Struct,
}

impl Analyze for PropValue {
    fn decoded_statistics_for(&self, stat: StatType) -> usize {
        match self {
            Self::Bool(v) => v.decoded_statistics_for(stat),
            Self::I8(v) => v.decoded_statistics_for(stat),
            Self::U8(v) => v.decoded_statistics_for(stat),
            Self::I32(v) => v.decoded_statistics_for(stat),
            Self::U32(v) => v.decoded_statistics_for(stat),
            Self::I64(v) => v.decoded_statistics_for(stat),
            Self::U64(v) => v.decoded_statistics_for(stat),
            Self::F32(v) => v.decoded_statistics_for(stat),
            Self::F64(v) => v.decoded_statistics_for(stat),
            Self::Str(v) => v.decoded_statistics_for(stat),
            Self::Struct => 0,
        }
    }
}

/// Format `Option` values on a single line each, even in alternate/pretty mode.
struct FmtOptVec<'a, T>(&'a [Option<T>]);

impl<T: Debug> Debug for FmtOptVec<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for item in self.0 {
            // Always format each element in compact (non-alternate) mode
            list.entry(&format_args!("{item:?}"));
        }
        list.finish()
    }
}

impl Debug for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => f.debug_tuple("Bool").field(&FmtOptVec(v)).finish(),
            Self::I8(v) => f.debug_tuple("I8").field(&FmtOptVec(v)).finish(),
            Self::U8(v) => f.debug_tuple("U8").field(&FmtOptVec(v)).finish(),
            Self::I32(v) => f.debug_tuple("I32").field(&FmtOptVec(v)).finish(),
            Self::U32(v) => f.debug_tuple("U32").field(&FmtOptVec(v)).finish(),
            Self::I64(v) => f.debug_tuple("I64").field(&FmtOptVec(v)).finish(),
            Self::U64(v) => f.debug_tuple("U64").field(&FmtOptVec(v)).finish(),
            Self::F32(v) => f.debug_tuple("F32").field(&FmtOptVec(v)).finish(),
            Self::F64(v) => f.debug_tuple("F64").field(&FmtOptVec(v)).finish(),
            Self::Str(v) => f.debug_tuple("Str").field(&FmtOptVec(v)).finish(),
            Self::Struct => write!(f, "Struct"),
        }
    }
}

impl PropValue {
    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
    #[expect(clippy::cast_possible_truncation)] // f64 stored as f32 in wire format
    pub fn to_geojson(&self, i: usize) -> Option<serde_json::Value> {
        use serde_json::Value;

        match self {
            Self::Bool(v) => v[i].map(Value::Bool),
            Self::I8(v) => v[i].map(Value::from),
            Self::U8(v) => v[i].map(Value::from),
            Self::I32(v) => v[i].map(Value::from),
            Self::U32(v) => v[i].map(Value::from),
            Self::I64(v) => v[i].map(Value::from),
            Self::U64(v) => v[i].map(Value::from),
            Self::F32(v) => v[i].map(f32_to_json),
            Self::F64(v) => v[i].map(|f| f32_to_json(f as f32)),
            Self::Str(v) => v[i].as_ref().map(|s| Value::String(s.clone())),
            Self::Struct => None,
        }
    }
}

impl_decodable!(Property<'a>, RawProperty<'a>, DecodedProperty);

impl<'a> From<RawProperty<'a>> for Property<'a> {
    fn from(value: RawProperty<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn raw(name: &'a str, optional: Option<Stream<'a>>, value: RawPropValue<'a>) -> Self {
        Self::Raw(RawProperty {
            name,
            optional,
            value,
        })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedProperty, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedProperty::from_raw(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl<'a> FromRaw<'a> for DecodedProperty {
    type Input = RawProperty<'a>;

    fn from_raw(v: RawProperty<'_>) -> Result<Self, MltError> {
        let present = v.optional.map(Stream::decode_bools);
        let values = match v.value {
            RawPropValue::Bool(s) => {
                PropValue::Bool(apply_present(present.as_ref(), s.decode_bools()))
            }
            RawPropValue::I8(s) => PropValue::I8(apply_present(
                present.as_ref(),
                s.decode_signed_int_stream()?,
            )),
            RawPropValue::U8(s) => PropValue::U8(apply_present(
                present.as_ref(),
                s.decode_unsigned_int_stream()?,
            )),
            RawPropValue::I32(s) => PropValue::I32(apply_present(
                present.as_ref(),
                s.decode_signed_int_stream()?,
            )),
            RawPropValue::U32(s) => PropValue::U32(apply_present(
                present.as_ref(),
                s.decode_unsigned_int_stream()?,
            )),
            RawPropValue::I64(s) => {
                PropValue::I64(apply_present(present.as_ref(), s.decode_i64()?))
            }
            RawPropValue::U64(s) => {
                PropValue::U64(apply_present(present.as_ref(), s.decode_u64()?))
            }
            RawPropValue::F32(s) => {
                PropValue::F32(apply_present(present.as_ref(), s.decode_f32s()))
            }
            RawPropValue::F64(s) => PropValue::F64(apply_present(
                present.as_ref(),
                s.decode_f32s().into_iter().map(f64::from).collect(),
            )),
            RawPropValue::Str(streams) => PropValue::Str(apply_present(
                present.as_ref(),
                decode_string_streams(streams)?,
            )),
            RawPropValue::Struct(_) => PropValue::Struct,
        };
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values,
        })
    }
}
