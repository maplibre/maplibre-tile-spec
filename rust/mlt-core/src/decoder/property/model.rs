use std::borrow::Cow;

use enum_dispatch::enum_dispatch;

use crate::decoder::RawStream;
use crate::utils::Presence;
use crate::{DecodeState, Lazy};

/// Property column representation, parameterized by decode state.
///
/// - `Property<'a>` / `Property<'a, Lazy>` — either raw bytes or decoded, in an [`crate::LazyParsed`] enum.
/// - `Property<'a, Parsed>` — decoded [`ParsedProperty`] directly (no enum wrapper).
pub type Property<'a, S = Lazy> =
    <S as DecodeState>::LazyOrParsed<RawProperty<'a>, ParsedProperty<'a>>;

/// Raw scalar column (bool, integer, or float) as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawScalar<'a> {
    pub(crate) name: &'a str,
    pub(crate) presence: RawPresence<'a>,
    pub(crate) data: RawStream<'a>,
}

/// Raw string column as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStrings<'a> {
    pub name: &'a str,
    pub presence: RawPresence<'a>,
    pub encoding: RawStringsEncoding<'a>,
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

/// Raw shared-dictionary column as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSharedDict<'a> {
    pub name: &'a str,
    pub encoding: RawSharedDictEncoding<'a>,
    pub children: Vec<RawSharedDictItem<'a>>,
}

/// Raw property data as read directly from the tile.
#[derive(Debug, PartialEq, Clone)]
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

/// Decoded scalar property column (bool, integer, or float).
///
/// `presence` indicates which features have a value; `values` holds only the
/// non-null (present) entries in dense order.
///
/// For a non-optional column, `presence` is [`Presence::AllPresent`] and
/// `values.len()` equals the feature count.  For an optional column, `presence`
/// is [`Presence::Bits(bits)`][`Presence::Bits`] with `bits.count_ones() == values.len()`.
#[derive(Clone, PartialEq)]
pub struct ParsedScalar<'a, T: Copy + PartialEq> {
    pub(crate) name: &'a str,
    pub(crate) presence: Presence<'a>,
    /// Dense values: only entries where the corresponding presence bit is set.
    pub(crate) values: Vec<T>,
}

/// Per-feature byte range into a shared dictionary corpus.
///
/// `start` and `end` are signed byte offsets into the corpus string.
/// The sentinel value [`DictRange::NULL`] (`-1, -1`) indicates a NULL (absent) entry.
/// Equal `start` and `end` indicate an empty string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct DictRange {
    pub start: i32,
    pub end: i32,
}
impl DictRange {
    /// Sentinel value indicating a NULL (absent) entry.
    pub const NULL: Self = Self { start: -1, end: -1 };
}

/// A single sub-property within a shared dictionary parsed value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ParsedSharedDictItem<'a> {
    /// The suffix name of this sub-property (appended to parent struct name).
    pub(crate) suffix: &'a str,
    /// Per-feature byte ranges into the parsed shared corpus.
    /// Non-null entries indicate a present string stored as
    /// `shared_dict.corpus()[start..end]`.
    /// [`DictRange::NULL`] indicates a NULL value.
    /// Equal `start` and `end` indicate an empty string.
    pub(crate) ranges: Vec<DictRange>,
}

/// Parsed string values for a single property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStrings<'a> {
    pub(crate) name: &'a str,
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
    pub(crate) lengths: Vec<i32>,
    pub(crate) data: Cow<'a, str>,
}

/// Parsed shared dictionary payload shared by one or more child string properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSharedDict<'a> {
    pub(crate) prefix: &'a str,
    pub(crate) data: Cow<'a, str>,
    pub(crate) items: Vec<ParsedSharedDictItem<'a>>,
}

/// A single child field within a `SharedDict` raw column
#[derive(Clone, Debug, PartialEq)]
pub struct RawSharedDictItem<'a> {
    pub name: &'a str,
    pub presence: RawPresence<'a>,
    pub data: RawStream<'a>,
}

/// Raw plain data (length stream + data stream) borrowed from input bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPlainData<'a> {
    pub lengths: RawStream<'a>,
    pub data: RawStream<'a>,
}

/// Raw FSST-compressed data (4 streams) borrowed from input bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RawFsstData<'a> {
    pub symbol_lengths: RawStream<'a>,
    pub symbol_table: RawStream<'a>,
    pub lengths: RawStream<'a>,
    pub corpus: RawStream<'a>,
}

/// Raw presence/nullability stream borrowed from input bytes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawPresence<'a>(pub Option<RawStream<'a>>);
