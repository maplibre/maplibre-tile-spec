use crate::decoder::{GeometryValues, IdValues};
use crate::encoder::StagedProperty;

/// Owned, pre-encoding variant of [`crate::Layer`] (stage 2 of the encoding pipeline).
#[derive(Debug, PartialEq, Clone)]
#[expect(clippy::large_enum_variant)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StagedLayer {
    Tag01(StagedLayer01),
    Unknown(EncodedUnknown),
}

/// Wire-ready variant of a layer that cannot be decoded to Tag01.
///
/// Tag01 layers are encoded directly into [`Encoder`](crate::encoder::Encoder) buffers
/// via [`StagedLayer::encode_into`] or [`StagedLayer01::encode_with`].
#[derive(Debug, PartialEq, Clone)]
pub enum EncodedLayer {
    Unknown(EncodedUnknown),
}

/// Owned variant of `Unknown`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncodedUnknown {
    pub(crate) tag: u8,
    pub(crate) value: Vec<u8>,
}

/// Columnar layer data being prepared for encoding (stage 2 of the encoding pipeline).
///
/// Holds fully-owned columnar data. Constructed directly (synthetics, benches) or
/// converted from [`TileLayer01`](crate::TileLayer01).
/// Consumed by encoding via [`StagedLayer::encode_into`] or [`StagedLayer01::encode_with`].
#[derive(Debug, PartialEq, Clone)]
pub struct StagedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<IdValues>,
    pub geometry: GeometryValues,
    pub properties: Vec<StagedProperty>,
}
