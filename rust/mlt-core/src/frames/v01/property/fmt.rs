use std::borrow::Cow;
use std::fmt::{self, Debug};

use crate::utils::FmtOptVec;
use crate::v01::{DecodedProperty, DecodedScalar};

impl Debug for DecodedProperty<'_> {
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

impl<T: Copy + PartialEq> DecodedScalar<'_, T> {
    #[must_use]
    pub fn to_owned(&self) -> DecodedScalar<'static, T> {
        DecodedScalar {
            name: Cow::Owned(self.name.as_ref().to_string()),
            values: self.values.clone(),
        }
    }
}

impl DecodedProperty<'_> {
    #[must_use]
    pub fn to_owned(&self) -> DecodedProperty<'static> {
        use DecodedProperty as P;
        match self {
            Self::Bool(v) => P::Bool(v.to_owned()),
            Self::I8(v) => P::I8(v.to_owned()),
            Self::U8(v) => P::U8(v.to_owned()),
            Self::I32(v) => P::I32(v.to_owned()),
            Self::U32(v) => P::U32(v.to_owned()),
            Self::I64(v) => P::I64(v.to_owned()),
            Self::U64(v) => P::U64(v.to_owned()),
            Self::F32(v) => P::F32(v.to_owned()),
            Self::F64(v) => P::F64(v.to_owned()),
            Self::Str(v) => P::Str(v.to_owned()),
            Self::SharedDict(v) => P::SharedDict(v.to_owned()),
        }
    }
}

impl DecodedProperty<'static> {
    #[must_use]
    pub fn as_borrowed(&self) -> DecodedProperty<'_> {
        use DecodedProperty as P;
        use DecodedScalar as S;
        match self {
            Self::Bool(v) => P::Bool(S::new(v.name.clone(), v.values.clone())),
            Self::I8(v) => P::I8(S::new(v.name.clone(), v.values.clone())),
            Self::U8(v) => P::U8(S::new(v.name.clone(), v.values.clone())),
            Self::I32(v) => P::I32(S::new(v.name.clone(), v.values.clone())),
            Self::U32(v) => P::U32(S::new(v.name.clone(), v.values.clone())),
            Self::I64(v) => P::I64(S::new(v.name.clone(), v.values.clone())),
            Self::U64(v) => P::U64(S::new(v.name.clone(), v.values.clone())),
            Self::F32(v) => P::F32(S::new(v.name.clone(), v.values.clone())),
            Self::F64(v) => P::F64(S::new(v.name.clone(), v.values.clone())),
            Self::Str(v) => P::Str(v.as_borrowed()),
            Self::SharedDict(v) => P::SharedDict(v.as_borrowed()),
        }
    }
}

// for impl_decodable
impl Default for DecodedProperty<'_> {
    fn default() -> Self {
        Self::Bool(DecodedScalar::new("", Vec::new()))
    }
}
