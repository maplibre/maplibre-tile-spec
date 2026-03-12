use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowmeToOwned};

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decode::{Decodable, Decode};
use crate::encode::Encodable;

/// Shared wrapper for values that may still be encoded or already decoded.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum EncDec<Encoded, Decoded> {
    Encoded(Encoded),
    Decoded(Decoded),
}

impl<Encoded, Decoded> From<Encoded> for EncDec<Encoded, Decoded> {
    fn from(encoded: Encoded) -> Self {
        Self::Encoded(encoded)
    }
}

impl<Encoded, Decoded> Encodable for EncDec<Encoded, Decoded> {
    type DecodedType = Decoded;
    type EncodedType = Encoded;

    fn is_decoded(&self) -> bool {
        matches!(self, Self::Decoded(_))
    }

    fn new_encoded(encoded: Self::EncodedType) -> Self {
        Self::Encoded(encoded)
    }

    fn borrow_encoded(&self) -> Option<&Self::EncodedType> {
        if let Self::Encoded(encoded) = self {
            Some(encoded)
        } else {
            None
        }
    }
}

impl<Encoded, Decoded> Decodable<'_> for EncDec<Encoded, Decoded>
where
    Decoded: Decode<Encoded> + Default,
{
    type EncodedType = Encoded;
    type DecodedType = Decoded;

    fn is_encoded(&self) -> bool {
        matches!(self, Self::Encoded(_))
    }

    fn new_decoded(decoded: Self::DecodedType) -> Self {
        Self::Decoded(decoded)
    }

    fn take_encoded(&mut self) -> Option<Self::EncodedType> {
        if let Self::Encoded(encoded) =
            std::mem::replace(self, Self::Decoded(Self::DecodedType::default()))
        {
            Some(encoded)
        } else {
            None
        }
    }

    fn borrow_decoded(&self) -> Option<&Self::DecodedType> {
        if let Self::Decoded(decoded) = self {
            Some(decoded)
        } else {
            None
        }
    }

    fn borrow_decoded_mut(&mut self) -> Option<&mut Self::DecodedType> {
        if let Self::Decoded(decoded) = self {
            Some(decoded)
        } else {
            None
        }
    }
}

impl<Encoded, Decoded> Analyze for EncDec<Encoded, Decoded>
where
    Encoded: Analyze,
    Decoded: Analyze,
{
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(encoded) => encoded.collect_statistic(stat),
            Self::Decoded(decoded) => decoded.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&crate::v01::Stream<'_>)) {
        match self {
            Self::Encoded(encoded) => encoded.for_each_stream(cb),
            Self::Decoded(decoded) => decoded.for_each_stream(cb),
        }
    }
}

impl<Encoded, Decoded> BorrowmeToOwned for EncDec<Encoded, Decoded>
where
    Encoded: BorrowmeToOwned,
    Decoded: BorrowmeToOwned,
{
    type Owned = EncDec<Encoded::Owned, Decoded::Owned>;

    fn to_owned(&self) -> Self::Owned {
        match self {
            Self::Encoded(encoded) => EncDec::Encoded(BorrowmeToOwned::to_owned(encoded)),
            Self::Decoded(decoded) => EncDec::Decoded(BorrowmeToOwned::to_owned(decoded)),
        }
    }
}

impl<Encoded, Decoded> BorrowmeBorrow for EncDec<Encoded, Decoded>
where
    Encoded: BorrowmeBorrow,
    Decoded: BorrowmeBorrow,
{
    type Target<'a>
        = EncDec<Encoded::Target<'a>, Decoded::Target<'a>>
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        match self {
            Self::Encoded(encoded) => EncDec::Encoded(BorrowmeBorrow::borrow(encoded)),
            Self::Decoded(decoded) => EncDec::Decoded(BorrowmeBorrow::borrow(decoded)),
        }
    }
}

impl<Encoded, Decoded> EncDec<Encoded, Decoded>
where
    Decoded: Decode<Encoded>,
{
    pub(crate) fn into_decoded(self) -> Result<Decoded, MltError> {
        match self {
            Self::Encoded(encoded) => Decode::decode(encoded),
            Self::Decoded(decoded) => Ok(decoded),
        }
    }
}
