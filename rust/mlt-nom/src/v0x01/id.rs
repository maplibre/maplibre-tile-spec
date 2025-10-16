use borrowme::borrowme;

use crate::MltError;
use crate::v0x01::{Parsable, Stream, impl_decodable};

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq, Default)]
pub enum Id<'a> {
    #[default]
    None,
    Raw(RawId<'a>),
    Decoded(DecodedId),
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn raw(optional: Option<Stream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { optional, value })
    }
}

impl_decodable!(Id<'a>, RawId<'a>, DecodedId);

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
    Id(Stream<'a>),
    LongId(Stream<'a>),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DecodedId(Vec<Option<u64>>);

impl<'a> Parsable<'a> for DecodedId {
    type Input = RawId<'a>;

    fn parse(RawId { optional, value }: RawId<'_>) -> Result<Self, MltError> {
        todo!()
    }
}
