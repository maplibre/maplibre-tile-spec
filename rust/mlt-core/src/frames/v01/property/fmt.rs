use std::borrow::Cow;
use std::fmt::{self, Debug};

use crate::utils::FmtOptVec;
use crate::v01::{
    ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
};

/// Custom implementation to ensure values are printed without newlines
impl Debug for ParsedProperty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => f
                .debug_tuple("Bool")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I8(v) => f
                .debug_tuple("I8")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U8(v) => f
                .debug_tuple("U8")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I32(v) => f
                .debug_tuple("I32")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U32(v) => f
                .debug_tuple("U32")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I64(v) => f
                .debug_tuple("I64")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U64(v) => f
                .debug_tuple("U64")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::F32(v) => f
                .debug_tuple("F32")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::F64(v) => f
                .debug_tuple("F64")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::Str(v) => f
                .debug_tuple("Str")
                .field(&v.name)
                .field(&FmtOptVec(&v.materialize()))
                .finish(),
            Self::SharedDict(shared_dict) => f
                .debug_tuple("SharedDict")
                .field(&shared_dict.prefix)
                .field(&shared_dict.items)
                .finish(),
        }
    }
}

impl<T: Copy + PartialEq> ParsedScalar<'_, T> {
    /// Materialize the borrowed name into an owned `'static` scalar.
    #[must_use]
    pub fn into_static(self) -> ParsedScalar<'static, T> {
        ParsedScalar {
            name: Cow::Owned(self.name.into_owned()),
            values: self.values,
        }
    }
}

impl ParsedStrings<'_> {
    /// Materialize all borrowed fields into an owned `'static` instance.
    #[must_use]
    pub fn into_static(self) -> ParsedStrings<'static> {
        ParsedStrings {
            name: Cow::Owned(self.name.into_owned()),
            lengths: self.lengths,
            data: Cow::Owned(self.data.into_owned()),
        }
    }
}

impl ParsedSharedDict<'_> {
    /// Materialize all borrowed fields into an owned `'static` instance.
    #[must_use]
    pub fn into_static(self) -> ParsedSharedDict<'static> {
        ParsedSharedDict {
            prefix: Cow::Owned(self.prefix.into_owned()),
            data: Cow::Owned(self.data.into_owned()),
            items: self
                .items
                .into_iter()
                .map(|item| ParsedSharedDictItem {
                    suffix: Cow::Owned(item.suffix.into_owned()),
                    ranges: item.ranges,
                })
                .collect(),
        }
    }
}

impl ParsedProperty<'_> {
    /// Consume this property, materializing all borrowed data into `'static` storage.
    #[must_use]
    pub fn into_static(self) -> ParsedProperty<'static> {
        use ParsedProperty as P;
        match self {
            Self::Bool(v) => P::Bool(v.into_static()),
            Self::I8(v) => P::I8(v.into_static()),
            Self::U8(v) => P::U8(v.into_static()),
            Self::I32(v) => P::I32(v.into_static()),
            Self::U32(v) => P::U32(v.into_static()),
            Self::I64(v) => P::I64(v.into_static()),
            Self::U64(v) => P::U64(v.into_static()),
            Self::F32(v) => P::F32(v.into_static()),
            Self::F64(v) => P::F64(v.into_static()),
            Self::Str(v) => P::Str(v.into_static()),
            Self::SharedDict(v) => P::SharedDict(v.into_static()),
        }
    }
}

// for impl_decodable
impl Default for ParsedProperty<'_> {
    fn default() -> Self {
        Self::Bool(ParsedScalar::new("", Vec::new()))
    }
}
