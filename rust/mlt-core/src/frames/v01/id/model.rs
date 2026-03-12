use crate::EncDec;
use crate::v01::{OwnedStream, Stream};

/// ID column representation, either encoded or decoded.
pub type Id<'a> = EncDec<EncodedId<'a>, DecodedId>;

/// Owned ID column representation, either encoded or decoded.
pub type OwnedId = EncDec<OwnedEncodedId, DecodedId>;

/// Unparsed ID data as read directly from the tile
#[derive(Debug, PartialEq)]
pub struct EncodedId<'a> {
    pub(crate) presence: Option<Stream<'a>>,
    pub(crate) value: EncodedIdValue<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OwnedEncodedId {
    pub(crate) presence: Option<OwnedStream>,
    pub(crate) value: OwnedEncodedIdValue,
}

/// A sequence of encoded ID values, either 32-bit or 64-bit unsigned integers
#[derive(Debug, PartialEq)]
pub enum EncodedIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum OwnedEncodedIdValue {
    Id32(OwnedStream),
    Id64(OwnedStream),
}

/// Decoded ID values as a vector of optional 64-bit unsigned integers
#[derive(Clone, Default, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedId(pub Vec<Option<u64>>);

impl DecodedId {
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }

    #[must_use]
    pub fn as_borrowed(&self) -> Self {
        self.clone()
    }
}

impl EncodedId<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedId {
        OwnedEncodedId {
            presence: self.presence.as_ref().map(Stream::to_owned),
            value: self.value.to_owned(),
        }
    }
}

impl OwnedEncodedId {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedId<'_> {
        EncodedId {
            presence: self.presence.as_ref().map(OwnedStream::as_borrowed),
            value: self.value.as_borrowed(),
        }
    }
}

impl EncodedIdValue<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedIdValue {
        match self {
            Self::Id32(stream) => OwnedEncodedIdValue::Id32(stream.to_owned()),
            Self::Id64(stream) => OwnedEncodedIdValue::Id64(stream.to_owned()),
        }
    }
}

impl OwnedEncodedIdValue {
    #[must_use]
    pub fn as_borrowed(&self) -> EncodedIdValue<'_> {
        match self {
            Self::Id32(stream) => EncodedIdValue::Id32(stream.as_borrowed()),
            Self::Id64(stream) => EncodedIdValue::Id64(stream.as_borrowed()),
        }
    }
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
