use std::borrow::Cow;
use std::fmt::{self, Debug};

use crate::utils::FmtOptVec;
use crate::v01::{ParsedProperty, ParsedScalar};

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
    #[must_use]
    pub fn to_owned_cow(&self) -> ParsedScalar<'static, T> {
        ParsedScalar {
            name: Cow::Owned(self.name.as_ref().to_string()),
            values: self.values.clone(),
        }
    }
}

impl ParsedProperty<'_> {
    /// Clone this property into a fully-owned `ParsedProperty<'static>`.
    /// TODO: This should be removed later, once we separate parsing and staging
    #[must_use]
    pub fn to_owned(&self) -> ParsedProperty<'static> {
        use ParsedProperty as P;
        match self {
            Self::Bool(v) => P::Bool(v.to_owned_cow()),
            Self::I8(v) => P::I8(v.to_owned_cow()),
            Self::U8(v) => P::U8(v.to_owned_cow()),
            Self::I32(v) => P::I32(v.to_owned_cow()),
            Self::U32(v) => P::U32(v.to_owned_cow()),
            Self::I64(v) => P::I64(v.to_owned_cow()),
            Self::U64(v) => P::U64(v.to_owned_cow()),
            Self::F32(v) => P::F32(v.to_owned_cow()),
            Self::F64(v) => P::F64(v.to_owned_cow()),
            Self::Str(v) => P::Str(v.to_owned()),
            Self::SharedDict(v) => P::SharedDict(v.to_owned()),
        }
    }
}

// for impl_decodable
impl Default for ParsedProperty<'_> {
    fn default() -> Self {
        Self::Bool(ParsedScalar::new("", Vec::new()))
    }
}
