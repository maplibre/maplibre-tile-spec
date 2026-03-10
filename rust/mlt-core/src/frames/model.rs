use borrowme::borrowme;

use crate::frames::unknown::Unknown;
use crate::frames::v01::Layer01;

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
