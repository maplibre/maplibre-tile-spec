use crate::encoder::optimizer::Tile01Encoder;
use crate::encoder::{EncodedGeometry, EncodedId, EncodedProperty, StagedProperty};
use crate::v01::{GeometryValues, IdValues};

/// Owned, pre-encoding variant of [`crate::Layer`] (stage 2 of the encoding pipeline).
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

/// Owned variant of `Unknown`.
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

/// Columnar layer data being prepared for encoding (stage 2 of the encoding pipeline).
///
/// Holds fully-owned columnar data. Constructed directly (synthetics, benches) or
/// converted from [`TileLayer01`].
/// Consumed by encoding to produce [`EncodedLayer01`].
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct StagedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<IdValues>,
    pub geometry: GeometryValues,
    pub properties: Vec<StagedProperty>,
}

/// Wire-ready layer data (stage 3 of the encoding pipeline).
///
/// Produced by encoding a [`StagedLayer01`]. Can be serialized directly to bytes
/// via [`EncodedLayer01::write_to`].
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedLayer01 {
    pub(crate) name: String,
    pub(crate) extent: u32,
    pub(crate) id: Option<EncodedId>,
    pub(crate) geometry: EncodedGeometry,
    pub(crate) properties: Vec<EncodedProperty>,
    #[cfg(fuzzing)]
    pub(crate) layer_order: Vec<crate::frames::v01::fuzzing::LayerOrdering>,
}
