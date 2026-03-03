pub mod strings;

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::MltError::{IntegerOverflow, NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::utils::{BinarySerializer as _, FmtOptVec, apply_present, f32_to_json, f64_to_json};
pub use crate::v01::property::strings::{
    EncodedStructChild, EncodedStructProp, SharedDictChild, SharedDictEncoder,
    SharedDictionaryGroup, StrEncoder, decode_string_streams, decode_struct_children,
    encode_shared_dictionary,
};
use crate::v01::{
    ColumnType, DictionaryType, FsstStrEncoder, IntEncoder, LengthType, OwnedStream, Stream,
};
use crate::{FromDecoded, MltError, impl_encodable};

/// Property representation, either encoded or decoded
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
        use {AproxPropertyType as T, OwnedEncodedPropValue as Enc, PropValue as Dec};
        match self {
            Self::Encoded(r) => match &r.value {
                Enc::Bool(_) => T::Bool,
                Enc::I8(_) | Enc::I32(_) | Enc::I64(_) | Enc::U8(_) | Enc::U32(_) | Enc::U64(_) => {
                    T::Integer
                }
                Enc::F32(_) | Enc::F64(_) => T::Float,
                Enc::Str(_) => T::String,
                Enc::Struct(_) => T::Struct,
            },
            Self::Decoded(r) => match &r.values {
                Dec::Bool(_) => T::Bool,
                Dec::I8(_) | Dec::I32(_) | Dec::I64(_) | Dec::U8(_) | Dec::U32(_) | Dec::U64(_) => {
                    T::Integer
                }
                Dec::F32(_) | Dec::F64(_) => T::Float,
                Dec::Str(_) => T::String,
                Dec::Struct => T::Struct,
            },
        }
    }
}

pub enum AproxPropertyType {
    Bool,
    Integer,
    Float,
    String,
    Struct,
}

/// Unparsed property data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedProperty<'a> {
    name: &'a str,
    optional: Option<Stream<'a>>,
    pub(crate) value: EncodedPropValue<'a>,
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

        // Struct children metadata must be written inline here so subsequent column
        // metadata offsets remain correct.
        if let OwnedEncodedPropValue::Struct(s) = &self.value {
            writer.write_varint(u64::try_from(s.children.len())?)?;
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
                        // warning: the _usize is necessary, we don't want to zigzag
                        writer.write_varint(2_usize)?; // stream_count => data and option
                        writer.write_boolean_stream(opt)?;
                    } else {
                        // warning: the _usize is necessary, we don't want to zigzag
                        writer.write_varint(1_usize)?; // stream_count => only data stream
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

/// Decoded property value types
#[derive(Clone, Default, PartialEq)]
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
            PropValue::Struct => "struct",
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

/// Instruction for how to encode a single decoded property when batch-encoding a
/// [`Vec<DecodedProperty>`] via [`FromDecoded`].
///
/// Each instruction corresponds positionally to one entry in the input slice.
/// Instructions sharing the same [`PropertyEncoder::shared_dict`] are grouped
/// into a single struct column with a shared dictionary.
/// The struct column appears in the output at the position of its first child in the input.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyEncoder {
    /// Encode this property as a standalone scalar column.
    Scalar(ScalarEncoder),
    /// Encode this property as a child field of a shared-dictionary struct column.
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

/// How to encode properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum ScalarValueEncoder {
    Int(IntEncoder),
    String(StrEncoder),
    Float,
    Bool,
    Struct,
}
impl ScalarValueEncoder {
    fn name(self) -> &'static str {
        match self {
            ScalarValueEncoder::Int(_) => "int",
            ScalarValueEncoder::String(_) => "string",
            ScalarValueEncoder::Float => "float",
            ScalarValueEncoder::Bool => "bool",
            ScalarValueEncoder::Struct => "struct",
        }
    }
}

impl PropertyEncoder {
    pub fn shared_dict(
        struct_name: impl Into<String>,
        child_name: impl Into<String>,
        optional: PresenceStream,
        offset: IntEncoder,
    ) -> Self {
        Self::SharedDict(SharedDictEncoder {
            struct_name: struct_name.into(),
            child_name: child_name.into(),
            optional,
            offset,
        })
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

pub struct MultiPropertyEncoder {
    properties: Vec<PropertyEncoder>,
    shared_dicts: HashMap<String, StrEncoder>,
}

impl MultiPropertyEncoder {
    #[must_use]
    pub fn new(
        properties: Vec<PropertyEncoder>,
        shared_dicts: HashMap<String, StrEncoder>,
    ) -> Self {
        Self {
            properties,
            shared_dicts,
        }
    }
}

impl FromDecoded<'_> for Vec<OwnedEncodedProperty> {
    type Input = Vec<DecodedProperty>;
    type Encoder = MultiPropertyEncoder;

    fn from_decoded(properties: &Self::Input, encoders: Self::Encoder) -> Result<Self, MltError> {
        let prop_encs = encoders.properties;
        if properties.len() != prop_encs.len() {
            return Err(MltError::EncodingInstructionCountMismatch {
                input_len: properties.len(),
                config_len: prop_encs.len(),
            });
        }

        // Pass 1: collect struct child groups, preserving first-occurrence order of struct names.
        let mut struct_groups: HashMap<String, SharedDictionaryGroup> = HashMap::new();
        let mut struct_order: Vec<String> = Vec::new();

        for (prop, encoder) in properties.iter().zip(&prop_encs) {
            if let PropertyEncoder::SharedDict(enc) = encoder {
                let SharedDictEncoder {
                    struct_name,
                    child_name,
                    optional,
                    offset,
                } = enc;
                let Some(&shared) = encoders.shared_dicts.get(struct_name) else {
                    return Err(MltError::MissingStructEncoderForStruct);
                };
                let group = struct_groups.entry(struct_name.clone()).or_insert_with(|| {
                    struct_order.push(struct_name.clone());
                    SharedDictionaryGroup {
                        shared,
                        children: Vec::new(),
                    }
                });
                group.children.push(SharedDictChild {
                    prop_value: prop,
                    prop_name: child_name.clone(),
                    optional: *optional,
                    offset: *offset,
                });
            }
        }

        // Pre-encode all struct groups.
        let mut encoded_structs: HashMap<String, OwnedEncodedProperty> = HashMap::new();
        for struct_name in &struct_order {
            let group = &struct_groups[struct_name];
            encoded_structs.insert(
                struct_name.clone(),
                encode_shared_dictionary(struct_name, group)?,
            );
        }

        // Pass 2: emit properties in input order; structs appear at their first child's position.
        let mut result = Vec::new();
        let mut emitted_structs = HashSet::new();

        for (prop, encoder) in properties.iter().zip(prop_encs) {
            match encoder {
                PropertyEncoder::Scalar(enc) => {
                    result.push(OwnedEncodedProperty::from_decoded(prop, enc)?);
                }
                PropertyEncoder::SharedDict(SharedDictEncoder { struct_name, .. }) => {
                    if emitted_structs.insert(struct_name.clone()) {
                        result.push(
                            encoded_structs
                                .remove(&struct_name)
                                .expect("pre-encoded in pass 1"),
                        );
                    }
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
        use {OwnedEncodedPropValue as EncVal, PropValue as Val};
        let optional = if encoder.optional == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.values.as_presence_stream()?;
            Some(OwnedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        let value = match (&decoded.values, encoder.value) {
            (Val::Bool(b), ScalarValueEncoder::Bool) => {
                EncVal::Bool(OwnedStream::encode_bools(&unapply_presence(b))?)
            }
            (Val::I8(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I8(OwnedStream::encode_i8s(&vals, enc)?)
            }
            (Val::U8(u), ScalarValueEncoder::Int(enc)) => {
                let values = unapply_presence(u);
                EncVal::U8(OwnedStream::encode_u8s(&values, enc)?)
            }
            (Val::I32(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I32(OwnedStream::encode_i32s(&vals, enc)?)
            }
            (Val::U32(u), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(u);
                EncVal::U32(OwnedStream::encode_u32s(&vals, enc)?)
            }
            (Val::I64(i), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(i);
                EncVal::I64(OwnedStream::encode_i64s(&vals, enc)?)
            }
            (Val::U64(u), ScalarValueEncoder::Int(enc)) => {
                let vals = unapply_presence(u);
                EncVal::U64(OwnedStream::encode_u64s(&vals, enc)?)
            }
            (Val::F32(f), ScalarValueEncoder::Float) => {
                let vals = unapply_presence(f);
                EncVal::F32(OwnedStream::encode_f32(&vals)?)
            }
            (Val::F64(d), ScalarValueEncoder::Float) => {
                let vals = unapply_presence(d);
                EncVal::F64(OwnedStream::encode_f64(&vals)?)
            }
            (Val::Str(s), ScalarValueEncoder::String(enc)) => {
                let values = unapply_presence(s);
                let streams = match enc {
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
                EncVal::Str(streams)
            }
            (Val::Struct, ScalarValueEncoder::Struct) => {
                Err(NotImplemented("struct property encoding"))?
            }
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.name(), e.name()))?,
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
            EncVal::F64(s) => Val::F64(apply_present(present, s.decode_f64()?)?),
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
    use crate::v01::{LogicalEncoder, PhysicalEncoder};

    fn roundtrip(decoded: &DecodedProperty, encoder: ScalarEncoder) -> DecodedProperty {
        let encoded_prop =
            OwnedEncodedProperty::from_decoded(decoded, encoder).expect("encoding failed");
        DecodedProperty::from_encoded(borrowme::borrow(&encoded_prop)).expect("decoding failed")
    }

    /// Excludes `FastPFOR` because it only handles 32-bit integers.
    fn physical_no_fastpfor() -> impl Strategy<Value = PhysicalEncoder> {
        any::<PhysicalEncoder>().prop_filter("no FastPFOR", |v| *v != PhysicalEncoder::FastPFOR)
    }

    /// Every integer type is encoded the same way; only the Rust primitive type and the
    /// set of valid physical encoders differ.  A macro avoids 12 near-identical blocks
    /// while keeping the two interesting axes (nullable presence stream vs. absent stream)
    /// as separate test functions so failures are easy to locate.
    ///
    /// `Absent` mode drops `None` entries during encoding and has no presence stream on
    /// the wire, so only all-`Some` inputs are generated for those variants.
    macro_rules! integer_roundtrip_proptests {
        ($present:ident, $absent:ident, $variant:ident, $ty:ty, $physical:expr) => {
            proptest! {
                #[test]
                fn $present(
                    values in prop::collection::vec(prop::option::of(any::<$ty>()), 0..100),
                    logical in any::<LogicalEncoder>(),
                    physical in $physical,
                ) {
                    let prop = DecodedProperty {
                        name: "x".to_string(),
                        values: PropValue::$variant(values),
                    };
                    let enc = ScalarEncoder::int(PresenceStream::Present, IntEncoder::new(logical, physical));
                    prop_assert_eq!(roundtrip(&prop, enc), prop);
                }

                #[test]
                fn $absent(
                    values in prop::collection::vec(any::<$ty>(), 0..100),
                    logical in any::<LogicalEncoder>(),
                    physical in $physical,
                ) {
                    let opt: Vec<Option<$ty>> = values.into_iter().map(Some).collect();
                    let prop = DecodedProperty {
                        name: "x".to_string(),
                        values: PropValue::$variant(opt),
                    };
                    let enc = ScalarEncoder::int(PresenceStream::Absent, IntEncoder::new(logical, physical));
                    prop_assert_eq!(roundtrip(&prop, enc), prop);
                }
            }
        };
    }

    // I8, U8, I32, U32 — all physical encoders are valid.
    integer_roundtrip_proptests!(i8_present, i8_absent, I8, i8, any::<PhysicalEncoder>());
    integer_roundtrip_proptests!(u8_present, u8_absent, U8, u8, any::<PhysicalEncoder>());
    integer_roundtrip_proptests!(i32_present, i32_absent, I32, i32, any::<PhysicalEncoder>());
    integer_roundtrip_proptests!(u32_present, u32_absent, U32, u32, any::<PhysicalEncoder>());
    // FastPFOR does not support 64-bit integers.
    integer_roundtrip_proptests!(i64_present, i64_absent, I64, i64, physical_no_fastpfor());
    integer_roundtrip_proptests!(u64_present, u64_absent, U64, u64, physical_no_fastpfor());

    // Bool values are packed into bitmaps; logical and physical encoder settings
    // have no effect on the encoding path.  A concrete test pins the specific
    // present/None interleaving; the proptest covers the nullable case broadly.
    #[test]
    fn bool_specific_values() {
        // Verifies that None entries survive the presence stream intact.
        let prop = DecodedProperty {
            name: "active".to_string(),
            values: PropValue::Bool(vec![Some(true), None, Some(false), Some(true), None]),
        };
        assert_eq!(
            roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
            prop
        );
    }
    #[test]
    fn bool_all_null() {
        let prop = DecodedProperty {
            name: "active".to_string(),
            values: PropValue::Bool(vec![None, None, None]),
        };
        assert_eq!(
            roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
            prop
        );
    }

    proptest! {
        // Encoder settings are ignored for bools; only presence/absence of None matters.
        #[test]
        fn bool_roundtrip(
            values in prop::collection::vec(prop::option::of(any::<bool>()), 0..100),
        ) {
            let prop = DecodedProperty {
                name: "flag".to_string(),
                values: PropValue::Bool(values),
            };
            let enc = ScalarEncoder::bool(PresenceStream::Present);
            prop_assert_eq!(roundtrip(&prop, enc), prop);
        }
    }

    // F32 and F64 are stored verbatim; logical and physical encoders are ignored.
    // NaN is excluded because NaN != NaN
    proptest! {
        #[test]
        fn f32_roundtrip(
            values in prop::collection::vec(
                prop::option::of(any::<f32>().prop_filter("no NaN", |f| !f.is_nan())),
                0..100,
            ),
        ) {
            let prop = DecodedProperty {
                name: "score".to_string(),
                values: PropValue::F32(values),
            };
            let enc = ScalarEncoder::float(PresenceStream::Present);
            prop_assert_eq!(roundtrip(&prop, enc), prop);
        }

        #[test]
        fn f64_roundtrip(
            values in prop::collection::vec(
                prop::option::of(
                    any::<f64>().prop_filter("no NaN", |f| !f.is_nan()),
                ),
                0..100,
            ),
        ) {
            let prop = DecodedProperty {
                name: "score".to_string(),
                values: PropValue::F64(values),
            };
            let enc = ScalarEncoder::float(PresenceStream::Present);
            prop_assert_eq!(roundtrip(&prop, enc), prop);
        }
    }
}
