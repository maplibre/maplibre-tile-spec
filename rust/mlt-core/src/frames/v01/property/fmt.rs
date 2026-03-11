use std::fmt::{self, Debug};

use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowmeToOwned};

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

impl BorrowmeToOwned for DecodedProperty<'_> {
    type Owned = DecodedProperty<'static>;

    fn to_owned(&self) -> Self::Owned {
        match self {
            Self::Bool(v) => DecodedProperty::Bool(v.clone()),
            Self::I8(v) => DecodedProperty::I8(v.clone()),
            Self::U8(v) => DecodedProperty::U8(v.clone()),
            Self::I32(v) => DecodedProperty::I32(v.clone()),
            Self::U32(v) => DecodedProperty::U32(v.clone()),
            Self::I64(v) => DecodedProperty::I64(v.clone()),
            Self::U64(v) => DecodedProperty::U64(v.clone()),
            Self::F32(v) => DecodedProperty::F32(v.clone()),
            Self::F64(v) => DecodedProperty::F64(v.clone()),
            Self::Str(values) => DecodedProperty::Str(BorrowmeToOwned::to_owned(values)),
            Self::SharedDict(shared_dict) => {
                DecodedProperty::SharedDict(BorrowmeToOwned::to_owned(shared_dict))
            }
        }
    }
}

impl BorrowmeBorrow for DecodedProperty<'static> {
    type Target<'a>
        = DecodedProperty<'a>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
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
            Self::Str(values) => P::Str(BorrowmeBorrow::borrow(values)),
            Self::SharedDict(shared_dict) => P::SharedDict(BorrowmeBorrow::borrow(shared_dict)),
        }
    }
}

// for impl_decodable
impl Default for DecodedProperty<'_> {
    fn default() -> Self {
        Self::Bool(DecodedScalar::new(String::new(), Vec::new()))
    }
}
