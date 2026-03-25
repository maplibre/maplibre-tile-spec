use std::fmt;

use crate::frames::v01::Layer01;
use crate::v01::{EncodedLayer01, StagedLayer01, StagedLayer01Encoder, Tag01Profile};
use crate::{DecodeState, Lazy, Parsed};

/// A layer that can be one of the known types, or an unknown.
///
/// The decode-state type parameter `S` mirrors [`Layer01<'a, S>`]:
/// - `Layer<'a>` / `Layer<'a, Lazy>` — freshly parsed; columns may still be raw bytes.
/// - `Layer<'a, Parsed>` — returned by [`Layer::decode_all`]; all columns are decoded. Use `ParsedLayer` alias.
pub enum Layer<'a, S: DecodeState = Lazy> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a, S>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}
pub type ParsedLayer<'a> = Layer<'a, Parsed>;

impl<'a, S: DecodeState> fmt::Debug for Layer<'a, S>
where
    Layer01<'a, S>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag01(l) => f.debug_tuple("Tag01").field(l).finish(),
            Self::Unknown(u) => f.debug_tuple("Unknown").field(u).finish(),
        }
    }
}

impl<'a, S: DecodeState> PartialEq for Layer<'a, S>
where
    Layer01<'a, S>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Tag01(a), Self::Tag01(b)) => a == b,
            (Self::Unknown(a), Self::Unknown(b)) => a == b,
            _ => false,
        }
    }
}

/// Owned, pre-encoding variant of [`Layer`] (stage 2 of the encoding pipeline).
#[derive(Debug, PartialEq, Clone)]
#[expect(clippy::large_enum_variant)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StagedLayer {
    Tag01(StagedLayer01),
    Unknown(EncodedUnknown),
}

/// Wire-ready variant of a layer (stage 3 of the encoding pipeline).
#[derive(Debug, PartialEq, Clone)]
#[expect(clippy::large_enum_variant)]
pub enum EncodedLayer {
    Tag01(EncodedLayer01),
    Unknown(EncodedUnknown),
}

/// Unknown layer data, stored as encoded bytes
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

/// Owned variant of [`Unknown`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncodedUnknown {
    pub tag: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum LayerEncoder {
    Tag01(StagedLayer01Encoder),
    Unknown,
}

/// Profile for a layer, built by running automatic optimization over a
/// representative sample of tiles and capturing the chosen encoders.
///
/// The `SortStrategy` stored inside the inner [`Tag01Profile`] is recorded
/// so that profile-driven encoding can reproduce the same feature ordering on
/// subsequent tiles.
#[derive(Debug, Clone)]
pub enum LayerProfile {
    Tag01(Tag01Profile),
    Unknown,
}
