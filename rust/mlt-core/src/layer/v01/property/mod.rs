mod model;
mod optimizer;
pub mod strings;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowmeToOwned};
use integer_encoding::VarIntWriter as _;
pub use model::*;

use crate::MltError::{NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::utils::{
    BinarySerializer as _, FmtOptVec, apply_present, checked_sum3, f32_to_json, f64_to_json,
};
pub use crate::v01::property::optimizer::PropertyOptimizer;
pub use crate::v01::property::strings::{
    SharedDictEncoder, SharedDictItemEncoder, StrEncoder, build_decoded_shared_dict,
    decode_shared_dict, decode_strings, decode_strings_with_presence, encode_shared_dict_prop,
};
use crate::v01::{
    ColumnType, DictionaryType, FsstStrEncoder, IntEncoder, LengthType, OwnedStream, Stream,
};
use crate::{Decodable as _, FromDecoded, MltError, impl_encodable};

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

    #[must_use]
    pub fn approx_type(&self) -> ApproxPropertyType {
        use ApproxPropertyType as T;
        use OwnedEncodedProperty as Enc;
        match self {
            Self::Encoded(Enc::Bool(..)) => T::Bool,
            Self::Encoded(
                Enc::I8(..)
                | Enc::I32(..)
                | Enc::I64(..)
                | Enc::U8(..)
                | Enc::U32(..)
                | Enc::U64(..),
            ) => T::Integer,
            Self::Encoded(Enc::F32(..) | Enc::F64(..)) => T::Float,
            Self::Encoded(Enc::Str(..)) => T::String,
            Self::Encoded(Enc::SharedDict(..)) => T::SharedDict,
            Self::Decoded(r) => r.approx_type(),
        }
    }
}

impl<'a> EncodedProperty<'a> {
    fn name(&self) -> &'a str {
        match self {
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
            | Self::SharedDict(name, _, _) => name.0,
        }
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(_, presence, data)
            | Self::I8(_, presence, data)
            | Self::U8(_, presence, data)
            | Self::I32(_, presence, data)
            | Self::U32(_, presence, data)
            | Self::I64(_, presence, data)
            | Self::U64(_, presence, data)
            | Self::F32(_, presence, data)
            | Self::F64(_, presence, data) => {
                presence.0.for_each_stream(cb);
                data.for_each_stream(cb);
            }
            Self::Str(_, presence, enc) => {
                presence.0.for_each_stream(cb);
                for s in enc.streams() {
                    cb(s);
                }
            }
            Self::SharedDict(_, shared, children) => {
                for stream in shared.dict_streams() {
                    cb(stream);
                }
                for child in children {
                    child.presence.0.for_each_stream(cb);
                    child.data.for_each_stream(cb);
                }
            }
        }
    }
}

impl OwnedEncodedProperty {
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

/// FIXME: why should there be a default???
impl Default for OwnedEncodedProperty {
    fn default() -> Self {
        Self::Bool(
            OwnedName(String::default()),
            OwnedEncodedPresence(None),
            OwnedStream::empty_without_encoding(),
        )
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedEncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: DecodedProperty<'static> = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Self =
            Self::from_decoded(&decoded, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(prop)
    }
}

impl Analyze for DecodedProperty<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name().len()
        } else {
            0
        };
        meta + self.collect_value_statistic(stat)
    }
}

impl DecodedPresence {
    #[must_use]
    pub fn bools(&self, non_null_count: usize) -> Vec<bool> {
        self.0.clone().unwrap_or_else(|| vec![true; non_null_count])
    }

    #[must_use]
    pub fn feature_count(&self, non_null_count: usize) -> usize {
        self.0.as_ref().map_or(non_null_count, Vec::len)
    }
}

impl From<Vec<bool>> for DecodedPresence {
    fn from(values: Vec<bool>) -> Self {
        if values.iter().all(|v| *v) {
            Self(None)
        } else {
            Self(Some(values))
        }
    }
}

impl From<Option<Vec<bool>>> for DecodedPresence {
    fn from(values: Option<Vec<bool>>) -> Self {
        match values {
            Some(values) => Self::from(values),
            None => Self::default(),
        }
    }
}

impl DecodedProperty<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(name, _)
            | Self::I8(name, _)
            | Self::U8(name, _)
            | Self::I32(name, _)
            | Self::U32(name, _)
            | Self::I64(name, _)
            | Self::U64(name, _)
            | Self::F32(name, _)
            | Self::F64(name, _)
            | Self::Str(name, _)
            | Self::SharedDict(name, _, _) => name,
        }
    }

    #[must_use]
    pub fn approx_type(&self) -> ApproxPropertyType {
        match self {
            Self::Bool(..) => ApproxPropertyType::Bool,
            Self::I8(..)
            | Self::U8(..)
            | Self::I32(..)
            | Self::U32(..)
            | Self::I64(..)
            | Self::U64(..) => ApproxPropertyType::Integer,
            Self::F32(..) | Self::F64(..) => ApproxPropertyType::Float,
            Self::Str(..) => ApproxPropertyType::String,
            Self::SharedDict(..) => ApproxPropertyType::SharedDict,
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            Self::Bool(..) => "bool",
            Self::I8(..) => "i8",
            Self::U8(..) => "u8",
            Self::I32(..) => "i32",
            Self::U32(..) => "u32",
            Self::I64(..) => "i64",
            Self::U64(..) => "u64",
            Self::F32(..) => "f32",
            Self::F64(..) => "f64",
            Self::Str(..) => "str",
            Self::SharedDict(..) => "shared_dict",
        }
    }

    fn as_presence_stream(&self) -> Result<Vec<bool>, MltError> {
        Ok(match self {
            Self::Bool(_, v) => v.iter().map(Option::is_some).collect(),
            Self::I8(_, v) => v.iter().map(Option::is_some).collect(),
            Self::U8(_, v) => v.iter().map(Option::is_some).collect(),
            Self::I32(_, v) => v.iter().map(Option::is_some).collect(),
            Self::U32(_, v) => v.iter().map(Option::is_some).collect(),
            Self::I64(_, v) => v.iter().map(Option::is_some).collect(),
            Self::U64(_, v) => v.iter().map(Option::is_some).collect(),
            Self::F32(_, v) => v.iter().map(Option::is_some).collect(),
            Self::F64(_, v) => v.iter().map(Option::is_some).collect(),
            Self::Str(_, values) => values.presence_bools(),
            Self::SharedDict(..) => Err(NotImplemented("presence stream for shared dict"))?,
        })
    }

    fn collect_value_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Bool(_, v) => v.collect_statistic(stat),
            Self::I8(_, v) => v.collect_statistic(stat),
            Self::U8(_, v) => v.collect_statistic(stat),
            Self::I32(_, v) => v.collect_statistic(stat),
            Self::U32(_, v) => v.collect_statistic(stat),
            Self::I64(_, v) => v.collect_statistic(stat),
            Self::U64(_, v) => v.collect_statistic(stat),
            Self::F32(_, v) => v.collect_statistic(stat),
            Self::F64(_, v) => v.collect_statistic(stat),
            Self::Str(_, v) => v.collect_statistic(stat),
            Self::SharedDict(_, shared_dict, items) => items
                .iter()
                .map(|item| item.materialize(shared_dict).collect_statistic(stat))
                .sum(),
        }
    }

    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
    pub fn to_geojson(&self, i: usize) -> Option<serde_json::Value> {
        use serde_json::Value;

        match self {
            Self::Bool(_, v) => v[i].map(Value::Bool),
            Self::I8(_, v) => v[i].map(Value::from),
            Self::U8(_, v) => v[i].map(Value::from),
            Self::I32(_, v) => v[i].map(Value::from),
            Self::U32(_, v) => v[i].map(Value::from),
            Self::I64(_, v) => v[i].map(Value::from),
            Self::U64(_, v) => v[i].map(Value::from),
            Self::F32(_, v) => v[i].map(f32_to_json),
            Self::F64(_, v) => v[i].map(f64_to_json),
            Self::Str(_, values) => values
                .get(u32::try_from(i).ok()?)
                .map(|s| Value::String(s.to_string())),
            Self::SharedDict(_, shared_dict, items) => {
                let mut obj = serde_json::Map::new();
                for item in items {
                    if let Some(s) = item.get(shared_dict, i) {
                        obj.insert(item.suffix.clone(), Value::String(s.to_string()));
                    }
                }
                if obj.is_empty() {
                    None
                } else {
                    Some(Value::Object(obj))
                }
            }
        }
    }
}

impl DecodedProperty<'static> {
    pub fn from_parts(name: impl Into<String>, values: PropValue) -> Self {
        let name = name.into();
        match values {
            PropValue::Bool(v) => Self::Bool(name, v),
            PropValue::I8(v) => Self::I8(name, v),
            PropValue::U8(v) => Self::U8(name, v),
            PropValue::I32(v) => Self::I32(name, v),
            PropValue::U32(v) => Self::U32(name, v),
            PropValue::I64(v) => Self::I64(name, v),
            PropValue::U64(v) => Self::U64(name, v),
            PropValue::F32(v) => Self::F32(name, v),
            PropValue::F64(v) => Self::F64(name, v),
            PropValue::Str(values) => Self::Str(name, values),
            PropValue::SharedDict(shared_dict, items) => Self::SharedDict(name, shared_dict, items),
        }
    }
}


impl BorrowmeToOwned for DecodedProperty<'_> {
    type Owned = DecodedProperty<'static>;

    fn to_owned(&self) -> Self::Owned {
        match self {
            Self::Bool(name, values) => DecodedProperty::Bool(name.clone(), values.clone()),
            Self::I8(name, values) => DecodedProperty::I8(name.clone(), values.clone()),
            Self::U8(name, values) => DecodedProperty::U8(name.clone(), values.clone()),
            Self::I32(name, values) => DecodedProperty::I32(name.clone(), values.clone()),
            Self::U32(name, values) => DecodedProperty::U32(name.clone(), values.clone()),
            Self::I64(name, values) => DecodedProperty::I64(name.clone(), values.clone()),
            Self::U64(name, values) => DecodedProperty::U64(name.clone(), values.clone()),
            Self::F32(name, values) => DecodedProperty::F32(name.clone(), values.clone()),
            Self::F64(name, values) => DecodedProperty::F64(name.clone(), values.clone()),
            Self::Str(name, values) => {
                DecodedProperty::Str(name.clone(), BorrowmeToOwned::to_owned(values))
            }
            Self::SharedDict(name, shared_dict, items) => DecodedProperty::SharedDict(
                name.clone(),
                BorrowmeToOwned::to_owned(shared_dict),
                items.clone(),
            ),
        }
    }
}

impl BorrowmeBorrow for DecodedProperty<'static> {
    type Target<'a>
        = DecodedProperty<'a>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        match self {
            Self::Bool(name, values) => DecodedProperty::Bool(name.clone(), values.clone()),
            Self::I8(name, values) => DecodedProperty::I8(name.clone(), values.clone()),
            Self::U8(name, values) => DecodedProperty::U8(name.clone(), values.clone()),
            Self::I32(name, values) => DecodedProperty::I32(name.clone(), values.clone()),
            Self::U32(name, values) => DecodedProperty::U32(name.clone(), values.clone()),
            Self::I64(name, values) => DecodedProperty::I64(name.clone(), values.clone()),
            Self::U64(name, values) => DecodedProperty::U64(name.clone(), values.clone()),
            Self::F32(name, values) => DecodedProperty::F32(name.clone(), values.clone()),
            Self::F64(name, values) => DecodedProperty::F64(name.clone(), values.clone()),
            Self::Str(name, values) => {
                DecodedProperty::Str(name.clone(), BorrowmeBorrow::borrow(values))
            }
            Self::SharedDict(name, shared_dict, items) => DecodedProperty::SharedDict(
                name.clone(),
                BorrowmeBorrow::borrow(shared_dict),
                items.clone(),
            ),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for DecodedProperty<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let name: String = u.arbitrary()?;
        Ok(match u.int_in_range(0..=9)? {
            0 => Self::Bool(name, u.arbitrary()?),
            1 => Self::I8(name, u.arbitrary()?),
            2 => Self::U8(name, u.arbitrary()?),
            3 => Self::I32(name, u.arbitrary()?),
            4 => Self::U32(name, u.arbitrary()?),
            5 => Self::I64(name, u.arbitrary()?),
            6 => Self::U64(name, u.arbitrary()?),
            7 => Self::F32(name, u.arbitrary()?),
            8 => Self::F64(name, u.arbitrary()?),
            _ => Self::Str(name, u.arbitrary()?),
        })
    }
}

impl Debug for DecodedProperty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(name, v) => f
                .debug_tuple("Bool")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::I8(name, v) => f
                .debug_tuple("I8")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::U8(name, v) => f
                .debug_tuple("U8")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::I32(name, v) => f
                .debug_tuple("I32")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::U32(name, v) => f
                .debug_tuple("U32")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::I64(name, v) => f
                .debug_tuple("I64")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::U64(name, v) => f
                .debug_tuple("U64")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::F32(name, v) => f
                .debug_tuple("F32")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::F64(name, v) => f
                .debug_tuple("F64")
                .field(name)
                .field(&FmtOptVec(v))
                .finish(),
            Self::Str(name, values) => {
                let values = values.materialize();
                f.debug_tuple("Str")
                    .field(name)
                    .field(&FmtOptVec(&values))
                    .finish()
            }
            Self::SharedDict(name, _, items) => f
                .debug_tuple("SharedDict")
                .field(name)
                .field(items)
                .finish(),
        }
    }
}

impl_decodable!(Property<'a>, EncodedProperty<'a>, DecodedProperty<'a>);
impl_encodable!(
    OwnedProperty,
    DecodedProperty<'static>,
    OwnedEncodedProperty
);

impl<'a> From<EncodedProperty<'a>> for Property<'a> {
    fn from(value: EncodedProperty<'a>) -> Self {
        Self::Encoded(value)
    }
}

impl<'a> Property<'a> {
    #[inline]
    pub fn decode(self) -> Result<DecodedProperty<'a>, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedProperty::from_encoded(v)?,
            Self::Decoded(v) => v,
        })
    }

    pub fn decoded_property(&mut self) -> Result<&DecodedProperty<'a>, MltError> {
        Ok(self.materialize()?)
    }
}

/// Instruction for how to encode a single decoded property when batch-encoding a
/// [`Vec<DecodedProperty>`] via [`FromDecoded`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyEncoder {
    /// How to encode a scalar property
    Scalar(ScalarEncoder),
    /// How to encode a shared dictionary property (multiple string sub-properties)
    SharedDict(SharedDictEncoder),
}

impl From<SharedDictEncoder> for PropertyEncoder {
    fn from(encoder: SharedDictEncoder) -> Self {
        Self::SharedDict(encoder)
    }
}
impl From<ScalarEncoder> for PropertyEncoder {
    fn from(encoder: ScalarEncoder) -> Self {
        Self::Scalar(encoder)
    }
}

/// How to encode properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ScalarEncoder {
    pub presence: PresenceStream,
    pub value: ScalarValueEncoder,
}

impl ScalarEncoder {
    #[must_use]
    pub fn str(presence: PresenceStream, string_lengths: IntEncoder) -> Self {
        let enc = StrEncoder::Plain { string_lengths };
        Self {
            presence,
            value: ScalarValueEncoder::String(enc),
        }
    }
    /// Create a property encoder with integer encoding
    #[must_use]
    pub fn int(presence: PresenceStream, enc: IntEncoder) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Int(enc),
        }
    }
    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn str_fsst(
        presence: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
    ) -> Self {
        let enc = FsstStrEncoder {
            symbol_lengths,
            dict_lengths,
        };
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::Fsst(enc)),
        }
    }
    /// Create a property encoder for boolean values
    #[must_use]
    pub fn bool(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Bool,
        }
    }
    /// Create a property encoder for float values
    #[must_use]
    pub fn float(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Float,
        }
    }
}

/// How to encode scalar property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum ScalarValueEncoder {
    Int(IntEncoder),
    String(StrEncoder),
    Float,
    Bool,
}

impl ScalarValueEncoder {
    fn name(self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::String(_) => "string",
            Self::Float => "float",
            Self::Bool => "bool",
        }
    }
}

impl FromDecoded<'_> for Vec<OwnedEncodedProperty> {
    type Input = Vec<DecodedProperty<'static>>;
    type Encoder = Vec<PropertyEncoder>;

    fn from_decoded(properties: &Self::Input, encoders: Self::Encoder) -> Result<Self, MltError> {
        if properties.len() != encoders.len() {
            return Err(MltError::EncodingInstructionCountMismatch {
                input_len: properties.len(),
                config_len: encoders.len(),
            });
        }

        let mut result = Vec::with_capacity(properties.len());

        for (prop, encoder) in properties.iter().zip(encoders) {
            match encoder {
                PropertyEncoder::Scalar(enc) => {
                    result.push(OwnedEncodedProperty::from_decoded(prop, enc)?);
                }
                PropertyEncoder::SharedDict(enc) => {
                    let DecodedProperty::SharedDict(name, shared_dict, items) = prop else {
                        return Err(UnsupportedPropertyEncoderCombination(
                            prop.kind_name(),
                            "SharedDict",
                        ));
                    };
                    result.push(encode_shared_dict_prop(name, shared_dict, items, &enc)?);
                }
            }
        }

        Ok(result)
    }
}

impl FromDecoded<'_> for OwnedEncodedProperty {
    type Input = DecodedProperty<'static>;
    type Encoder = ScalarEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError> {
        let presence = if encoder.presence == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.as_presence_stream()?;
            Some(OwnedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        match (decoded, encoder.value) {
            (DecodedProperty::Bool(name, b), ScalarValueEncoder::Bool) => Ok(Self::Bool(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_bools(&unapply_presence(b))?,
            )),
            (DecodedProperty::I8(name, i), ScalarValueEncoder::Int(enc)) => Ok(Self::I8(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i8s(&unapply_presence(i), enc)?,
            )),
            (DecodedProperty::U8(name, u), ScalarValueEncoder::Int(enc)) => Ok(Self::U8(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u8s(&unapply_presence(u), enc)?,
            )),
            (DecodedProperty::I32(name, i), ScalarValueEncoder::Int(enc)) => Ok(Self::I32(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i32s(&unapply_presence(i), enc)?,
            )),
            (DecodedProperty::U32(name, u), ScalarValueEncoder::Int(enc)) => Ok(Self::U32(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u32s(&unapply_presence(u), enc)?,
            )),
            (DecodedProperty::I64(name, i), ScalarValueEncoder::Int(enc)) => Ok(Self::I64(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i64s(&unapply_presence(i), enc)?,
            )),
            (DecodedProperty::U64(name, u), ScalarValueEncoder::Int(enc)) => Ok(Self::U64(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u64s(&unapply_presence(u), enc)?,
            )),
            (DecodedProperty::F32(name, f), ScalarValueEncoder::Float) => Ok(Self::F32(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_f32(&unapply_presence(f))?,
            )),
            (DecodedProperty::F64(name, d), ScalarValueEncoder::Float) => Ok(Self::F64(
                OwnedName(name.clone()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_f64(&unapply_presence(d))?,
            )),
            (DecodedProperty::Str(name, s), ScalarValueEncoder::String(enc)) => {
                let str_enc = match enc {
                    StrEncoder::Plain { string_lengths } => OwnedStream::encode_strings_with_type(
                        &s.dense_values(),
                        string_lengths,
                        LengthType::VarBinary,
                        DictionaryType::None,
                    )?,
                    StrEncoder::Fsst(enc) => OwnedStream::encode_strings_fsst_with_type(
                        &s.dense_values(),
                        enc,
                        DictionaryType::Single,
                    )?,
                };
                Ok(Self::Str(
                    OwnedName(name.clone()),
                    OwnedEncodedPresence(presence),
                    str_enc,
                ))
            }
            (DecodedProperty::SharedDict(..), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(
                v.kind_name(),
                e.name(),
            ))?,
        }
    }
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl<'a> FromEncoded<'a> for DecodedProperty<'a> {
    type Input = EncodedProperty<'a>;

    fn from_encoded(v: EncodedProperty<'_>) -> Result<Self, MltError> {
        let name = v.name().to_string();
        Ok(match v {
            EncodedProperty::Bool(_, presence, data) => {
                Self::Bool(name, apply_present(presence.0, data.decode_bools()?)?)
            }
            EncodedProperty::I8(_, presence, data) => {
                Self::I8(name, apply_present(presence.0, data.decode_i8s()?)?)
            }
            EncodedProperty::U8(_, presence, data) => {
                Self::U8(name, apply_present(presence.0, data.decode_u8s()?)?)
            }
            EncodedProperty::I32(_, presence, data) => {
                Self::I32(name, apply_present(presence.0, data.decode_i32s()?)?)
            }
            EncodedProperty::U32(_, presence, data) => {
                Self::U32(name, apply_present(presence.0, data.decode_u32s()?)?)
            }
            EncodedProperty::I64(_, presence, data) => {
                Self::I64(name, apply_present(presence.0, data.decode_i64()?)?)
            }
            EncodedProperty::U64(_, presence, data) => {
                Self::U64(name, apply_present(presence.0, data.decode_u64()?)?)
            }
            EncodedProperty::F32(_, presence, data) => {
                Self::F32(name, apply_present(presence.0, data.decode_f32()?)?)
            }
            EncodedProperty::F64(_, presence, data) => {
                Self::F64(name, apply_present(presence.0, data.decode_f64()?)?)
            }
            EncodedProperty::Str(_, presence, s) => {
                Self::Str(name, decode_strings_with_presence(presence, s)?)
            }
            EncodedProperty::SharedDict(_, sd, children) => {
                let (shared_dict, items) = decode_shared_dict(&sd, &children)?;
                Self::SharedDict(name, shared_dict, items)
            }
        })
    }
}
