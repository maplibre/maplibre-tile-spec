use crate::encoder::stream::{FsstStrEncoder, IntEncoder};
use crate::v01::{
    EncodedStream, RawFsstData, RawPlainData, RawPresence, RawSharedDictItem, RawStream,
};

/// Owned name string (Stage 4/5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedName(pub String);

pub enum PropertyKind {
    Bool,
    Integer,
    Float,
    String,
    SharedDict,
}

/// Wire-ready encoded scalar column (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedScalar {
    pub(crate) name: EncodedName,
    pub(crate) presence: EncodedPresence,
    pub(crate) data: EncodedStream,
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
    pub(crate) name: EncodedName,
    pub(crate) presence: EncodedPresence,
    pub(crate) encoding: EncodedStringsEncoding,
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
    pub children: Vec<RawSharedDictItem<'a>>,
}

/// Wire-ready encoded shared-dictionary column (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedSharedDict {
    pub name: EncodedName,
    pub encoding: EncodedSharedDictEncoding,
    pub children: Vec<EncodedSharedDictItem>,
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

/// Staged property column (encode-side, fully owned).
///
/// Unlike `ParsedProperty` (decode-side, potentially borrowed), all string names
/// and corpus data are owned `String`s.  No lifetime parameter needed.
///
/// The `Encoded` variant holds wire-ready data after the `Staged*` → `Encoded*`
/// encoding step has been applied. This allows `StagedLayer01` to hold a mix of
/// staged and encoded properties before serialization.
#[derive(Debug, Clone, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
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

/// Wire-ready encoded shared dict child column (owns its byte buffers).
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedSharedDictItem {
    pub name: EncodedName,
    pub presence: EncodedPresence,
    pub data: EncodedStream,
}

/// Wire-ready encoded plain data (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPlainData {
    pub lengths: EncodedStream,
    pub data: EncodedStream,
}

/// Wire-ready encoded FSST data (owns its byte buffers).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedFsstData {
    pub symbol_lengths: EncodedStream,
    pub symbol_table: EncodedStream,
    pub lengths: EncodedStream,
    pub corpus: EncodedStream,
}

/// Wire-ready encoded presence/nullability stream (owns its byte buffers).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodedPresence(pub Option<EncodedStream>);

/// Instruction for how to encode a single parsed property when batch-encoding a
/// `Vec<ParsedProperty>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyEncoder {
    /// How to encode a scalar property
    Scalar(ScalarEncoder),
    /// How to encode a shared dictionary property (multiple string sub-properties)
    SharedDict(SharedDictEncoder),
}

/// How to encode properties
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ScalarEncoder {
    pub(crate) value: ScalarValueEncoder,
    /// When `true`, always emit a presence stream regardless of whether the column
    /// contains nulls. `false` (default) = derive from data.
    /// Only settable via `with_forced_presence` under the `__private` feature or in tests.
    #[cfg(feature = "__private")]
    #[cfg_attr(all(not(test), feature = "arbitrary"), arbitrary(value = false))]
    pub(crate) forced_presence: bool,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for ScalarEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarEncoder")
            .field("value", &self.value)
            .finish()
    }
}

#[cfg(feature = "__private")]
impl ScalarEncoder {
    /// Force a presence stream to be emitted even when the column has no nulls.
    /// Intended only for generating intentionally edge-case tiles in synthetics/tests.
    #[must_use]
    pub fn forced_presence(mut self, present: bool) -> Self {
        self.forced_presence = present;
        self
    }
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SharedDictItemEncoder {
    /// Encoder used for the offset-index stream of this child.
    pub offsets: IntEncoder,
    /// When `true`, always emit a presence stream regardless of whether the column
    /// contains nulls. `false` (default) = derive from data.
    /// Only settable via `with_forced_presence` under the `__private` feature or in tests.
    #[cfg(feature = "__private")]
    pub(crate) forced_presence: bool,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for SharedDictItemEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedDictItemEncoder")
            .field("offsets", &self.offsets)
            .finish()
    }
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
    Plain {
        string_lengths: IntEncoder,
    },
    /// Deduplicated plain dictionary: unique strings + per-feature offset indices.
    Dict {
        string_lengths: IntEncoder,
        offsets: IntEncoder,
    },
    Fsst(FsstStrEncoder),
    /// Deduplicated FSST dictionary: FSST-compressed unique strings + per-feature offset indices.
    FsstDict {
        fsst: FsstStrEncoder,
        offsets: IntEncoder,
    },
}

/// Describes the null pattern of a single column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceKind {
    /// The column has no rows
    Empty,
    /// Every row has a value
    AllPresent,
    /// Every row is null
    AllNull,
    /// Some rows are null, some have values
    Mixed,
}

impl PresenceKind {
    /// Returns `true` when a presence/validity stream must be written to the
    /// wire format (i.e. the column has at least one `null` value).
    #[must_use]
    pub fn needs_presence_stream(self) -> bool {
        matches!(self, Self::AllNull | Self::Mixed)
    }
}

/// Owned scalar column prepared for encoding (bool, integer, or float).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StagedScalar<T: Copy + PartialEq> {
    pub(crate) name: String,
    pub values: Vec<Option<T>>,
}

/// Owned string column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedStrings {
    pub(crate) name: String,
    /// Per-feature cumulative end offsets into `data` (same encoding as `ParsedStrings::lengths`).
    pub lengths: Vec<i32>,
    pub(crate) data: String,
}

/// Owned shared-dictionary column prepared for encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDict {
    pub(crate) prefix: String,
    pub(crate) data: String,
    pub items: Vec<StagedSharedDictItem>,
}

/// A single child within a staged shared-dictionary column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSharedDictItem {
    pub(crate) suffix: String,
    /// Per-feature `(start, end)` byte offsets into the shared corpus.
    pub ranges: Vec<(i32, i32)>,
}
