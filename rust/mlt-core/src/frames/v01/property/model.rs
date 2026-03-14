use std::borrow::Cow;

use enum_dispatch::enum_dispatch;

use crate::EncDec;
use crate::analyse::{Analyze, StatType};
use crate::v01::{EncodedStream, FsstStrEncoder, IntEncoder, RawStream};

/// Owned name string (Stage 4/5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedName(pub String);

/// Property representation, either raw (borrowed from bytes) or parsed.
pub type Property<'a> = EncDec<RawProperty<'a>, ParsedProperty<'a>>;

pub enum PropertyKind {
    Bool,
    Integer,
    Float,
    String,
    SharedDict,
}

/// Raw scalar column (bool, integer, or float) as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawScalar<'a> {
    pub name: &'a str,
    pub presence: RawPresence<'a>,
    pub data: RawStream<'a>,
}

/// Wire-ready encoded scalar column (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedScalar {
    pub name: EncodedName,
    pub presence: EncodedPresence,
    pub data: EncodedStream,
}

/// Raw encoding payload for a string column (plain, dictionary, or FSST variants).
///
/// `RawStream` order matches the encoder: see `StringEncoder.encode()`.
#[derive(Debug, Clone, PartialEq)]
pub enum RawStringsEncoding<'a> {
    /// Plain: length stream + data stream
    Plain(RawPlainData<'a>),
    /// Dictionary: lengths + offsets + dictionary data
    Dictionary {
        plain_data: RawPlainData<'a>,
        offsets: RawStream<'a>,
    },
    /// FSST plain (4 streams): symbol lengths, symbol table, value lengths, compressed corpus. No offsets.
    FsstPlain(RawFsstData<'a>),
    /// FSST dictionary (5 streams): symbol lengths, symbol table, value lengths, compressed corpus, offsets.
    FsstDictionary {
        fsst_data: RawFsstData<'a>,
        offsets: RawStream<'a>,
    },
}

/// Wire-ready encoded strings encoding (owns byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStringsEncoding {
    Plain(EncodedPlainData),
    Dictionary {
        plain_data: EncodedPlainData,
        offsets: EncodedStream,
    },
    FsstPlain(EncodedFsstData),
    FsstDictionary {
        fsst_data: EncodedFsstData,
        offsets: EncodedStream,
    },
}

/// Raw string column as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStrings<'a> {
    pub name: &'a str,
    pub presence: RawPresence<'a>,
    pub encoding: RawStringsEncoding<'a>,
}

/// Wire-ready encoded string column (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedStrings {
    pub name: EncodedName,
    pub presence: EncodedPresence,
    pub encoding: EncodedStringsEncoding,
}

/// Raw encoding payload for a `SharedDict` column.
///
/// Unlike [`RawStringsEncoding`], shared dictionaries do NOT have their own offset stream.
/// Instead, each child column has its own offset stream that references the shared dictionary.
/// This is why only `Plain` and `FsstPlain` variants exist here.
#[derive(Debug, Clone, PartialEq)]
pub enum RawSharedDictEncoding<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    Plain(RawPlainData<'a>),
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    FsstPlain(RawFsstData<'a>),
}

/// Wire-ready encoded shared dict encoding (owns byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedSharedDictEncoding {
    Plain(EncodedPlainData),
    FsstPlain(EncodedFsstData),
}

/// Raw shared-dictionary column as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSharedDict<'a> {
    pub name: &'a str,
    pub encoding: RawSharedDictEncoding<'a>,
    pub children: Vec<RawSharedDictChild<'a>>,
}

/// Wire-ready encoded shared-dictionary column (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedSharedDict {
    pub name: EncodedName,
    pub encoding: EncodedSharedDictEncoding,
    pub children: Vec<EncodedSharedDictChild>,
}

/// Raw property data as read directly from the tile.
#[derive(Debug, PartialEq)]
pub enum RawProperty<'a> {
    Bool(RawScalar<'a>),
    I8(RawScalar<'a>),
    U8(RawScalar<'a>),
    I32(RawScalar<'a>),
    U32(RawScalar<'a>),
    I64(RawScalar<'a>),
    U64(RawScalar<'a>),
    F32(RawScalar<'a>),
    F64(RawScalar<'a>),
    Str(RawStrings<'a>),
    SharedDict(RawSharedDict<'a>),
}

/// Wire-ready encoded property data (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedProperty {
    Bool(EncodedScalar),
    I8(EncodedScalar),
    U8(EncodedScalar),
    I32(EncodedScalar),
    U32(EncodedScalar),
    I64(EncodedScalar),
    U64(EncodedScalar),
    F32(EncodedScalar),
    F64(EncodedScalar),
    Str(EncodedStrings),
    SharedDict(EncodedSharedDict),
}

/// Parsed property values in a typed enum form.
#[derive(Clone, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[enum_dispatch(Analyze)]
pub enum ParsedProperty<'a> {
    Bool(ParsedScalar<'a, bool>),
    I8(ParsedScalar<'a, i8>),
    U8(ParsedScalar<'a, u8>),
    I32(ParsedScalar<'a, i32>),
    U32(ParsedScalar<'a, u32>),
    I64(ParsedScalar<'a, i64>),
    U64(ParsedScalar<'a, u64>),
    F32(ParsedScalar<'a, f32>),
    F64(ParsedScalar<'a, f64>),
    Str(ParsedStrings<'a>),
    SharedDict(ParsedSharedDict<'a>),
}

/// Staged property column (encode-side, fully owned).
///
/// Unlike `ParsedProperty` (decode-side, potentially borrowed), all string names
/// and corpus data are owned `String`s.  No lifetime parameter needed.
///
/// The `Encoded` variant holds wire-ready data after the `Staged*` → `Encoded*`
/// encoding step has been applied. This allows `StagedLayer01` to hold a mix of
/// staged and encoded properties before serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedProperty {
    Bool(StagedScalar<bool>),
    I8(StagedScalar<i8>),
    U8(StagedScalar<u8>),
    I32(StagedScalar<i32>),
    U32(StagedScalar<u32>),
    I64(StagedScalar<i64>),
    U64(StagedScalar<u64>),
    F32(StagedScalar<f32>),
    F64(StagedScalar<f64>),
    Str(StagedStrings),
    SharedDict(StagedSharedDict),
}

#[derive(Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ParsedScalar<'a, T: Copy + PartialEq> {
    pub name: &'a str,
    pub values: Vec<Option<T>>,
}

/// A single sub-property within a shared dictionary parsed value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ParsedSharedDictItem<'a> {
    /// The suffix name of this sub-property (appended to parent struct name).
    pub suffix: &'a str,
    /// Per-feature `(start, end)` byte offsets into the parsed shared corpus.
    /// Non-negative pairs indicate a present string stored as
    /// `shared_dict.corpus()[start..end]`.
    /// `(-1, -1)` indicates NULL.
    /// Equal `start` and `end` indicate an empty string.
    pub ranges: Vec<(i32, i32)>,
}

/// Parsed string values for a single property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStrings<'a> {
    pub name: &'a str,
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

/// Parsed shared dictionary payload shared by one or more child string properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSharedDict<'a> {
    pub prefix: &'a str,
    pub data: Cow<'a, str>,
    pub items: Vec<ParsedSharedDictItem<'a>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ParsedPresence(pub Option<Vec<bool>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PresenceStream {
    /// Attaches a nullability stream
    Present,
    /// If there are nulls, drop them
    Absent,
}

/// A single child field within a `SharedDict` raw column
#[derive(Clone, Debug, PartialEq)]
pub struct RawSharedDictChild<'a> {
    pub name: &'a str,
    pub presence: RawPresence<'a>,
    pub data: RawStream<'a>,
}

/// Wire-ready encoded shared dict child column (owns its byte buffers).
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedSharedDictChild {
    pub name: EncodedName,
    pub presence: EncodedPresence,
    pub data: EncodedStream,
}

/// Raw plain data (length stream + data stream) borrowed from input bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPlainData<'a> {
    pub lengths: RawStream<'a>,
    pub data: RawStream<'a>,
}

/// Wire-ready encoded plain data (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPlainData {
    pub lengths: EncodedStream,
    pub data: EncodedStream,
}

/// Raw FSST-compressed data (4 streams) borrowed from input bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RawFsstData<'a> {
    pub symbol_lengths: RawStream<'a>,
    pub symbol_table: RawStream<'a>,
    pub lengths: RawStream<'a>,
    pub corpus: RawStream<'a>,
}

/// Wire-ready encoded FSST data (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedFsstData {
    pub symbol_lengths: EncodedStream,
    pub symbol_table: EncodedStream,
    pub lengths: EncodedStream,
    pub corpus: EncodedStream,
}

/// Raw presence/nullability stream borrowed from input bytes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawPresence<'a>(pub Option<RawStream<'a>>);

/// Wire-ready encoded presence/nullability stream (owns its byte buffers).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodedPresence(pub Option<EncodedStream>);

/// Instruction for how to encode a single parsed property when batch-encoding a
/// [`Vec<ParsedProperty>`] via [`crate::optimizer::ManualOptimisation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyEncoder {
    /// How to encode a scalar property
    Scalar(ScalarEncoder),
    /// How to encode a shared dictionary property (multiple string sub-properties)
    SharedDict(SharedDictEncoder),
}

/// How to encode properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ScalarEncoder {
    pub presence: PresenceStream,
    pub value: ScalarValueEncoder,
}

/// How to encode scalar property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum ScalarValueEncoder {
    Int(IntEncoder),
    String(StrEncoder),
    Float,
    Bool,
}

/// Encoder for an individual sub-property within a shared dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDictItemEncoder {
    /// If a stream for optional values should be attached.
    pub presence: PresenceStream,
    /// Encoder used for the offset-index stream of this child.
    pub offsets: IntEncoder,
}

/// Encoder for a shared dictionary property with multiple string sub-properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDictEncoder {
    /// Encoder for the shared dictionary strings (plain vs FSST).
    pub dict_encoder: StrEncoder,
    /// Encoders for individual sub-properties.
    pub items: Vec<SharedDictItemEncoder>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StrEncoder {
    Plain { string_lengths: IntEncoder },
    Fsst(FsstStrEncoder),
}

// ── Staged* types (encode-side, fully owned) ─────────────────────────────────

/// Owned scalar column prepared for encoding (bool, integer, or float).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StagedScalar<T: Copy + PartialEq> {
    pub name: String,
    pub values: Vec<Option<T>>,
}

/// Owned string column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedStrings {
    pub name: String,
    /// Per-feature cumulative end offsets into `data` (same encoding as [`ParsedStrings::lengths`]).
    pub lengths: Vec<i32>,
    pub data: String,
}

/// A single child within a staged shared-dictionary column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDictItem {
    pub suffix: String,
    /// Per-feature `(start, end)` byte offsets into the shared corpus.
    pub ranges: Vec<(i32, i32)>,
}

/// Owned shared-dictionary column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDict {
    pub prefix: String,
    pub data: String,
    pub items: Vec<StagedSharedDictItem>,
}
