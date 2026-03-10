use std::borrow::Cow;

use borrowme::borrowme;

use crate::v01::Stream;

#[borrowme(name = OwnedName)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRef<'a>(pub &'a str);

/// Property representation, either encoded or decoded
#[allow(clippy::large_enum_variant)]
#[borrowme]
#[derive(Debug, PartialEq)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
pub enum Property<'a> {
    Encoded(EncodedProperty<'a>),
    Decoded(DecodedProperty<'a>),
}

pub enum PropertyKind {
    Bool,
    Integer,
    Float,
    String,
    SharedDict,
}

/// Unparsed property data as read directly from the tile.
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedProperty<'a> {
    Bool(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    I8(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    U8(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    I32(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    U32(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    I64(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    U64(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    F32(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    F64(NameRef<'a>, EncodedPresence<'a>, Stream<'a>),
    Str(NameRef<'a>, EncodedPresence<'a>, EncodedStrings<'a>),
    SharedDict(
        NameRef<'a>,
        EncodedSharedDict<'a>,
        Vec<EncodedSharedDictChild<'a>>,
    ),
}

/// Decoded property values in a typed enum form.
#[derive(Clone, PartialEq)]
pub enum DecodedProperty<'a> {
    Bool(DecodedScalar<bool>),
    I8(DecodedScalar<i8>),
    U8(DecodedScalar<u8>),
    I32(DecodedScalar<i32>),
    U32(DecodedScalar<u32>),
    I64(DecodedScalar<i64>),
    U64(DecodedScalar<u64>),
    F32(DecodedScalar<f32>),
    F64(DecodedScalar<f64>),
    Str(DecodedStrings<'a>),
    SharedDict(DecodedSharedDict<'a>),
}

#[derive(Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedScalar<T: Copy + PartialEq> {
    pub name: String,
    pub values: Vec<Option<T>>,
}

/// A single sub-property within a shared dictionary decoded value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedSharedDictItem {
    // TODO: suffix should be Cow<'a, str>
    /// The suffix name of this sub-property (appended to parent struct name).
    pub suffix: String,
    /// Per-feature `(start, end)` byte offsets into the decoded shared corpus.
    /// Non-negative pairs indicate a present string stored as
    /// `shared_dict.corpus()[start..end]`.
    /// `(-1, -1)` indicates NULL.
    /// Equal `start` and `end` indicate an empty string.
    pub ranges: Vec<(i32, i32)>,
}

/// Decoded string values for a single property.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStrings<'a> {
    pub name: Cow<'a, str>,
    /// Per-feature cumulative end offsets into `data`.
    /// Non-negative values indicate a present string and store its exclusive
    /// end offset in `data`.
    /// Negative values indicate NULL and encode the current offset as `-end-1`,
    /// which is equivalent to `!end` in two's-complement form,
    /// so the next item can still recover its start offset without scanning back
    /// to the previous non-null value. This allows even the first item to be NULL.
    /// In other words, if `lengths == [5, 5, -6, 8]`, then the strings are:
    /// ```ignore
    /// data[0..5], // 0th string
    /// data[5..5], // 1st string is empty
    /// NULL,       // 2nd string, offset stays 5 because -6 == -5-1
    /// data[5..8], // 3rd string
    /// ```
    pub lengths: Vec<i32>,
    pub data: Cow<'a, str>,
}

/// Decoded shared dictionary payload shared by one or more child string properties.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedSharedDict<'a> {
    pub prefix: Cow<'a, str>,
    pub data: Cow<'a, str>,
    pub items: Vec<DecodedSharedDictItem>,
}

/// Compatibility helper for constructing typed decoded properties.
#[derive(Debug, Clone, PartialEq)]
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
    Str(DecodedStrings<'static>),
    SharedDict(DecodedSharedDict<'static>),
}
pub type SharedDictItem = DecodedSharedDictItem;

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedPresence(pub Option<Vec<bool>>);

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
pub struct EncodedSharedDictChild<'a> {
    pub name: NameRef<'a>,
    pub presence: EncodedPresence<'a>,
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
        lengths: Stream<'a>,
        data: Stream<'a>,
    },
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    FsstPlain {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
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
        lengths: Stream<'a>,
        data: Stream<'a>,
    },
    /// Dictionary: lengths + offsets + dictionary data
    Dictionary {
        lengths: Stream<'a>,
        offsets: Stream<'a>,
        data: Stream<'a>,
    },
    /// FSST plain (4 streams): symbol lengths, symbol table, value lengths, compressed corpus. No offsets.
    FsstPlain {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
    },
    /// FSST dictionary (5 streams): symbol lengths, symbol table, value lengths, compressed corpus, offsets.
    FsstDictionary {
        symbol_lengths: Stream<'a>,
        symbol_table: Stream<'a>,
        lengths: Stream<'a>,
        corpus: Stream<'a>,
        offsets: Stream<'a>,
    },
}

#[borrowme]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodedPresence<'a>(pub Option<Stream<'a>>);
