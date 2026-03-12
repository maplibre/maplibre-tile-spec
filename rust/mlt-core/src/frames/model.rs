use borrowme::borrowme;

use crate::frames::v01::Layer01;
use crate::v01::{SortStrategy, Tag01Encoder, Tag01Profile};

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

impl LayerEncoder {
    /// Set the [`SortStrategy`] on a `Tag01` encoder, returning `self` for chaining.
    /// Has no effect on [`LayerEncoder::Unknown`].
    #[must_use]
    pub fn with_sort(mut self, strategy: SortStrategy) -> Self {
        if let LayerEncoder::Tag01(ref mut enc) = self {
            enc.sort_strategy = strategy;
        }
        self
    }

    /// Return the active [`SortStrategy`], or [`SortStrategy::None`] for unknown layers.
    #[must_use]
    pub fn sort_strategy(&self) -> SortStrategy {
        match self {
            LayerEncoder::Tag01(enc) => enc.sort_strategy,
            LayerEncoder::Unknown => SortStrategy::None,
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
    /// Set the [`SortStrategy`] on a `Tag01` profile, returning `self` for
    /// chaining.  Has no effect on [`LayerProfile::Unknown`].
    #[must_use]
    pub fn with_sort(mut self, strategy: SortStrategy) -> Self {
        if let LayerProfile::Tag01(ref mut p) = self {
            p.preferred_sort_strategy = strategy;
        }
        self
    }

    /// Return the active [`SortStrategy`], or [`SortStrategy::None`] for
    /// unknown layers.
    #[must_use]
    pub fn sort_strategy(&self) -> SortStrategy {
        match self {
            LayerProfile::Tag01(p) => p.preferred_sort_strategy,
            LayerProfile::Unknown => SortStrategy::None,
        }
    }
}
