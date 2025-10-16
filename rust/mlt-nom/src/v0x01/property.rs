use borrowme::borrowme;

use crate::MltError;
use crate::v0x01::{Parsable, Stream, impl_decodable};

#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Property<'a> {
    Raw(RawProperty<'a>),
    Decoded(DecodedProperty),
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
}

impl_decodable!(Property<'a>, RawProperty<'a>, DecodedProperty);

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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DecodedProperty();

impl<'a> Parsable<'a> for DecodedProperty {
    type Input = RawProperty<'a>;

    fn parse(RawProperty { .. }: RawProperty<'_>) -> Result<Self, MltError> {
        todo!()
    }
}
