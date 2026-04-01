use crate::v01::{EncodedLayer01, StagedLayer01, Tile01Encoder};

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

/// Owned variant of [`Unknown`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncodedUnknown {
    pub(crate) tag: u8,
    pub(crate) value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum LayerEncoder {
    Tag01(Tile01Encoder),
    Unknown,
}
