use borrowme::borrowme;

use crate::MltError;
use crate::decodable::{FromRaw, impl_decodable};
use crate::v01::Stream;

#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Raw(RawProperty<'a>),
    Decoded(DecodedProperty),
}

#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawProperty<'a> {
    name: &'a str,
    optional: Option<Stream<'a>>,
    value: RawPropValue<'a>,
}

/// Column type enumeration
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawPropValue<'a> {
    Bool(Stream<'a>),
    I8(Stream<'a>),
    U8(Stream<'a>),
    I32(Stream<'a>),
    U32(Stream<'a>),
    I64(Stream<'a>),
    U64(Stream<'a>),
    F32(Stream<'a>),
    F64(Stream<'a>),
    Str(Vec<Stream<'a>>),
    Struct(Stream<'a>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedProperty();

impl_decodable!(Property<'a>, RawProperty<'a>, DecodedProperty);

impl<'a> From<RawProperty<'a>> for Property<'a> {
    fn from(value: RawProperty<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn raw(name: &'a str, optional: Option<Stream<'a>>, value: RawPropValue<'a>) -> Self {
        Self::Raw(RawProperty {
            name,
            optional,
            value,
        })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedProperty, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedProperty::from_raw(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl<'a> FromRaw<'a> for DecodedProperty {
    type Input = RawProperty<'a>;

    fn from_raw(RawProperty { .. }: RawProperty<'_>) -> Result<Self, MltError> {
        todo!()
    }
}
