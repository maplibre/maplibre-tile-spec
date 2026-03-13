use crate::frames::v01::Layer01;
use crate::v01::{OwnedLayer01, SortStrategy, Tag01Encoder, Tag01Profile};

/// A layer that can be one of the known types, or an unknown
#[derive(Debug, PartialEq)]
#[expect(clippy::large_enum_variant)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

/// Owned variant of [`Layer`].
#[derive(Debug, PartialEq, Clone)]
#[expect(clippy::large_enum_variant)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum OwnedLayer {
    Tag01(OwnedLayer01),
    Unknown(OwnedUnknown),
}

/// Unknown layer data, stored as encoded bytes
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

/// Owned variant of [`Unknown`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnedUnknown {
    pub tag: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum LayerEncoder {
    Tag01(Tag01Encoder),
    Unknown,
}

impl LayerEncoder {
    /// Return the active sort strategy, or [`None`] for unknown layers.
    #[must_use]
    pub fn sort_strategy(&self) -> Option<SortStrategy> {
        match self {
            LayerEncoder::Tag01(enc) => enc.sort_strategy,
            LayerEncoder::Unknown => None,
        }
    }
}

/// Profile for a layer, built by running automatic optimisation over a
/// representative sample of tiles and capturing the chosen encoders.
///
/// The [`SortStrategy`] stored inside the inner [`Tag01Profile`] is recorded
/// so that profile-driven encoding can reproduce the same feature ordering on
/// subsequent tiles.
#[derive(Debug, Clone)]
pub enum LayerProfile {
    Tag01(Tag01Profile),
    Unknown,
}

impl LayerProfile {
    /// Return the active sort strategy, or [`None`] for unknown layers.
    #[must_use]
    pub fn sort_strategy(&self) -> Option<SortStrategy> {
        match self {
            LayerProfile::Tag01(p) => p.sort_strategy(),
            LayerProfile::Unknown => None,
        }
    }
}
