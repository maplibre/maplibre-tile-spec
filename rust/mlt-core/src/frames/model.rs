use borrowme::borrowme;

use crate::frames::v01::Layer01;
use crate::v01::{Tag01Encoder, Tag01Profile};

/// A layer that can be one of the known types, or an unknown
#[borrowme]
#[derive(Debug, PartialEq)]
#[expect(clippy::large_enum_variant)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

/// Unknown layer data, stored as encoded bytes
#[borrowme]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub value: &'a [u8],
}

#[derive(Debug, Clone)]
pub enum LayerEncoder {
    Tag01(Tag01Encoder),
    Unknown,
}

#[derive(Debug, Clone)]
pub enum LayerProfile {
    Tag01(Tag01Profile),
    Unknown,
}
