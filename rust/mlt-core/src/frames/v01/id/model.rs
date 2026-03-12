use borrowme::{Borrow as BorrowmeBorrow, ToOwned as BorrowmeToOwned, borrowme};

use crate::EncDec;
use crate::v01::Stream;

/// ID column representation, either encoded or decoded.
pub type Id<'a> = EncDec<EncodedId<'a>, DecodedId>;

/// Owned ID column representation, either encoded or decoded.
pub type OwnedId = EncDec<OwnedEncodedId, DecodedId>;

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedId<'a> {
    pub(crate) presence: Option<Stream<'a>>,
    pub(crate) value: EncodedIdValue<'a>,
}

/// A sequence of encoded ID values, either 32-bit or 64-bit unsigned integers
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

/// Decoded ID values as a vector of optional 64-bit unsigned integers
#[derive(Clone, Default, PartialEq)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct DecodedId(pub Vec<Option<u64>>);

impl BorrowmeToOwned for DecodedId {
    type Owned = Self;

    fn to_owned(&self) -> Self::Owned {
        self.clone()
    }
}

impl BorrowmeBorrow for DecodedId {
    type Target<'a>
        = Self
    where
        Self: 'a;

    fn borrow(&self) -> Self::Target<'_> {
        self.clone()
    }
}

/// How wide are the IDs
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum IdWidth {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}
