use borrowme::borrowme;

use crate::MltError;
use crate::decodable::{FromRaw, impl_decodable};
use crate::v01::Stream;

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, Default, PartialEq)]
pub enum Id<'a> {
    #[default]
    None,
    Raw(RawId<'a>),
    Decoded(DecodedId),
}

#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawId<'a> {
    optional: Option<Stream<'a>>,
    value: RawIdValue<'a>,
}

/// Column type enumeration
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedId(Option<Vec<Option<u64>>>);

impl_decodable!(Id<'a>, RawId<'a>, DecodedId);

impl<'a> From<RawId<'a>> for Id<'a> {
    fn from(value: RawId<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn raw(optional: Option<Stream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { optional, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedId::from_raw(v)?,
            Self::Decoded(v) => v,
            Self::None => DecodedId(None),
        })
    }
}

impl<'a> FromRaw<'a> for DecodedId {
    type Input = RawId<'a>;

    fn from_raw(RawId { optional, value }: RawId<'_>) -> Result<Self, MltError> {
        let value = match value {
            RawIdValue::Id32(stream) => {
                todo!("decode 32 bit Id from stream")
            }
            RawIdValue::Id64(stream) => {
                todo!("decode 64 bit LongId from stream")
            }
        };

        // Ok(DecodedId(Some(value)))
    }
}
