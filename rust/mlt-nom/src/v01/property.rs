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
pub struct DecodedProperty {
    name: String,
    values: PropValue,
}

/// Column type enumeration
#[derive(Debug, Clone, Default, PartialEq)]
pub enum PropValue {
    Bool(Vec<Option<bool>>),
    I8(Vec<Option<i8>>),
    U8(Vec<Option<u8>>),
    I32(Vec<Option<i32>>),
    U32(Vec<Option<u32>>),
    I64(Vec<Option<i64>>),
    U64(Vec<Option<u64>>),
    F32(Vec<Option<f32>>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    #[default]
    Struct,
}

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

    fn from_raw(v: RawProperty<'_>) -> Result<Self, MltError> {
        Ok(DecodedProperty {
            name: v.name.to_string(),
            values: match v.value {
                RawPropValue::Bool(_) => PropValue::Bool(Vec::new()),
                RawPropValue::I8(_) => PropValue::I8(Vec::new()),
                RawPropValue::U8(_) => PropValue::U8(Vec::new()),
                RawPropValue::I32(_) => PropValue::I32(Vec::new()),
                RawPropValue::U32(_) => PropValue::U32(Vec::new()),
                RawPropValue::I64(_) => PropValue::I64(Vec::new()),
                RawPropValue::U64(_) => PropValue::U64(Vec::new()),
                RawPropValue::F32(_) => PropValue::F32(Vec::new()),
                RawPropValue::F64(_) => PropValue::F64(Vec::new()),
                RawPropValue::Str(_) => PropValue::Str(Vec::new()),
                RawPropValue::Struct(_) => PropValue::Struct,
            },
        })
    }
}
