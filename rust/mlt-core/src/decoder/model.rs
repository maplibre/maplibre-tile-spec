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

/// The bytes a layer spends on the wire, by what they hold.
///
/// Every byte the columns leave over is layer metadata: the tag, name, header and column schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerBytes {
    pub(crate) size: u32,
    pub(crate) geometry: u32,
    pub(crate) properties: u32,
    pub(crate) ids: u32,
}

impl LayerBytes {
    /// Start counting a layer whose body is `body_len` bytes.
    pub(crate) fn of_body(body_len: usize) -> Self {
        Self {
            // The body never exceeds the u32 size varint that framed it.
            size: u32::try_from(body_len).map_or(u32::MAX, |len| len.saturating_add(1)),
            ..Self::default()
        }
    }

    /// The value of the layer's size varint: the tag and the body, without the varint itself.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The geometry column, or the v2 geometry section.
    #[must_use]
    pub fn geometry(&self) -> u32 {
        self.geometry
    }

    /// Every column that is neither geometry nor the id, nested and m-value columns included.
    #[must_use]
    pub fn properties(&self) -> u32 {
        self.properties
    }

    /// The id column.
    #[must_use]
    pub fn ids(&self) -> u32 {
        self.ids
    }

    /// What is left of [`Self::size`] once every column is taken out.
    #[must_use]
    pub fn metadata(&self) -> u32 {
        self.size - self.geometry - self.properties - self.ids
    }
}

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
