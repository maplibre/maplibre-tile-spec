mod optimizer;
pub mod strings;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::MltError::{NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::utils::{
    BinarySerializer as _, FmtOptVec, apply_present, checked_sum2, checked_sum3, f32_to_json,
    f64_to_json,
};
pub use crate::v01::property::optimizer::PropertyOptimizer;
pub use crate::v01::property::strings::{
    EncodedSharedDictProp, EncodedStrProp, EncodedStructChild, SharedDictEncoder,
    SharedDictItemEncoder, StrEncoder, decode_shared_dict, decode_strings, encode_shared_dict_prop,
};
use crate::v01::{
    ColumnType, DictionaryType, FsstStrEncoder, IntEncoder, LengthType, OwnedStream, Stream,
};
use crate::{FromDecoded, MltError, impl_encodable};

/// Property representation, either encoded or decoded
#[expect(clippy::large_enum_variant)]
#[borrowme]
#[derive(Debug, PartialEq)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
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

    #[must_use]
    pub fn approx_type(&self) -> AproxPropertyType {
        use AproxPropertyType as T;
        use OwnedEncodedPropValue as Enc;
        use PropValue as Dec;
        match self {
            Self::Encoded(r) => match &r.value {
                Enc::Bool(..) => T::Bool,
                Enc::I8(..)
                | Enc::I32(..)
                | Enc::I64(..)
                | Enc::U8(..)
                | Enc::U32(..)
                | Enc::U64(..) => T::Integer,
                Enc::F32(..) | Enc::F64(..) => T::Float,
                Enc::Str(..) => T::String,
                Enc::SharedDict(..) => T::SharedDict,
            },
            Self::Decoded(r) => match &r.values {
                Dec::Bool(..) => T::Bool,
                Dec::I8(..)
                | Dec::I32(..)
                | Dec::I64(..)
                | Dec::U8(..)
                | Dec::U32(..)
                | Dec::U64(..) => T::Integer,
                Dec::F32(..) | Dec::F64(..) => T::Float,
                Dec::Str(..) => T::String,
                Dec::SharedDict(_) => T::SharedDict,
            },
        }
    }
}

pub enum AproxPropertyType {
    Bool,
    Integer,
    Float,
    String,
    SharedDict,
}

/// Unparsed property data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedProperty<'a> {
    name: &'a str,
    pub(crate) value: EncodedPropValue<'a>,
}

impl<'a> EncodedProperty<'a> {
    pub(crate) fn new(name: &'a str, value: EncodedPropValue<'a>) -> Self {
        Self { name, value }
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.value.for_each_stream(cb);
    }
}

impl OwnedEncodedProperty {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        // type
        match &self.value {
            OwnedEncodedPropValue::Bool(Some(_), _) => ColumnType::OptBool.write_to(writer)?,
            OwnedEncodedPropValue::Bool(None, _) => ColumnType::Bool.write_to(writer)?,
            OwnedEncodedPropValue::I8(Some(_), _) => ColumnType::OptI8.write_to(writer)?,
            OwnedEncodedPropValue::I8(None, _) => ColumnType::I8.write_to(writer)?,
            OwnedEncodedPropValue::U8(Some(_), _) => ColumnType::OptU8.write_to(writer)?,
            OwnedEncodedPropValue::U8(None, _) => ColumnType::U8.write_to(writer)?,
            OwnedEncodedPropValue::I32(Some(_), _) => ColumnType::OptI32.write_to(writer)?,
            OwnedEncodedPropValue::I32(None, _) => ColumnType::I32.write_to(writer)?,
            OwnedEncodedPropValue::U32(Some(_), _) => ColumnType::OptU32.write_to(writer)?,
            OwnedEncodedPropValue::U32(None, _) => ColumnType::U32.write_to(writer)?,
            OwnedEncodedPropValue::I64(Some(_), _) => ColumnType::OptI64.write_to(writer)?,
            OwnedEncodedPropValue::I64(None, _) => ColumnType::I64.write_to(writer)?,
            OwnedEncodedPropValue::U64(Some(_), _) => ColumnType::OptU64.write_to(writer)?,
            OwnedEncodedPropValue::U64(None, _) => ColumnType::U64.write_to(writer)?,
            OwnedEncodedPropValue::F32(Some(_), _) => ColumnType::OptF32.write_to(writer)?,
            OwnedEncodedPropValue::F32(None, _) => ColumnType::F32.write_to(writer)?,
            OwnedEncodedPropValue::F64(Some(_), _) => ColumnType::OptF64.write_to(writer)?,
            OwnedEncodedPropValue::F64(None, _) => ColumnType::F64.write_to(writer)?,
            OwnedEncodedPropValue::Str(Some(_), _) => ColumnType::OptStr.write_to(writer)?,
            OwnedEncodedPropValue::Str(None, _) => ColumnType::Str.write_to(writer)?,
            OwnedEncodedPropValue::SharedDict(_) => ColumnType::SharedDict.write_to(writer)?,
        }

        // name
        writer.write_string(&self.name)?;

        // Struct children metadata must be written inline here so subsequent column
        // metadata offsets remain correct.
        if let OwnedEncodedPropValue::SharedDict(s) = &self.value {
            writer.write_varint(u64::try_from(s.children().len())?)?;
            for child in s.children() {
                child.write_columns_meta_to(writer)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        use OwnedEncodedPropValue as Val;

        match &self.value {
            Val::Bool(opt, val) => {
                writer.write_optional_stream(opt.as_ref())?;
                writer.write_boolean_stream(val)?;
            }
            Val::I8(opt, val)
            | Val::U8(opt, val)
            | Val::I32(opt, val)
            | Val::U32(opt, val)
            | Val::I64(opt, val)
            | Val::U64(opt, val)
            | Val::F32(opt, val)
            | Val::F64(opt, val) => {
                writer.write_optional_stream(opt.as_ref())?;
                writer.write_stream(val)?;
            }
            Val::Str(opt, encoding) => {
                let streams = encoding.streams();
                let stream_count =
                    checked_sum2(u64::try_from(streams.len())?, u64::from(opt.is_some()))?;
                writer.write_varint(stream_count)?;
                writer.write_optional_stream(opt.as_ref())?;
                for s in streams {
                    writer.write_stream(s)?;
                }
            }
            Val::SharedDict(s) => {
                let dict_streams = s.dict_streams();
                let children = s.children();
                writer.write_varint(checked_sum3(
                    dict_streams.len(),
                    children.len(),
                    children.iter().filter(|c| c.optional.is_some()).count(),
                )?)?;
                for stream in dict_streams {
                    writer.write_stream(stream)?;
                }
                for child in children {
                    // stream_count => data + (0 or 1 for optional stream)
                    // must be usize because we don't want to zigzag
                    writer.write_varint(1 + usize::from(child.optional.is_some()))?;
                    writer.write_optional_stream(child.optional.as_ref())?;
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
        Self {
            name: String::default(),
            value: OwnedEncodedPropValue::Bool(None, OwnedStream::empty_without_encoding()),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedEncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: DecodedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Self =
            Self::from_decoded(&decoded, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(prop)
    }
}

/// A sequence of encoded property values of various types
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedPropValue<'a> {
    Bool(Option<Stream<'a>>, Stream<'a>),
    I8(Option<Stream<'a>>, Stream<'a>),
    U8(Option<Stream<'a>>, Stream<'a>),
    I32(Option<Stream<'a>>, Stream<'a>),
    U32(Option<Stream<'a>>, Stream<'a>),
    I64(Option<Stream<'a>>, Stream<'a>),
    U64(Option<Stream<'a>>, Stream<'a>),
    F32(Option<Stream<'a>>, Stream<'a>),
    F64(Option<Stream<'a>>, Stream<'a>),
    Str(Option<Stream<'a>>, EncodedStrProp<'a>),
    SharedDict(EncodedSharedDictProp<'a>),
}

impl Analyze for EncodedPropValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(opt, stream)
            | Self::I8(opt, stream)
            | Self::U8(opt, stream)
            | Self::I32(opt, stream)
            | Self::U32(opt, stream)
            | Self::I64(opt, stream)
            | Self::U64(opt, stream)
            | Self::F32(opt, stream)
            | Self::F64(opt, stream) => {
                opt.for_each_stream(cb);
                stream.for_each_stream(cb);
            }
            Self::Str(opt, encoding) => {
                opt.for_each_stream(cb);
                for s in encoding.streams() {
                    cb(s);
                }
            }
            Self::SharedDict(sp) => {
                let streams = sp.streams();
                for s in &streams {
                    cb(s);
                }
            }
        }
    }
}

/// Decoded property values as a name and a vector of optional typed values
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
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

/// A single sub-property within a shared dictionary decoded value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct SharedDictItem {
    /// The suffix name of this sub-property (appended to parent struct name).
    pub suffix: String,
    /// The string values for each feature (None = null).
    pub values: Vec<Option<String>>,
}

/// Decoded property value types
#[derive(Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
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
    /// Shared dictionary with multiple string sub-properties.
    SharedDict(Vec<SharedDictItem>),
}

impl Default for PropValue {
    fn default() -> Self {
        Self::Bool(Vec::new())
    }
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
            PropValue::SharedDict(_) => Err(NotImplemented("presence stream for shared dict"))?,
        })
    }

    fn name(&self) -> &'static str {
        match self {
            PropValue::Bool(_) => "bool",
            PropValue::I8(_) => "i8",
            PropValue::U8(_) => "u8",
            PropValue::I32(_) => "i32",
            PropValue::U32(_) => "u32",
            PropValue::I64(_) => "i64",
            PropValue::U64(_) => "u64",
            PropValue::F32(_) => "f32",
            PropValue::F64(_) => "f64",
            PropValue::Str(_) => "str",
            PropValue::SharedDict(_) => "shared_dict",
        }
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
            Self::SharedDict(items) => items
                .iter()
                .map(|item| item.values.collect_statistic(stat))
                .sum(),
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
            Self::SharedDict(items) => f.debug_tuple("SharedDict").field(items).finish(),
        }
    }
}

impl PropValue {
    /// Convert the value at index `i` to a [`serde_json::Value`]
    #[must_use]
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
            Self::F64(v) => v[i].map(f64_to_json),
            Self::Str(v) => v[i].as_ref().map(|s| Value::String(s.clone())),
            Self::SharedDict(items) => {
                let mut obj = serde_json::Map::new();
                for item in items {
                    if let Some(ref s) = item.values[i] {
                        obj.insert(item.suffix.clone(), Value::String(s.clone()));
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

impl_decodable!(Property<'a>, EncodedProperty<'a>, DecodedProperty);
impl_encodable!(OwnedProperty, DecodedProperty, OwnedEncodedProperty);

impl<'a> From<EncodedProperty<'a>> for Property<'a> {
    fn from(value: EncodedProperty<'a>) -> Self {
        Self::Encoded(value)
    }
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn new_encoded(name: &'a str, value: EncodedPropValue<'a>) -> Self {
        Self::Encoded(EncodedProperty { name, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedProperty, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedProperty::from_encoded(v)?,
            Self::Decoded(v) => v,
        })
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
    pub optional: PresenceStream,
    pub value: ScalarValueEncoder,
}

impl ScalarEncoder {
    #[must_use]
    pub fn str(optional: PresenceStream, string_lengths: IntEncoder) -> Self {
        let enc = StrEncoder::Plain { string_lengths };
        Self {
            optional,
            value: ScalarValueEncoder::String(enc),
        }
    }
    /// Create a property encoder with integer encoding
    #[must_use]
    pub fn int(optional: PresenceStream, enc: IntEncoder) -> Self {
        Self {
            optional,
            value: ScalarValueEncoder::Int(enc),
        }
    }
    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn str_fsst(
        optional: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
    ) -> Self {
        let enc = FsstStrEncoder {
            symbol_lengths,
            dict_lengths,
        };
        Self {
            optional,
            value: ScalarValueEncoder::String(StrEncoder::Fsst(enc)),
        }
    }
    /// Create a property encoder for boolean values
    #[must_use]
    pub fn bool(optional: PresenceStream) -> Self {
        Self {
            optional,
            value: ScalarValueEncoder::Bool,
        }
    }
    /// Create a property encoder for float values
    #[must_use]
    pub fn float(optional: PresenceStream) -> Self {
        Self {
            optional,
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
            ScalarValueEncoder::Int(_) => "int",
            ScalarValueEncoder::String(_) => "string",
            ScalarValueEncoder::Float => "float",
            ScalarValueEncoder::Bool => "bool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PresenceStream {
    /// Attaches a nullability stream
    Present,
    /// If there are nulls, drop them
    Absent,
}

impl FromDecoded<'_> for Vec<OwnedEncodedProperty> {
    type Input = Vec<DecodedProperty>;
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
                    let PropValue::SharedDict(items) = &prop.values else {
                        return Err(UnsupportedPropertyEncoderCombination(
                            prop.values.name(),
                            "SharedDict",
                        ));
                    };
                    result.push(OwnedEncodedProperty {
                        name: prop.name.clone(),
                        value: encode_shared_dict_prop(items, &enc)?,
                    });
                }
            }
        }

        Ok(result)
    }
}

impl FromDecoded<'_> for OwnedEncodedProperty {
    type Input = DecodedProperty;
    type Encoder = ScalarEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError> {
        use OwnedEncodedPropValue as EncVal;
        use PropValue as Val;
        let optional = if encoder.optional == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.values.as_presence_stream()?;
            Some(OwnedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        let value = match (&decoded.values, encoder.value) {
            (Val::Bool(b), ScalarValueEncoder::Bool) => {
                EncVal::Bool(optional, OwnedStream::encode_bools(&unapply_presence(b))?)
            }
            (Val::I8(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I8(optional, OwnedStream::encode_i8s(&vals, enc)?)
            }
            (Val::U8(u), ScalarValueEncoder::Int(enc)) => {
                let values = unapply_presence(u);
                EncVal::U8(optional, OwnedStream::encode_u8s(&values, enc)?)
            }
            (Val::I32(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I32(optional, OwnedStream::encode_i32s(&vals, enc)?)
            }
            (Val::U32(u), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(u);
                EncVal::U32(optional, OwnedStream::encode_u32s(&vals, enc)?)
            }
            (Val::I64(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I64(optional, OwnedStream::encode_i64s(&vals, enc)?)
            }
            (Val::U64(u), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(u);
                EncVal::U64(optional, OwnedStream::encode_u64s(&vals, enc)?)
            }
            (Val::F32(f), ScalarValueEncoder::Float) => {
                let vals = unapply_presence(f);
                EncVal::F32(optional, OwnedStream::encode_f32(&vals)?)
            }
            (Val::F64(d), ScalarValueEncoder::Float) => {
                let vals = unapply_presence(d);
                EncVal::F64(optional, OwnedStream::encode_f64(&vals)?)
            }
            (Val::Str(s), ScalarValueEncoder::String(enc)) => {
                let values = unapply_presence(s);
                let encoding = match enc {
                    StrEncoder::Plain { string_lengths } => OwnedStream::encode_strings_with_type(
                        &values,
                        string_lengths,
                        LengthType::VarBinary,
                        DictionaryType::None,
                    )?,
                    StrEncoder::Fsst(enc) => OwnedStream::encode_strings_fsst_with_type(
                        &values,
                        enc,
                        DictionaryType::Single,
                    )?,
                };
                EncVal::Str(optional, encoding)
            }
            (Val::SharedDict(_), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.name(), e.name()))?,
        };

        Ok(Self {
            name: decoded.name.clone(),
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
        use EncodedPropValue as EncVal;
        use PropValue as Val;
        let values = match v.value {
            EncVal::Bool(o, s) => Val::Bool(apply_present(o, s.decode_bools()?)?),
            EncVal::I8(o, s) => Val::I8(apply_present(o, s.decode_i8s()?)?),
            EncVal::U8(o, s) => Val::U8(apply_present(o, s.decode_u8s()?)?),
            EncVal::I32(o, s) => Val::I32(apply_present(o, s.decode_i32s()?)?),
            EncVal::U32(o, s) => Val::U32(apply_present(o, s.decode_u32s()?)?),
            EncVal::I64(o, s) => Val::I64(apply_present(o, s.decode_i64()?)?),
            EncVal::U64(o, s) => Val::U64(apply_present(o, s.decode_u64()?)?),
            EncVal::F32(o, s) => Val::F32(apply_present(o, s.decode_f32()?)?),
            EncVal::F64(o, s) => Val::F64(apply_present(o, s.decode_f64()?)?),
            EncVal::Str(o, s) => Val::Str(apply_present(o, decode_strings(s)?)?),
            EncVal::SharedDict(sd) => decode_shared_dict(&sd)?,
        };
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values,
        })
    }
}
