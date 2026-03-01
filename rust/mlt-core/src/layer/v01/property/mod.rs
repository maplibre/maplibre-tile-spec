pub(crate) mod decode;

use std::fmt::{self, Debug};
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::MltError::{IntegerOverflow, NotImplemented};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::utils::{BinarySerializer as _, FmtOptVec, apply_present, f32_to_json, f64_to_json};
use crate::v01::property::decode::{decode_string_streams, decode_struct_children};
use crate::v01::{
    ColumnType, DictionaryType, Encoder, LengthType, LogicalEncoder, OffsetType, OwnedStream,
    PhysicalEncoder, Stream, StreamType,
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
        let encoder: PropertyEncoder = u.arbitrary()?;
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

/// How to encode string properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StringEncoding {
    /// Plain encoding: length stream + data stream
    #[default]
    Plain,
    /// FSST encoding: symbol lengths + symbol table + value lengths + compressed corpus
    ///
    /// Note: The FSST algorithm implementation may differ from Java's, so the
    /// compressed output may not be byte-for-byte identical. Both implementations
    /// are semantically compatible and can decode each other's output.
    Fsst,
}

/// How to encode properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct PropertyEncoder {
    pub optional: PresenceStream,
    pub logical: LogicalEncoder,
    pub physical: PhysicalEncoder,
    pub string_encoding: StringEncoding,
}
impl PropertyEncoder {
    #[must_use]
    pub fn new(
        optional: PresenceStream,
        logical: LogicalEncoder,
        physical: PhysicalEncoder,
    ) -> Self {
        Self {
            optional,
            logical,
            physical,
            string_encoding: StringEncoding::Plain,
        }
    }

    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn with_fsst(
        optional: PresenceStream,
        logical: LogicalEncoder,
        physical: PhysicalEncoder,
    ) -> Self {
        Self {
            optional,
            logical,
            physical,
            string_encoding: StringEncoding::Fsst,
        }
    }

    #[must_use]
    pub fn encoder(self) -> Encoder {
        Encoder::new(self.logical, self.physical)
    }
}

/// Instruction for how to encode a single decoded property when batch-encoding a
/// [`Vec<DecodedProperty>`] via [`FromDecoded`].
///
/// Each instruction corresponds positionally to one entry in the input slice.
/// Instructions sharing the same [`EncodingInstruction::StructChild::struct_name`] are grouped
/// into a single struct column with a shared dictionary.
/// The struct column appears in the output at the position of its first child in the input.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingInstruction {
    /// Encode this property as a standalone scalar column.
    Scalar(PropertyEncoder),
    /// Encode this property as a child field of a shared-dictionary struct column.
    StructChild {
        /// Name of the parent struct column.
        ///
        /// All instructions with the same value are grouped into one struct column.
        struct_name: String,
        /// Name of this field within the struct column.
        child_name: String,
        /// Encoder used for the offset-index stream of this child.
        encoder: PropertyEncoder,
    },
}

impl EncodingInstruction {
    pub fn struct_child(
        struct_name: impl Into<String>,
        child_name: impl Into<String>,
        encoder: PropertyEncoder,
    ) -> Self {
        Self::StructChild {
            struct_name: struct_name.into(),
            child_name: child_name.into(),
            encoder,
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
    type Encoder = Vec<EncodingInstruction>;

    fn from_decoded(input: &Self::Input, config: Self::Encoder) -> Result<Self, MltError> {
        use std::collections::{HashMap, HashSet};

        if input.len() != config.len() {
            return Err(MltError::EncodingInstructionCountMismatch {
                input_len: input.len(),
                config_len: config.len(),
            });
        }

        // Pass 1: collect struct child groups, preserving first-occurrence order of struct names.
        let mut struct_groups: HashMap<String, Vec<(&DecodedProperty, PropertyEncoder, String)>> =
            HashMap::new();
        let mut struct_order: Vec<String> = Vec::new();

        for (prop, instruction) in input.iter().zip(config.iter()) {
            if let EncodingInstruction::StructChild {
                struct_name,
                child_name,
                encoder,
            } = instruction
            {
                let group = struct_groups.entry(struct_name.clone()).or_insert_with(|| {
                    struct_order.push(struct_name.clone());
                    Vec::new()
                });
                group.push((prop, *encoder, child_name.clone()));
            }
        }

        // Pre-encode all struct groups.
        let mut encoded_structs: HashMap<String, OwnedEncodedProperty> = HashMap::new();
        for struct_name in &struct_order {
            let children = &struct_groups[struct_name];
            encoded_structs.insert(
                struct_name.clone(),
                encode_struct_property(struct_name.clone(), children)?,
            );
        }

        // Pass 2: emit properties in input order; structs appear at their first child's position.
        let mut result = Vec::new();
        let mut emitted_structs: HashSet<String> = HashSet::new();

        for (prop, instruction) in input.iter().zip(config.into_iter()) {
            match instruction {
                EncodingInstruction::Scalar(encoder) => {
                    result.push(OwnedEncodedProperty::from_decoded(prop, encoder)?);
                }
                EncodingInstruction::StructChild { struct_name, .. } => {
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

/// Encode a group of decoded string properties into a single struct column with a shared
/// dictionary. Children are ordered as provided.
fn encode_struct_property(
    name: String,
    children: &[(&DecodedProperty, PropertyEncoder, String)],
) -> Result<OwnedEncodedProperty, MltError> {
    // Build shared dictionary: unique strings in first-occurrence insertion order.
    let mut dict: Vec<String> = Vec::new();
    let mut dict_index: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for (prop, _, _) in children {
        match &prop.values {
            PropValue::Str(values) => {
                for s in values.iter().flatten() {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        dict_index.entry(s.clone())
                    {
                        let idx = u32::try_from(dict.len())?;
                        e.insert(idx);
                        dict.push(s.clone());
                    }
                }
            }
            _ => return Err(NotImplemented("generic prop_child encoding")),
        }
    }

    // Use the first child's string_encoding to decide how to encode the shared dictionary.
    // Use the first child's encoder for all shared-dict stream encoding choices.
    let first_encoder = children.first().map_or(
        PropertyEncoder::new(
            PresenceStream::Absent,
            LogicalEncoder::None,
            PhysicalEncoder::None,
        ),
        |(_, enc, _)| *enc,
    );
    let string_encoding = first_encoder.string_encoding;
    let dict_encoding = first_encoder.encoder();

    let dict_streams = match string_encoding {
        StringEncoding::Plain => OwnedStream::encode_strings_with_type(
            &dict,
            dict_encoding,
            LengthType::Dictionary,
            DictionaryType::Shared,
        )?,
        StringEncoding::Fsst => OwnedStream::encode_strings_fsst_with_type(
            &dict,
            dict_encoding,
            DictionaryType::Shared,
        )?,
    };

    // Encode each child column.
    let mut encoded_children = Vec::new();
    for (prop, encoder, child_name) in children {
        let PropValue::Str(values) = &prop.values else {
            return Err(NotImplemented("generic struct child encoding"));
        };

        // Presence stream: emit when the encoder requests it (PresenceStream::Present) or when
        // there are actual null values. This matches Java's behaviour of always writing a presence
        // bitmap for Present-encoded struct children even if no values are null.
        let has_nulls = values.iter().any(Option::is_none);
        let emit_presence = encoder.optional == PresenceStream::Present || has_nulls;
        let optional = if emit_presence {
            let present_bools: Vec<bool> = values.iter().map(Option::is_some).collect();
            Some(OwnedStream::encode_presence(&present_bools)?)
        } else {
            None
        };

        // Offset indices for non-null values only.
        let offsets: Vec<u32> = values
            .iter()
            .filter_map(|v| v.as_deref())
            .map(|s| dict_index[s])
            .collect();

        let data = OwnedStream::encode_u32s_of_type(
            &offsets,
            encoder.encoder(),
            StreamType::Offset(OffsetType::String),
        )?;

        encoded_children.push(OwnedEncodedStructChild {
            name: child_name.clone(),
            typ: if emit_presence {
                ColumnType::OptStr
            } else {
                ColumnType::Str
            },
            optional,
            data,
        });
    }

    Ok(OwnedEncodedProperty {
        name,
        optional: None,
        value: OwnedEncodedPropValue::Struct(OwnedEncodedStructProp {
            dict_streams,
            children: encoded_children,
        }),
    })
}

impl FromDecoded<'_> for OwnedEncodedProperty {
    type Input = DecodedProperty;
    type Encoder = PropertyEncoder;

    fn from_decoded(decoded: &Self::Input, config: Self::Encoder) -> Result<Self, MltError> {
        use {OwnedEncodedPropValue as EncVal, PropValue as Val};
        let optional = if config.optional == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.values.as_presence_stream()?;
            Some(OwnedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        let value = match &decoded.values {
            Val::Bool(b) => EncVal::Bool(OwnedStream::encode_bools(&unapply_presence(b))?),
            Val::I8(i) => {
                let vals = unapply_presence(i);
                EncVal::I8(OwnedStream::encode_i8s(&vals, config.encoder())?)
            }
            Val::U8(u) => {
                let values = unapply_presence(u);
                EncVal::U8(OwnedStream::encode_u8s(&values, config.encoder())?)
            }
            Val::I32(i) => {
                let vals = unapply_presence(i);
                EncVal::I32(OwnedStream::encode_i32s(&vals, config.encoder())?)
            }
            Val::U32(u) => {
                let vals = unapply_presence(u);
                EncVal::U32(OwnedStream::encode_u32s(&vals, config.encoder())?)
            }
            Val::I64(i) => {
                let vals = unapply_presence(i);
                EncVal::I64(OwnedStream::encode_i64s(&vals, config.encoder())?)
            }
            Val::U64(u) => {
                let vals = unapply_presence(u);
                EncVal::U64(OwnedStream::encode_u64s(&vals, config.encoder())?)
            }
            Val::F32(f) => {
                let vals = unapply_presence(f);
                EncVal::F32(OwnedStream::encode_f32(&vals)?)
            }
            Val::F64(f) => {
                // F64 is stored on the wire as F32 (lossy, matching the decoder).
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the decoder does it this way, but why?"
                )]
                let as_f32: Vec<f32> = unapply_presence(f).iter().map(|&v| v as f32).collect();
                EncVal::F64(OwnedStream::encode_f32(&as_f32)?)
            }
            Val::Str(s) => {
                let values = unapply_presence(s);
                let streams = match config.string_encoding {
                    StringEncoding::Plain => OwnedStream::encode_strings_with_type(
                        &values,
                        config.encoder(),
                        LengthType::VarBinary,
                        DictionaryType::None,
                    )?,
                    StringEncoding::Fsst => OwnedStream::encode_strings_fsst_with_type(
                        &values,
                        config.encoder(),
                        DictionaryType::Single,
                    )?,
                };
                EncVal::Str(streams)
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

    /// Default encoder: no logical/physical encoding, with a presence stream.
    fn default_encoder() -> PropertyEncoder {
        PropertyEncoder::new(
            PresenceStream::Present,
            LogicalEncoder::None,
            PhysicalEncoder::None,
        )
    }

    /// Build a `Vec<Option<String>>` where every entry is `Some`.
    fn strs(vals: &[&str]) -> Vec<Option<String>> {
        vals.iter().map(|v| Some(v.to_string())).collect()
    }

    /// Build a `Vec<Option<String>>` from a mix of `Some(&str)` and `None`.
    fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
        vals.iter().map(|v| v.map(ToString::to_string)).collect()
    }

    /// Build a [`DecodedProperty`] with [`PropValue::Str`].
    fn str_prop(name: &str, values: Vec<Option<String>>) -> DecodedProperty {
        DecodedProperty {
            name: name.to_string(),
            values: PropValue::Str(values),
        }
    }

    /// Borrow an encoded struct property and expand + fully decode all its children.
    fn expand_struct(prop: &OwnedEncodedProperty) -> Vec<DecodedProperty> {
        Property::from(borrowme::borrow(prop))
            .decode_expand()
            .expect("decode_expand failed")
            .into_iter()
            .map(|p| p.decode().expect("decode failed"))
            .collect()
    }

    /// Borrow and decode an encoded scalar property.
    fn decode_scalar(prop: &OwnedEncodedProperty) -> DecodedProperty {
        DecodedProperty::from_encoded(borrowme::borrow(prop)).expect("decode failed")
    }

    /// Encode a [`Vec<DecodedProperty>`] as a struct and decode it back via [`decode_expand`].
    fn struct_roundtrip(
        struct_name: &str,
        children: &[(&str, Vec<Option<String>>)],
        encoder: PropertyEncoder,
    ) -> Vec<DecodedProperty> {
        let decoded: Vec<DecodedProperty> = children
            .iter()
            .map(|(child_name, values)| str_prop(child_name, values.clone()))
            .collect();

        let instructions: Vec<EncodingInstruction> = children
            .iter()
            .map(|(child_name, _)| {
                EncodingInstruction::struct_child(struct_name, *child_name, encoder)
            })
            .collect();

        let encoded_property = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, instructions)
            .expect("encoding failed");
        assert_eq!(
            encoded_property.len(),
            1,
            "children must collapse to one struct column"
        );

        expand_struct(&encoded_property[0])
    }

    #[test]
    fn test_struct_shared_dict_present_roundtrip() {
        let de_values = opt_strs(&[Some("Berlin"), Some("München"), None]);
        let en_values = opt_strs(&[Some("Berlin"), None, Some("London")]);

        let result = struct_roundtrip(
            "name",
            &[(":de", de_values.clone()), (":en", en_values.clone())],
            default_encoder(),
        );

        assert_eq!(result.len(), 2);
        // decode_expand prefixes each child name with the struct column name
        assert_eq!(result[0].name, "name:de");
        assert_eq!(result[0].values, PropValue::Str(de_values));
        assert_eq!(result[1].name, "name:en");
        assert_eq!(result[1].values, PropValue::Str(en_values));
    }

    #[test]
    fn test_struct_shared_dict_no_nulls_roundtrip() {
        let de_values = strs(&["Berlin", "München", "Hamburg"]);
        let en_values = strs(&["Berlin", "Munich", "Hamburg"]);

        let result = struct_roundtrip(
            "name",
            &[(":de", de_values.clone()), (":en", en_values.clone())],
            default_encoder(),
        );

        assert_eq!(result[0].values, PropValue::Str(de_values));
        assert_eq!(result[1].values, PropValue::Str(en_values));
    }

    #[test]
    fn test_struct_shared_dict_deduplication() {
        // Strings shared across children should appear only once in the dictionary.
        // "Berlin" appears in both children; dictionary must deduplicate it.
        let de_values = strs(&["Berlin", "Berlin"]);
        let en_values = strs(&["Berlin", "London"]);

        let result = struct_roundtrip(
            "name",
            &[(":de", de_values.clone()), (":en", en_values.clone())],
            default_encoder(),
        );

        assert_eq!(result[0].values, PropValue::Str(de_values));
        assert_eq!(result[1].values, PropValue::Str(en_values));
    }

    #[test]
    fn test_struct_mixed_with_scalars() {
        // A scalar property before and after the struct group must be preserved,
        // correctly ordered, and round-trip to the same values.
        let enc = default_encoder();

        let scalar_before = DecodedProperty {
            name: "population".to_string(),
            values: PropValue::U32(vec![Some(1_000_000), Some(2_000_000)]),
        };
        let de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
        let en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
        let scalar_after = DecodedProperty {
            name: "rank".to_string(),
            values: PropValue::U32(vec![Some(1), Some(2)]),
        };

        let input = vec![
            scalar_before.clone(),
            de.clone(),
            en.clone(),
            scalar_after.clone(),
        ];
        let instructions = vec![
            EncodingInstruction::Scalar(enc),
            EncodingInstruction::struct_child("name", ":de", enc),
            EncodingInstruction::struct_child("name", ":en", enc),
            EncodingInstruction::Scalar(enc),
        ];

        let encoded_prop = Vec::<OwnedEncodedProperty>::from_decoded(&input, instructions)
            .expect("encoding failed");

        // Output order: scalar_before, struct("name"), scalar_after
        assert_eq!(encoded_prop.len(), 3);

        assert_eq!(decode_scalar(&encoded_prop[0]), scalar_before);

        let struct_children = expand_struct(&encoded_prop[1]);
        assert_eq!(struct_children[0].name, "name:de");
        assert_eq!(struct_children[0].values, de.values);
        assert_eq!(struct_children[1].name, "name:en");
        assert_eq!(struct_children[1].values, en.values);

        assert_eq!(decode_scalar(&encoded_prop[2]), scalar_after);
    }

    #[test]
    fn test_two_struct_groups() {
        // Two independent struct columns ("name" and "label") interleaved with a scalar,
        // all encoded in a single pass. Each struct must produce its own shared dictionary
        // and appear at the position of its first child in the output.
        let enc = default_encoder();

        let name_de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
        let name_en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
        let population = DecodedProperty {
            name: "population".to_string(),
            values: PropValue::U32(vec![Some(3_700_000), Some(1_800_000)]),
        };
        let label_de = str_prop(":de", strs(&["BE", "HH"]));
        let label_en = str_prop(":en", strs(&["BER", "HAM"]));

        // Input order: name:de, name:en, population, label:de, label:en
        let input = vec![
            name_de.clone(),
            name_en.clone(),
            population.clone(),
            label_de.clone(),
            label_en.clone(),
        ];
        let instructions = vec![
            EncodingInstruction::struct_child("name", ":de", enc),
            EncodingInstruction::struct_child("name", ":en", enc),
            EncodingInstruction::Scalar(enc),
            EncodingInstruction::struct_child("label", ":de", enc),
            EncodingInstruction::struct_child("label", ":en", enc),
        ];

        let encoded_prop = Vec::<OwnedEncodedProperty>::from_decoded(&input, instructions)
            .expect("encoding failed");

        // Expected output order: struct("name"), scalar("population"), struct("label")
        assert_eq!(encoded_prop.len(), 3);

        let name_children = expand_struct(&encoded_prop[0]);
        assert_eq!(name_children[0].name, "name:de");
        assert_eq!(name_children[0].values, name_de.values);
        assert_eq!(name_children[1].name, "name:en");
        assert_eq!(name_children[1].values, name_en.values);

        assert_eq!(decode_scalar(&encoded_prop[1]), population);

        let label_children = expand_struct(&encoded_prop[2]);
        assert_eq!(label_children[0].name, "label:de");
        assert_eq!(label_children[0].values, label_de.values);
        assert_eq!(label_children[1].name, "label:en");
        assert_eq!(label_children[1].values, label_en.values);
    }

    #[test]
    fn test_struct_instruction_count_mismatch() {
        let decoded = vec![DecodedProperty::default()];
        let instructions = vec![]; // wrong length

        let err = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, instructions)
            .expect_err("should fail");
        assert!(
            matches!(
                err,
                MltError::EncodingInstructionCountMismatch {
                    input_len: 1,
                    config_len: 0
                }
            ),
            "unexpected error: {err}"
        );
    }

    proptest! {
        /// Property-based roundtrip: arbitrary struct names, child names, string values,
        /// and encoder settings all survive encode → decode_expand intact.
        #[test]
        fn test_struct_shared_dict_proptest(
            struct_name in "[a-z]{1,8}",
            children in prop::collection::vec(
                (
                    "[a-z]{1,6}",
                    prop::collection::vec(prop::option::of("[a-zA-Z ]{0,20}"), 0..20),
                ),
                1..5usize,
            ),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
        ) {
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);

            let decoded: Vec<DecodedProperty> = children
                .iter()
                .map(|(child_name, values)| str_prop(child_name, values.clone()))
                .collect();

            let instructions: Vec<EncodingInstruction> = children
                .iter()
                .map(|(child_name, _)| EncodingInstruction::struct_child(&struct_name, child_name, encoder))
                .collect();

            let encoded = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, instructions)
                .expect("encoding failed");
            prop_assert_eq!(encoded.len(), 1, "children must collapse to one struct column");

            let re_children = expand_struct(&encoded[0]);
            prop_assert_eq!(re_children.len(), children.len());

            for (re, (child_name, values)) in re_children.into_iter().zip(children.iter()) {
                prop_assert_eq!(re.name, format!("{struct_name}{child_name}"));
                prop_assert_eq!(re.values, PropValue::Str(values.clone()));
            }
        }
    }

    /// Strategy for [`PhysicalEncoder`] that excludes `FastPFOR` to support 64bit ints
    fn physical_no_fastpfor() -> impl Strategy<Value = PhysicalEncoder> {
        any::<PhysicalEncoder>().prop_filter("not fastpfor", |v| *v != PhysicalEncoder::FastPFOR)
    }

    /// Encode a `DecodedProperty` and immediately decode it back.
    fn roundtrip(decoded: &DecodedProperty, encoder: PropertyEncoder) -> DecodedProperty {
        let encoded_data =
            OwnedEncodedProperty::from_decoded(decoded, encoder).expect("encoding failed");
        let borrowed = borrowme::borrow(&encoded_data);
        DecodedProperty::from_encoded(borrowed).expect("decoding failed")
    }

    proptest! {
        #[test]
        fn test_bool_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<bool>()), 0..100),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::Bool(values) };
            let encoder = PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::None,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_bool_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<bool>(), 0..100),
        ) {
            let opt_values: Vec<Option<bool>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::Bool(opt_values) };
            let encoder = PropertyEncoder::new(
                PresenceStream::Absent,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_i8_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i8>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I8(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_u8_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u8>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U8(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_i32_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i32>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I32(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_u32_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u32>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in any::<PhysicalEncoder>(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U32(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_i64_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<i64>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::I64(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_i64_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
        ) {
            let opt_values: Vec<Option<i64>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::I64(opt_values) };
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_u64_present_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(prop::option::of(any::<u64>()), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
        ) {
            let decoded = DecodedProperty { name, values: PropValue::U64(values) };
            let encoder = PropertyEncoder::new(PresenceStream::Present, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }

        #[test]
        fn test_u64_absent_roundtrip(
            name in any::<String>(),
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
            physical in physical_no_fastpfor(),
        ) {
            let opt_values: Vec<Option<u64>> = values.into_iter().map(Some).collect();
            let decoded = DecodedProperty { name, values: PropValue::U64(opt_values) };
            let encoder = PropertyEncoder::new(PresenceStream::Absent, logical, physical);
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::None,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(
                PresenceStream::Absent,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(
                PresenceStream::Present,
                LogicalEncoder::None,
                PhysicalEncoder::None,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
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
            let encoder = PropertyEncoder::new(
                PresenceStream::Absent,
                LogicalEncoder::None,
                PhysicalEncoder::VarInt,
            );
            prop_assert_eq!(roundtrip(&decoded, encoder), decoded);
        }
    }
}
