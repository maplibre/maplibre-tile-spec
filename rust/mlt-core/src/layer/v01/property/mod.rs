pub(crate) mod decode;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;
use derive_builder::Builder;
use integer_encoding::VarIntWriter as _;

use crate::MltError::{IntegerOverflow, NotImplemented};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::utils::{
    BinarySerializer as _, FmtOptVec, apply_present, encode_bools_to_bytes, encode_byte_rle,
    f32_to_json,
};
use crate::v01::property::decode::{decode_string_streams, decode_struct_children};
use crate::v01::{
    ColumnType, Encoder, LogicalEncoder, LogicalEncoding, OwnedEncodedData, OwnedStream,
    OwnedStreamData, PhysicalEncoder, PhysicalEncoding, PhysicalStreamType, Stream, StreamMeta,
};
use crate::{FromDecoded, MltError, impl_encodable};

/// Encoded data for a Struct column with shared dictionary encoding
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedStructProp<'a> {
    pub dict_streams: Vec<Stream<'a>>,
    pub children: Vec<EncodedStructChild<'a>>,
}

/// A single child field within a Struct column
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedStructChild<'a> {
    pub name: &'a str,
    pub typ: ColumnType,
    pub optional: Option<Stream<'a>>,
    pub data: Stream<'a>,
}

impl OwnedEncodedStructChild {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.typ.write_to(writer)?;
        writer.write_string(&self.name)?;
        Ok(())
    }
}

/// Property representation, either encoded or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Encoded(EncodedProperty<'a>),
    Decoded(DecodedProperty),
}

impl Analyze for Property<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(d) => d.collect_statistic(stat),
            Self::Decoded(d) => d.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Encoded(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

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
}

/// Unparsed property data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedProperty<'a> {
    name: &'a str,
    optional: Option<Stream<'a>>,
    value: EncodedPropValue<'a>,
}

impl<'a> EncodedProperty<'a> {
    pub(crate) fn new(
        name: &'a str,
        optional: Option<Stream<'a>>,
        value: EncodedPropValue<'a>,
    ) -> Self {
        Self {
            name,
            optional,
            value,
        }
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.optional.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl OwnedEncodedProperty {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        // type
        match (&self.value, &self.optional) {
            (OwnedEncodedPropValue::Bool(_), Some(_)) => ColumnType::OptBool.write_to(writer)?,
            (OwnedEncodedPropValue::Bool(_), None) => ColumnType::Bool.write_to(writer)?,
            (OwnedEncodedPropValue::I8(_), Some(_)) => ColumnType::OptI8.write_to(writer)?,
            (OwnedEncodedPropValue::I8(_), None) => ColumnType::I8.write_to(writer)?,
            (OwnedEncodedPropValue::U8(_), Some(_)) => ColumnType::OptU8.write_to(writer)?,
            (OwnedEncodedPropValue::U8(_), None) => ColumnType::U8.write_to(writer)?,
            (OwnedEncodedPropValue::I32(_), Some(_)) => ColumnType::OptI32.write_to(writer)?,
            (OwnedEncodedPropValue::I32(_), None) => ColumnType::I32.write_to(writer)?,
            (OwnedEncodedPropValue::U32(_), Some(_)) => ColumnType::OptU32.write_to(writer)?,
            (OwnedEncodedPropValue::U32(_), None) => ColumnType::U32.write_to(writer)?,
            (OwnedEncodedPropValue::I64(_), Some(_)) => ColumnType::OptI64.write_to(writer)?,
            (OwnedEncodedPropValue::I64(_), None) => ColumnType::I64.write_to(writer)?,
            (OwnedEncodedPropValue::U64(_), Some(_)) => ColumnType::OptU64.write_to(writer)?,
            (OwnedEncodedPropValue::U64(_), None) => ColumnType::U64.write_to(writer)?,
            (OwnedEncodedPropValue::F32(_), Some(_)) => ColumnType::OptF32.write_to(writer)?,
            (OwnedEncodedPropValue::F32(_), None) => ColumnType::F32.write_to(writer)?,
            (OwnedEncodedPropValue::F64(_), Some(_)) => ColumnType::OptF64.write_to(writer)?,
            (OwnedEncodedPropValue::F64(_), None) => ColumnType::F64.write_to(writer)?,
            (OwnedEncodedPropValue::Str(_), Some(_)) => ColumnType::OptStr.write_to(writer)?,
            (OwnedEncodedPropValue::Str(_), None) => ColumnType::Str.write_to(writer)?,
            (OwnedEncodedPropValue::Struct(_), None) => ColumnType::Struct.write_to(writer)?,
            (OwnedEncodedPropValue::Struct(_), Some(_)) => {
                return Err(MltError::TriedToEncodeOptionalStruct);
            }
        }

        // name
        writer.write_string(&self.name)?;

        // struct children
        if let OwnedEncodedPropValue::Struct(s) = &self.value {
            // Yes, we need to write the children right here, otherwise this messes up the next columns metadata
            let child_column_count = u64::try_from(s.children.len())?;
            writer.write_varint(child_column_count)?;
            for child in &s.children {
                child.write_columns_meta_to(writer)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        use OwnedEncodedPropValue as Val;

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
                let stream_count = u64::try_from(streams.len())?;
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
                let child_len = u64::try_from(s.children.len())?;
                let dict_cnt = u64::try_from(s.dict_streams.len())?;
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

impl Default for OwnedEncodedProperty {
    fn default() -> Self {
        Self {
            name: String::default(),
            optional: None,
            value: OwnedEncodedPropValue::Bool(OwnedStream::empty_without_encoding()),
        }
    }
}

/// A sequence of encoded property values of various types
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedPropValue<'a> {
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
    Struct(EncodedStructProp<'a>),
}

impl Analyze for EncodedPropValue<'_> {
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
impl PropValue {
    fn as_presence_stream(&self) -> Result<Vec<bool>, MltError> {
        Ok(match self {
            PropValue::Bool(v) => v.iter().map(Option::is_some).collect(),
            PropValue::I8(v) => v.iter().map(Option::is_some).collect(),
            PropValue::U8(v) => v.iter().map(Option::is_some).collect(),
            PropValue::I32(v) => v.iter().map(Option::is_some).collect(),
            PropValue::U32(v) => v.iter().map(Option::is_some).collect(),
            PropValue::I64(v) => v.iter().map(Option::is_some).collect(),
            PropValue::U64(v) => v.iter().map(Option::is_some).collect(),
            PropValue::F32(v) => v.iter().map(Option::is_some).collect(),
            PropValue::F64(v) => v.iter().map(Option::is_some).collect(),
            PropValue::Str(v) => v.iter().map(Option::is_some).collect(),
            PropValue::Struct => Err(NotImplemented("struct property encoding"))?,
        })
    }
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

impl_decodable!(Property<'a>, EncodedProperty<'a>, DecodedProperty);
impl_encodable!(OwnedProperty, DecodedProperty, OwnedEncodedProperty);

impl<'a> From<EncodedProperty<'a>> for Property<'a> {
    fn from(value: EncodedProperty<'a>) -> Self {
        Self::Encoded(value)
    }
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn new_encoded(
        name: &'a str,
        optional: Option<Stream<'a>>,
        value: EncodedPropValue<'a>,
    ) -> Self {
        Self::Encoded(EncodedProperty {
            name,
            optional,
            value,
        })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedProperty, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedProperty::from_encoded(v)?,
            Self::Decoded(v) => v,
        })
    }

    /// Decode this property. Struct properties expand into multiple decoded properties.
    pub fn decode_expand(self) -> Result<Vec<Property<'a>>, MltError> {
        match self {
            Self::Encoded(enc) => match enc.value {
                EncodedPropValue::Struct(v) => decode_struct_children(enc.name, v),
                _ => Ok(vec![Self::Decoded(DecodedProperty::from_encoded(enc)?)]),
            },
            Self::Decoded(d) => Ok(vec![Self::Decoded(d)]),
        }
    }
}

/// How to encode properties
#[derive(Debug, Clone, Copy, Builder)]
pub struct PropertyEncodingStrategy {
    optional: PresenceStream,
    logical: LogicalEncoder,
    physical: PhysicalEncoder,
}
impl PropertyEncodingStrategy {
    #[must_use]
    pub fn encoding(self) -> Encoder {
        Encoder::new(self.logical, self.physical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStream {
    /// Attaches a nullability stream
    Present,
    /// If there are nulls, drop them
    Absent,
}

impl FromDecoded<'_> for OwnedEncodedProperty {
    type Input = DecodedProperty;
    type Encoder = PropertyEncodingStrategy;

    fn from_decoded(decoded: &Self::Input, config: Self::Encoder) -> Result<Self, MltError> {
        use {OwnedEncodedPropValue as EncVal, PropValue as Val};
        let optional = if config.optional == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.values.as_presence_stream()?;
            let data = encode_byte_rle(&encode_bools_to_bytes(&present_vec));
            Some(OwnedStream {
                meta: StreamMeta {
                    physical_type: PhysicalStreamType::Present,
                    num_values: u32::try_from(present_vec.len())?,
                    logical_encoding: LogicalEncoding::None,
                    physical_encoding: PhysicalEncoding::None,
                },
                data: OwnedStreamData::Encoded(OwnedEncodedData { data }),
            })
        } else {
            None
        };

        let value = match &decoded.values {
            Val::Bool(b) => EncVal::Bool(OwnedStream::encode_bools(&unapply_presence(b))?),
            Val::I8(i) => {
                let vals = unapply_presence(i);
                EncVal::I8(OwnedStream::encode_i8s(&vals, config.encoding())?)
            }
            Val::U8(u) => {
                let values = unapply_presence(u);
                EncVal::U8(OwnedStream::encode_u8s(&values, config.encoding())?)
            }
            Val::I32(i) => {
                let vals = unapply_presence(i);
                EncVal::I32(OwnedStream::encode_i32s(&vals, config.encoding())?)
            }
            Val::U32(u) => {
                let vals = unapply_presence(u);
                EncVal::U32(OwnedStream::encode_u32s(&vals, config.encoding())?)
            }
            Val::I64(i) => {
                let vals = unapply_presence(i);
                EncVal::I64(OwnedStream::encode_i64s(&vals, config.encoding())?)
            }
            Val::U64(u) => {
                let vals = unapply_presence(u);
                EncVal::U64(OwnedStream::encode_u64s(&vals, config.encoding())?)
            }
            Val::F32(f) => {
                let vals = unapply_presence(f);
                EncVal::F32(OwnedStream::encode_f32(&vals)?)
            }
            Val::F64(f) => {
                let values = unapply_presence(f);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the decoder does it this way, but why?"
                )]
                let values = values.iter().map(|&f| f as f32).collect::<Vec<_>>();
                EncVal::F64(OwnedStream::encode_f32(&values)?)
            }
            Val::Str(s) => {
                let values = unapply_presence(s);
                EncVal::Str(OwnedStream::encode_strings(&values, config.encoding())?)
            }
            Val::Struct => Err(NotImplemented("struct property encoding"))?,
        };

        Ok(Self {
            name: decoded.name.clone(),
            optional,
            value,
        })
    }
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl<'a> FromEncoded<'a> for DecodedProperty {
    type Input = EncodedProperty<'a>;

    fn from_encoded(v: EncodedProperty<'_>) -> Result<Self, MltError> {
        use {EncodedPropValue as EncVal, PropValue as Val};
        let present = if let Some(c) = v.optional {
            Some(c.decode_bools()?)
        } else {
            None
        };
        let values = match v.value {
            EncVal::Bool(s) => Val::Bool(apply_present(present, s.decode_bools()?)?),
            EncVal::I8(s) => Val::I8(apply_present(present, s.decode_i8s()?)?),
            EncVal::U8(s) => Val::U8(apply_present(present, s.decode_u8s()?)?),
            EncVal::I32(s) => Val::I32(apply_present(present, s.decode_i32s()?)?),
            EncVal::U32(s) => Val::U32(apply_present(present, s.decode_u32s()?)?),
            EncVal::I64(s) => Val::I64(apply_present(present, s.decode_i64()?)?),
            EncVal::U64(s) => Val::U64(apply_present(present, s.decode_u64()?)?),
            EncVal::F32(s) => Val::F32(apply_present(present, s.decode_f32()?)?),
            EncVal::F64(s) => Val::F64(apply_present(
                present,
                s.decode_f32()?.into_iter().map(f64::from).collect(),
            )?),
            EncVal::Str(streams) => {
                Val::Str(apply_present(present, decode_string_streams(streams)?)?)
            }
            EncVal::Struct(_) => Err(MltError::NotDecoded("struct must use decode_expand"))?,
        };
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Encode a `DecodedProperty` and immediately decode it back.
    fn roundtrip(decoded: &DecodedProperty, strategy: PropertyEncodingStrategy) -> DecodedProperty {
        let encoded =
            OwnedEncodedProperty::from_decoded(decoded, strategy).expect("encoding failed");
        let borrowed = borrowme::borrow(&encoded);
        DecodedProperty::from_encoded(borrowed).expect("decoding failed")
    }

    proptest! {
        #[test]
        fn test_bool_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<bool>()), 0..100),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::Bool(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_bool_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<bool>(), 0..100),
        ) {
            let opt_values: Vec<Option<bool>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::Bool(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i8_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i8>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I8(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i8_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<i8>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<i8>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::I8(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u8_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u8>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U8(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u8_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<u8>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<u8>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::U8(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i32_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i32>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I32(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i32_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<i32>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::I32(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u32_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u32>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U32(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u32_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<u32>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<u32>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::U32(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i64_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i64>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I64(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_i64_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<i64>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::I64(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u64_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u64>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U64(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_u64_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let opt_values: Vec<Option<u64>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::U64(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical,
                physical,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        /// F32 values are stored verbatim (no logical/physical encoding selection).
        /// NaN is excluded because NaN != NaN breaks equality checks.
        #[test]
        fn test_f32_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(
                prop::option::of(any::<f32>().prop_filter("no NaN", |f| !f.is_nan())),
                0..100,
            ),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::F32(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_f32_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(
                any::<f32>().prop_filter("no NaN", |f| !f.is_nan()),
                0..100,
            ),
        ) {
            let opt_values: Vec<Option<f32>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::F32(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        /// F64 is stored as F32 on the wire, so we generate from f32 to avoid precision loss during the roundtrip comparison.
        #[test]
        fn test_f64_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(
                prop::option::of(
                    any::<f32>()
                        .prop_filter("no NaN", |f| !f.is_nan())
                        .prop_map(f64::from)
                ),
                0..100,
            ),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::F64(values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Present,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }

        #[test]
        fn test_f64_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(
                any::<f32>()
                    .prop_filter("no NaN", |f| !f.is_nan())
                    .prop_map(f64::from),
                0..100,
            ),
        ) {
            let opt_values: Vec<Option<f64>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::F64(opt_values) };
            let strategy = PropertyEncodingStrategy {
                optional: PresenceStream::Absent,
                logical: LogicalEncoder::None,
                physical: PhysicalEncoder::None,
            };
            prop_assert_eq!(roundtrip(&decoded, strategy), decoded);
        }
    }
}
