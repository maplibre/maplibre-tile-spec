use std::borrow::Cow;
use std::fmt::{self, Debug};

use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowMe};

use crate::utils::FmtOptVec;
use crate::v01::{DecodedOptScalar, DecodedProperty, DecodedScalar};

impl Debug for DecodedProperty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => f
                .debug_tuple("Bool")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::BoolOpt(v) => f
                .debug_tuple("BoolOpt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I8(v) => f.debug_tuple("I8").field(&v.name).field(&v.values).finish(),
            Self::I8Opt(v) => f
                .debug_tuple("I8Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U8(v) => f.debug_tuple("U8").field(&v.name).field(&v.values).finish(),
            Self::U8Opt(v) => f
                .debug_tuple("U8Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I32(v) => f
                .debug_tuple("I32")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::I32Opt(v) => f
                .debug_tuple("I32Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U32(v) => f
                .debug_tuple("U32")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::U32Opt(v) => f
                .debug_tuple("U32Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::I64(v) => f
                .debug_tuple("I64")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::I64Opt(v) => f
                .debug_tuple("I64Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::U64(v) => f
                .debug_tuple("U64")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::U64Opt(v) => f
                .debug_tuple("U64Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::F32(v) => f
                .debug_tuple("F32")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::F32Opt(v) => f
                .debug_tuple("F32Opt")
                .field(&v.name)
                .field(&FmtOptVec(&v.values))
                .finish(),
            Self::F64(v) => f
                .debug_tuple("F64")
                .field(&v.name)
                .field(&v.values)
                .finish(),
            Self::F64Opt(v) => f
                .debug_tuple("F64Opt")
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

impl<T: Copy + PartialEq> BorrowMe for DecodedScalar<'_, T> {
    type Owned = DecodedScalar<'static, T>;

    fn to_owned(&self) -> Self::Owned {
        DecodedScalar {
            name: Cow::Owned(self.name.as_ref().to_string()),
            values: self.values.clone(),
        }
    }
}

impl<T: Copy + PartialEq> BorrowMe for DecodedOptScalar<'_, T> {
    type Owned = DecodedOptScalar<'static, T>;

    fn to_owned(&self) -> Self::Owned {
        DecodedOptScalar {
            name: Cow::Owned(self.name.as_ref().to_string()),
            values: self.values.clone(),
        }
    }
}

impl BorrowMe for DecodedProperty<'_> {
    type Owned = DecodedProperty<'static>;

    fn to_owned(&self) -> Self::Owned {
        use DecodedProperty as P;
        match self {
            Self::Bool(v) => P::Bool(BorrowMe::to_owned(v)),
            Self::BoolOpt(v) => P::BoolOpt(BorrowMe::to_owned(v)),
            Self::I8(v) => P::I8(BorrowMe::to_owned(v)),
            Self::I8Opt(v) => P::I8Opt(BorrowMe::to_owned(v)),
            Self::U8(v) => P::U8(BorrowMe::to_owned(v)),
            Self::U8Opt(v) => P::U8Opt(BorrowMe::to_owned(v)),
            Self::I32(v) => P::I32(BorrowMe::to_owned(v)),
            Self::I32Opt(v) => P::I32Opt(BorrowMe::to_owned(v)),
            Self::U32(v) => P::U32(BorrowMe::to_owned(v)),
            Self::U32Opt(v) => P::U32Opt(BorrowMe::to_owned(v)),
            Self::I64(v) => P::I64(BorrowMe::to_owned(v)),
            Self::I64Opt(v) => P::I64Opt(BorrowMe::to_owned(v)),
            Self::U64(v) => P::U64(BorrowMe::to_owned(v)),
            Self::U64Opt(v) => P::U64Opt(BorrowMe::to_owned(v)),
            Self::F32(v) => P::F32(BorrowMe::to_owned(v)),
            Self::F32Opt(v) => P::F32Opt(BorrowMe::to_owned(v)),
            Self::F64(v) => P::F64(BorrowMe::to_owned(v)),
            Self::F64Opt(v) => P::F64Opt(BorrowMe::to_owned(v)),
            Self::Str(v) => P::Str(BorrowMe::to_owned(v)),
            Self::SharedDict(v) => P::SharedDict(BorrowMe::to_owned(v)),
        }
    }
}

impl BorrowmeBorrow for DecodedProperty<'static> {
    type Target<'a>
        = DecodedProperty<'a>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        use DecodedOptScalar as O;
        use DecodedProperty as P;
        use DecodedScalar as S;
        match self {
            Self::Bool(v) => P::Bool(S::new(v.name.clone(), v.values.clone())),
            Self::BoolOpt(v) => P::BoolOpt(O::new(v.name.clone(), v.values.clone())),
            Self::I8(v) => P::I8(S::new(v.name.clone(), v.values.clone())),
            Self::I8Opt(v) => P::I8Opt(O::new(v.name.clone(), v.values.clone())),
            Self::U8(v) => P::U8(S::new(v.name.clone(), v.values.clone())),
            Self::U8Opt(v) => P::U8Opt(O::new(v.name.clone(), v.values.clone())),
            Self::I32(v) => P::I32(S::new(v.name.clone(), v.values.clone())),
            Self::I32Opt(v) => P::I32Opt(O::new(v.name.clone(), v.values.clone())),
            Self::U32(v) => P::U32(S::new(v.name.clone(), v.values.clone())),
            Self::U32Opt(v) => P::U32Opt(O::new(v.name.clone(), v.values.clone())),
            Self::I64(v) => P::I64(S::new(v.name.clone(), v.values.clone())),
            Self::I64Opt(v) => P::I64Opt(O::new(v.name.clone(), v.values.clone())),
            Self::U64(v) => P::U64(S::new(v.name.clone(), v.values.clone())),
            Self::U64Opt(v) => P::U64Opt(O::new(v.name.clone(), v.values.clone())),
            Self::F32(v) => P::F32(S::new(v.name.clone(), v.values.clone())),
            Self::F32Opt(v) => P::F32Opt(O::new(v.name.clone(), v.values.clone())),
            Self::F64(v) => P::F64(S::new(v.name.clone(), v.values.clone())),
            Self::F64Opt(v) => P::F64Opt(O::new(v.name.clone(), v.values.clone())),
            Self::Str(v) => P::Str(BorrowmeBorrow::borrow(v)),
            Self::SharedDict(v) => P::SharedDict(BorrowmeBorrow::borrow(v)),
        }
    }
}

// for impl_decodable
impl Default for DecodedProperty<'_> {
    fn default() -> Self {
        Self::Bool(DecodedScalar::new("", Vec::new()))
    }
}
