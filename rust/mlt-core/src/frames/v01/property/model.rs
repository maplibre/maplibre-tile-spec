use std::borrow::Cow;

use enum_dispatch::enum_dispatch;

use crate::EncDec;
use crate::analyse::{Analyze, StatType};
use crate::v01::{FsstStrEncoder, IntEncoder, OwnedStream, Stream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRef<'a>(pub &'a str);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedName(pub String);

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

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedEncodedProperty {
    Bool(OwnedName, OwnedEncodedPresence, OwnedStream),
    I8(OwnedName, OwnedEncodedPresence, OwnedStream),
    U8(OwnedName, OwnedEncodedPresence, OwnedStream),
    I32(OwnedName, OwnedEncodedPresence, OwnedStream),
    U32(OwnedName, OwnedEncodedPresence, OwnedStream),
    I64(OwnedName, OwnedEncodedPresence, OwnedStream),
    U64(OwnedName, OwnedEncodedPresence, OwnedStream),
    F32(OwnedName, OwnedEncodedPresence, OwnedStream),
    F64(OwnedName, OwnedEncodedPresence, OwnedStream),
    Str(OwnedName, OwnedEncodedPresence, OwnedEncodedStrings),
    SharedDict(
        OwnedName,
        OwnedEncodedSharedDict,
        Vec<OwnedEncodedSharedDictChild>,
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
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedSharedDictChild<'a> {
    pub name: NameRef<'a>,
    pub presence: EncodedPresence<'a>,
    pub data: Stream<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedEncodedSharedDictChild {
    pub name: OwnedName,
    pub presence: OwnedEncodedPresence,
    pub data: OwnedStream,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlainData<'a> {
    pub lengths: Stream<'a>,
    pub data: Stream<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedPlainData {
    pub lengths: OwnedStream,
    pub data: OwnedStream,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FsstData<'a> {
    pub symbol_lengths: Stream<'a>,
    pub symbol_table: Stream<'a>,
    pub lengths: Stream<'a>,
    pub corpus: Stream<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedFsstData {
    pub symbol_lengths: OwnedStream,
    pub symbol_table: OwnedStream,
    pub lengths: OwnedStream,
    pub corpus: OwnedStream,
}

/// Encoded data for a `SharedDict` column with shared dictionary encoding.
///
/// Unlike `EncodedStrings`, shared dictionaries do NOT have their own offset stream.
/// Instead, each child column has its own offset stream that references the shared dictionary.
/// This is why only `Plain` and `FsstPlain` variants exist here.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedSharedDict<'a> {
    /// Plain shared dict (2 streams): lengths + data.
    Plain(PlainData<'a>),
    /// FSST plain shared dict (4 streams): symbol lengths, symbol table, lengths, corpus.
    FsstPlain(FsstData<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedEncodedSharedDict {
    Plain(OwnedPlainData),
    FsstPlain(OwnedFsstData),
}

/// String column encoding as produced by the encoder (plain, dictionary, or FSST).
/// Stream order matches the encoder: see `StringEncoder.encode()` and `encodePlain` /
/// `encodeDictionary` / `encodeFsstDictionary`.
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

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedEncodedStrings {
    Plain(OwnedPlainData),
    Dictionary {
        plain_data: OwnedPlainData,
        offsets: OwnedStream,
    },
    FsstPlain(OwnedFsstData),
    FsstDictionary {
        fsst_data: OwnedFsstData,
        offsets: OwnedStream,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodedPresence<'a>(pub Option<Stream<'a>>);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnedEncodedPresence(pub Option<OwnedStream>);

impl NameRef<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedName {
        OwnedName(self.0.to_string())
    }
}

impl OwnedName {
    #[must_use]
    pub fn as_borrowed(&self) -> NameRef<'_> {
        NameRef(&self.0)
    }
}

impl EncodedPresence<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedPresence {
        OwnedEncodedPresence(self.0.as_ref().map(Stream::to_owned))
    }
}

impl OwnedEncodedPresence {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedPresence<'_> {
        EncodedPresence(self.0.as_ref().map(OwnedStream::as_borrowed))
    }
}

impl EncodedSharedDictChild<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedSharedDictChild {
        OwnedEncodedSharedDictChild {
            name: self.name.to_owned(),
            presence: self.presence.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl OwnedEncodedSharedDictChild {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedSharedDictChild<'_> {
        EncodedSharedDictChild {
            name: self.name.as_borrowed(),
            presence: self.presence.as_borrowed(),
            data: self.data.as_borrowed(),
        }
    }
}

impl PlainData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedPlainData {
        OwnedPlainData {
            lengths: self.lengths.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl OwnedPlainData {
    #[must_use]
    pub fn as_borrowed(&self) -> PlainData<'_> {
        PlainData {
            lengths: self.lengths.as_borrowed(),
            data: self.data.as_borrowed(),
        }
    }
}

impl FsstData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedFsstData {
        OwnedFsstData {
            symbol_lengths: self.symbol_lengths.to_owned(),
            symbol_table: self.symbol_table.to_owned(),
            lengths: self.lengths.to_owned(),
            corpus: self.corpus.to_owned(),
        }
    }
}

impl OwnedFsstData {
    #[must_use]
    pub fn as_borrowed(&self) -> FsstData<'_> {
        FsstData {
            symbol_lengths: self.symbol_lengths.as_borrowed(),
            symbol_table: self.symbol_table.as_borrowed(),
            lengths: self.lengths.as_borrowed(),
            corpus: self.corpus.as_borrowed(),
        }
    }
}

impl EncodedSharedDict<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedSharedDict {
        match self {
            Self::Plain(data) => OwnedEncodedSharedDict::Plain(data.to_owned()),
            Self::FsstPlain(data) => OwnedEncodedSharedDict::FsstPlain(data.to_owned()),
        }
    }
}

impl OwnedEncodedSharedDict {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedSharedDict<'_> {
        match self {
            Self::Plain(data) => EncodedSharedDict::Plain(data.as_borrowed()),
            Self::FsstPlain(data) => EncodedSharedDict::FsstPlain(data.as_borrowed()),
        }
    }
}

impl EncodedStrings<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedStrings {
        match self {
            Self::Plain(data) => OwnedEncodedStrings::Plain(data.to_owned()),
            Self::Dictionary {
                plain_data,
                offsets,
            } => OwnedEncodedStrings::Dictionary {
                plain_data: plain_data.to_owned(),
                offsets: offsets.to_owned(),
            },
            Self::FsstPlain(data) => OwnedEncodedStrings::FsstPlain(data.to_owned()),
            Self::FsstDictionary { fsst_data, offsets } => OwnedEncodedStrings::FsstDictionary {
                fsst_data: fsst_data.to_owned(),
                offsets: offsets.to_owned(),
            },
        }
    }
}

impl OwnedEncodedStrings {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedStrings<'_> {
        match self {
            Self::Plain(data) => EncodedStrings::Plain(data.as_borrowed()),
            Self::Dictionary {
                plain_data,
                offsets,
            } => EncodedStrings::Dictionary {
                plain_data: plain_data.as_borrowed(),
                offsets: offsets.as_borrowed(),
            },
            Self::FsstPlain(data) => EncodedStrings::FsstPlain(data.as_borrowed()),
            Self::FsstDictionary { fsst_data, offsets } => EncodedStrings::FsstDictionary {
                fsst_data: fsst_data.as_borrowed(),
                offsets: offsets.as_borrowed(),
            },
        }
    }
}

impl EncodedProperty<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedProperty {
        match self {
            Self::Bool(name, presence, data) => {
                OwnedEncodedProperty::Bool(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::I8(name, presence, data) => {
                OwnedEncodedProperty::I8(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::U8(name, presence, data) => {
                OwnedEncodedProperty::U8(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::I32(name, presence, data) => {
                OwnedEncodedProperty::I32(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::U32(name, presence, data) => {
                OwnedEncodedProperty::U32(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::I64(name, presence, data) => {
                OwnedEncodedProperty::I64(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::U64(name, presence, data) => {
                OwnedEncodedProperty::U64(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::F32(name, presence, data) => {
                OwnedEncodedProperty::F32(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::F64(name, presence, data) => {
                OwnedEncodedProperty::F64(name.to_owned(), presence.to_owned(), data.to_owned())
            }
            Self::Str(name, presence, strings) => {
                OwnedEncodedProperty::Str(name.to_owned(), presence.to_owned(), strings.to_owned())
            }
            Self::SharedDict(name, dict, children) => OwnedEncodedProperty::SharedDict(
                name.to_owned(),
                dict.to_owned(),
                children
                    .iter()
                    .map(EncodedSharedDictChild::to_owned)
                    .collect(),
            ),
        }
    }
}

impl OwnedEncodedProperty {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedProperty<'_> {
        match self {
            Self::Bool(name, presence, data) => EncodedProperty::Bool(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::I8(name, presence, data) => EncodedProperty::I8(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::U8(name, presence, data) => EncodedProperty::U8(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::I32(name, presence, data) => EncodedProperty::I32(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::U32(name, presence, data) => EncodedProperty::U32(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::I64(name, presence, data) => EncodedProperty::I64(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::U64(name, presence, data) => EncodedProperty::U64(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::F32(name, presence, data) => EncodedProperty::F32(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::F64(name, presence, data) => EncodedProperty::F64(
                name.as_borrowed(),
                presence.as_borrowed(),
                data.as_borrowed(),
            ),
            Self::Str(name, presence, strings) => EncodedProperty::Str(
                name.as_borrowed(),
                presence.as_borrowed(),
                strings.as_borrowed(),
            ),
            Self::SharedDict(name, dict, children) => EncodedProperty::SharedDict(
                name.as_borrowed(),
                dict.as_borrowed(),
                children
                    .iter()
                    .map(OwnedEncodedSharedDictChild::as_borrowed)
                    .collect(),
            ),
        }
    }
}

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
