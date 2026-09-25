//! Version-agnostic decode-side layer types.
//!
//! [`Layer01`] is the in-memory columnar form for *both* tag `0x01` and tag
//! `0x02` layers - the two differ only in wire format, which lives in
//! [`super::model01`] and `model02`.

use crate::decoder::model01::Layer01;
#[cfg(feature = "unstable-v2")]
use crate::decoder::model02::Layer02;
use crate::{DecodeState, Lazy, Parsed};

/// A layer that can be one of the known types, or an unknown.
///
/// The decode-state type parameter `S` mirrors [`Layer01<'a, S>`]:
/// - `Layer<'a>` / `Layer<'a, Lazy>` - freshly parsed; columns may still be raw bytes.
/// - `Layer<'a, Parsed>` - returned by [`Layer::decode_all`]; all columns are decoded. Use `ParsedLayer` alias.
#[derive(Debug)]
#[non_exhaustive]
pub enum Layer<'a, S: DecodeState = Lazy> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a, S>),
    /// Experimental v2 layer (tag = 2).
    ///
    /// Carries the same columnar representation as `Tag01` plus the columns only
    /// v2 has, in a more compact wire format.
    #[cfg(feature = "unstable-v2")]
    Tag02(Layer02<'a, S>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}
pub type ParsedLayer<'a> = Layer<'a, Parsed>;

/// Unknown layer data, stored as encoded bytes.
///
/// Returned inside [`Layer::Unknown`] for any layer tag that is not recognized
/// by this version of the library. Consumers can inspect the tag and raw bytes
/// to forward or log the layer without losing data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub(crate) tag: u8,
    pub(crate) value: &'a [u8],
}

impl<'a> Unknown<'a> {
    /// The raw layer tag identifying this unrecognised layer type.
    #[must_use]
    pub fn tag(&self) -> u32 {
        u32::from(self.tag)
    }

    /// The raw encoded bytes of this layer's body.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.value
    }
}
