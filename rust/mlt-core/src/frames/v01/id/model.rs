use crate::EncDec;
use crate::v01::{EncodedStream, RawPresence, RawStream};

/// ID column representation, either raw (borrowed from bytes) or parsed.
pub type Id<'a> = EncDec<RawId<'a>, IdValues>;

/// Unparsed ID data as read directly from the tile (borrows from input bytes)
#[derive(Debug, PartialEq, Clone)]
pub struct RawId<'a> {
    pub(crate) presence: RawPresence<'a>,
    pub(crate) value: RawIdValue<'a>,
}

/// A sequence of raw ID values, either 32-bit or 64-bit unsigned integers
#[derive(Debug, PartialEq, Clone)]
pub enum RawIdValue<'a> {
    Id32(RawStream<'a>),
    Id64(RawStream<'a>),
}

/// Parsed ID values as a vector of optional 64-bit unsigned integers
#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IdValues(pub Vec<Option<u64>>);

/// Wire-ready encoded ID data (owns its byte buffers)
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedId {
    pub(crate) presence: Option<EncodedStream>,
    pub(crate) value: EncodedIdValue,
}

/// Wire-ready encoded ID value, either 32-bit or 64-bit
#[derive(Debug, PartialEq, Clone)]
pub enum EncodedIdValue {
    Id32(EncodedStream),
    Id64(EncodedStream),
}

/// How wide are the IDs
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum IdWidth {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}
