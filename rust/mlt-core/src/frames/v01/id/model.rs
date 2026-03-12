use borrowme::borrowme;
use enum_dispatch::enum_dispatch;

use crate::analyse::{Analyze, StatType};
use crate::v01::Stream;

/// ID column representation, either encoded or decoded.
#[borrowme]
#[derive(Debug, PartialEq)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
#[enum_dispatch(Analyze)]
pub enum Id<'a> {
    Encoded(EncodedId<'a>),
    Decoded(DecodedId),
}

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
