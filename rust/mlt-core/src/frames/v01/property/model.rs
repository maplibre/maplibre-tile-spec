use std::borrow::Cow;

use borrowme::borrowme;
use enum_dispatch::enum_dispatch;

use crate::EncDec;
use crate::analyse::{Analyze, StatType};
use crate::v01::{FsstStrEncoder, IntEncoder, Stream};

#[borrowme(name = OwnedName)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRef<'a>(pub &'a str);

/// Property representation, either encoded or decoded
pub type Property<'a> = EncDec<EncodedProperty<'a>, DecodedProperty<'a>>;

/// Owned property representation, either encoded or decoded.
pub type OwnedProperty = EncDec<OwnedEncodedProperty, DecodedProperty<'static>>;

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
#[derive(Clone, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[enum_dispatch(Analyze)]
pub enum DecodedProperty<'a> {
    Bool(DecodedScalar<'a, bool>),
    I8(DecodedScalar<'a, i8>),
    U8(DecodedScalar<'a, u8>),
    I32(DecodedScalar<'a, i32>),
    U32(DecodedScalar<'a, u32>),
    I64(DecodedScalar<'a, i64>),
    U64(DecodedScalar<'a, u64>),
    F32(DecodedScalar<'a, f32>),
    F64(DecodedScalar<'a, f64>),
    Str(DecodedStrings<'a>),
    SharedDict(DecodedSharedDict<'a>),
}

#[derive(Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedScalar<'a, T: Copy + PartialEq> {
    pub name: Cow<'a, str>,
    pub values: Vec<Option<T>>,
}

/// A single sub-property within a shared dictionary decoded value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedSharedDictItem<'a> {
    /// The suffix name of this sub-property (appended to parent struct name).
    pub suffix: Cow<'a, str>,
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
    pub items: Vec<DecodedSharedDictItem<'a>>,
}

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

#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub struct PlainData<'a> {
    pub lengths: Stream<'a>,
    pub data: Stream<'a>,
}

#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub struct FsstData<'a> {
    pub symbol_lengths: Stream<'a>,
    pub symbol_table: Stream<'a>,
    pub lengths: Stream<'a>,
    pub corpus: Stream<'a>,
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
    Plain(PlainData<'a>),
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    FsstPlain(FsstData<'a>),
}

/// String column encoding as produced by the encoder (plain, dictionary, or FSST).
/// Stream order matches the encoder: see `StringEncoder.encode()` and `encodePlain` /
/// `encodeDictionary` / `encodeFsstDictionary`.
#[borrowme]
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStrings<'a> {
    /// Plain: length stream + data stream
    Plain(PlainData<'a>),
    /// Dictionary: lengths + offsets + dictionary data
    Dictionary {
        plain_data: PlainData<'a>,
        offsets: Stream<'a>,
    },
    /// FSST plain (4 streams): symbol lengths, symbol table, value lengths, compressed corpus. No offsets.
    FsstPlain(FsstData<'a>),
    /// FSST dictionary (5 streams): symbol lengths, symbol table, value lengths, compressed corpus, offsets.
    FsstDictionary {
        fsst_data: FsstData<'a>,
        offsets: Stream<'a>,
    },
}

#[borrowme]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodedPresence<'a>(pub Option<Stream<'a>>);

/// Instruction for how to encode a single decoded property when batch-encoding a
/// [`Vec<DecodedProperty>`] via [`crate::optimizer::ManualOptimisation`].
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
