use borrowme::borrowme;

use crate::v01::Stream;

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
    pub(crate) name: &'a str,
    pub(crate) value: EncodedPropValue<'a>,
}

/// A sequence of encoded property values of various types
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedPropValue<'a> {
    Bool(EncodedValues<'a>),
    I8(EncodedValues<'a>),
    U8(EncodedValues<'a>),
    I32(EncodedValues<'a>),
    U32(EncodedValues<'a>),
    I64(EncodedValues<'a>),
    U64(EncodedValues<'a>),
    F32(EncodedValues<'a>),
    F64(EncodedValues<'a>),
    Str(EncodedStrings<'a>),
    SharedDict(EncodedSharedDict<'a>),
}

/// Decoded property values as a name and a vector of optional typed values
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedProperty {
    pub name: String,
    pub values: PropValue,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PresenceStream {
    /// Attaches a nullability stream
    Present,
    /// If there are nulls, drop them
    Absent,
}

// String-related types from property/strings.rs

/// A single child field within a `SharedDict` column
#[borrowme]
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedValues<'a> {
    pub name: &'a str,
    pub presence: Option<Stream<'a>>,
    pub data: Stream<'a>,
}

/// Encoded data for a `SharedDict` column with shared dictionary encoding.
///
/// Unlike `EncodedStrings`, shared dictionaries do NOT have their own offset stream.
/// Instead, each child column has its own offset stream that references the shared dictionary.
/// This is why only `Plain` and `FsstPlain` variants exist here.
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedSharedDict<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    Plain {
        prefix: &'a str,
        lengths: Stream<'a>,
        data: Stream<'a>,
        children: Vec<EncodedValues<'a>>,
    },
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    FsstPlain {
        prefix: &'a str,
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
        children: Vec<EncodedValues<'a>>,
    },
}

/// String column encoding as produced by the encoder (plain, dictionary, or FSST).
/// Stream order matches the encoder: see `StringEncoder.encode()` and `encodePlain` /
/// `encodeDictionary` / `encodeFsstDictionary`.
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStrings<'a> {
    /// Plain: length stream + data stream
    Plain {
        name: &'a str,
        presence: Option<Stream<'a>>,
        lengths: Stream<'a>,
        data: Stream<'a>,
    },
    /// Dictionary: lengths + offsets + dictionary data
    Dictionary {
        name: &'a str,
        presence: Option<Stream<'a>>,
        lengths: Stream<'a>,
        offsets: Stream<'a>,
        data: Stream<'a>,
    },
    /// FSST plain (4 streams): symbol lengths, symbol table, value lengths, compressed corpus. No offsets.
    FsstPlain {
        name: &'a str,
        presence: Option<Stream<'a>>,
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
    },
    /// FSST dictionary (5 streams): symbol lengths, symbol table, value lengths, compressed corpus, offsets.
    FsstDictionary {
        name: &'a str,
        presence: Option<Stream<'a>>,
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
        offsets: Stream<'a>,
    },
}
