use crate::frames::v01::Layer01;
use crate::v01::{OwnedLayer01, Tag01Encoder, Tag01Profile};

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

impl Unknown<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedUnknown {
        OwnedUnknown {
            tag: self.tag,
            value: self.value.to_vec(),
        }
    }
}

impl Layer<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedLayer {
        match self {
            Self::Tag01(layer) => OwnedLayer::Tag01(layer.to_owned()),
            Self::Unknown(unknown) => OwnedLayer::Unknown(unknown.to_owned()),
        }
    }
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
