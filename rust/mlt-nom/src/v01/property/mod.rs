mod decode;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::MltError;
use crate::MltError::IntegerOverflow;
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::utils::{BinarySerializer as _, apply_present, f32_to_json};
use crate::v01::property::decode::{
    decode_shared_dictionary, decode_string_streams, resolve_offsets,
};
use crate::v01::{ColumnType, Stream};

/// Raw data for a Struct column with shared dictionary encoding
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawStructProp<'a> {
    pub dict_streams: Vec<Stream<'a>>,
    pub children: Vec<RawStructChild<'a>>,
}

/// A single child field within a Struct column
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawStructChild<'a> {
    pub name: &'a str,
    pub typ: ColumnType,
    pub optional: Option<Stream<'a>>,
    pub data: Stream<'a>,
}

impl OwnedRawStructChild {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.typ.write_to(writer)?;
        writer.write_string(&self.name)?;
        Ok(())
    }
}

/// Property representation, either raw or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Raw(RawProperty<'a>),
    Decoded(DecodedProperty),
}

impl Analyze for Property<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(d) => d.collect_statistic(stat),
            Self::Decoded(d) => d.collect_statistic(stat),
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

impl<'a> RawProperty<'a> {
    pub(crate) fn new(
        name: &'a str,
        optional: Option<Stream<'a>>,
        value: RawPropValue<'a>,
    ) -> Self {
        Self {
            name,
            optional,
            value,
        }
    }
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
        if let OwnedRawPropValue::Struct(s) = &self.value {
            // Yes, we need to write the children right here, otherwise this messes up the next columns metadata
            let child_column_count =
                u64::try_from(s.children.len()).map_err(|_| IntegerOverflow)?;
            writer.write_varint(child_column_count)?;
            for child in &s.children {
                child.write_columns_meta_to(writer)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        use OwnedRawPropValue as Val;

        match &self.value {
            Val::Bool(b) => {
                if let Some(opt) = &self.optional {
                    writer.write_boolean_stream(opt)?;
                }
                writer.write_boolean_stream(b)?;
            }
            Val::I8(s)
            | Val::U8(s)
            | Val::I32(s)
            | Val::U32(s)
            | Val::I64(s)
            | Val::U64(s)
            | Val::F32(s)
            | Val::F64(s) => {
                if let Some(opt) = &self.optional {
                    writer.write_boolean_stream(opt)?;
                }
                writer.write_stream(s)?;
            }
            Val::Str(streams) => {
                let stream_count = u64::try_from(streams.len()).map_err(|_| IntegerOverflow)?;
                let opt_stream_count = u64::from(self.optional.is_some());
                let Some(stream_count) = stream_count.checked_add(opt_stream_count) else {
                    return Err(IntegerOverflow);
                };
                writer.write_varint(stream_count)?;
                if let Some(opt) = &self.optional {
                    writer.write_boolean_stream(opt)?;
                }
                for s in streams {
                    writer.write_stream(s)?;
                }
            }
            Val::Struct(s) => {
                let child_len = u64::try_from(s.children.len()).map_err(|_| IntegerOverflow)?;
                let dict_cnt = u64::try_from(s.dict_streams.len()).map_err(|_| IntegerOverflow)?;
                let stream_count = child_len.checked_add(dict_cnt).ok_or(IntegerOverflow)?;
                writer.write_varint(stream_count)?;
                for dict in &s.dict_streams {
                    writer.write_stream(dict)?;
                }
                for child in &s.children {
                    if let Some(opt) = &child.optional {
                        writer.write_varint(2)?; // stream_count => data and option
                        writer.write_boolean_stream(opt)?;
                    } else {
                        writer.write_varint(1)?; // stream_count => only data stream
                    }
                    writer.write_stream(&child.data)?;
                }
            }
        }
        Ok(())
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
    Struct(RawStructProp<'a>),
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
            | Self::F64(s) => s.for_each_stream(cb),
            Self::Str(streams) => streams.for_each_stream(cb),
            Self::Struct(sp) => {
                sp.dict_streams.for_each_stream(cb);
                for child in &sp.children {
                    child.optional.for_each_stream(cb);
                    child.data.for_each_stream(cb);
                }
            }
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
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.values.collect_statistic(stat)
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
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Bool(v) => v.collect_statistic(stat),
            Self::I8(v) => v.collect_statistic(stat),
            Self::U8(v) => v.collect_statistic(stat),
            Self::I32(v) => v.collect_statistic(stat),
            Self::U32(v) => v.collect_statistic(stat),
            Self::I64(v) => v.collect_statistic(stat),
            Self::U64(v) => v.collect_statistic(stat),
            Self::F32(v) => v.collect_statistic(stat),
            Self::F64(v) => v.collect_statistic(stat),
            Self::Str(v) => v.collect_statistic(stat),
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

    /// Decode this property. Struct properties expand into multiple decoded properties.
    pub fn decode_expand(self) -> Result<Vec<Property<'a>>, MltError> {
        match self {
            Self::Raw(raw) => match raw.value {
                RawPropValue::Struct(v) => decode_struct_children(raw.name, v),
                _ => Ok(vec![Self::Decoded(DecodedProperty::from_raw(raw)?)]),
            },
            Self::Decoded(d) => Ok(vec![Self::Decoded(d)]),
        }
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
            RawPropValue::Struct(_) => Err(MltError::NotDecoded("struct must use decode_expand"))?,
        };
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values,
        })
    }
}

/// Decode a struct with shared dictionary into one decoded property per child.
fn decode_struct_children<'a>(
    parent_name: &str,
    struct_data: RawStructProp<'_>,
) -> Result<Vec<Property<'a>>, MltError> {
    let dict = decode_shared_dictionary(struct_data.dict_streams)?;
    struct_data
        .children
        .into_iter()
        .map(|child| {
            let present = child.optional.map(Stream::decode_bools);
            let offsets = child.data.decode_bits_u32()?.decode_u32()?;
            let strings = resolve_offsets(&dict, &offsets)?;
            let name = format!("{parent_name}{}", child.name);
            let values = PropValue::Str(apply_present(present.as_ref(), strings));
            Ok(Property::Decoded(DecodedProperty { name, values }))
        })
        .collect()
}
