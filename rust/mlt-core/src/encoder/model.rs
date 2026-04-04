use std::fmt;

use crate::decoder::{GeometryValues, IdValues};
use crate::encoder::{IntEncoder, StagedProperty};

/// Owned, pre-encoding variant of [`crate::Layer`] (stage 2 of the encoding pipeline).
#[derive(Debug, PartialEq, Clone)]
#[expect(clippy::large_enum_variant)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum StagedLayer {
    Tag01(StagedLayer01),
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
/// Consumed by encoding via [`StagedLayer::encode_into`] or `StagedLayer01::encode_explicit`
/// (with [`Encoder::explicit`](crate::encoder::Encoder::explicit) set).
#[derive(Debug, PartialEq, Clone)]
pub struct StagedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: Option<IdValues>,
    pub geometry: GeometryValues,
    pub properties: Vec<StagedProperty>,
}

/// Global encoder settings controlling which optimization strategies are attempted.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "enums would not model this better, not a state machine"
)]
pub struct EncoderConfig {
    /// Generate tessellation data for polygons and multi-polygons.
    pub tessellate: bool,
    /// Try sorting features by the Z-order (Morton) curve index of their first vertex.
    pub try_spatial_morton_sort: bool,
    /// Try sorting features by the Hilbert curve index of their first vertex.
    pub try_spatial_hilbert_sort: bool,
    /// Try sorting features by their feature ID in ascending order.
    pub try_id_sort: bool,
    /// Allow `FSST` string compression
    pub allow_fsst: bool,
    /// Allow `FastPFOR` integer compression
    pub allow_fpf: bool,
    /// Allow string grouping into shared dictionaries
    pub allow_shared_dict: bool,
}
impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            tessellate: false,
            try_spatial_morton_sort: true,
            try_spatial_hilbert_sort: true,
            try_id_sort: true,
            allow_fsst: true,
            allow_fpf: true,
            allow_shared_dict: true,
        }
    }
}

/// How to encode a string column.
///
/// Used by [`ExplicitEncoder`] to control per-column string encoding in the
/// explicit (synthetics / `__private`) path and in property-encoding helpers.
///
/// Publicly visible only when the `__private` feature is enabled (re-exported from
/// [`crate::encoder`]).  Always compiled so that the unified property-encoding path
/// can reference it without feature flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrEncoding {
    Plain,
    Dict,
    Fsst,
    FsstDict,
}

/// Explicit, deterministic encoding configuration for synthetics and tests.
///
/// All encoding choices are caller-specified via callbacks so one struct can cover
/// any combination without per-stream boilerplate.
///
/// Always compiled; publicly visible only when the `__private` feature is enabled
/// (re-exported from [`crate::encoder`]).
#[expect(
    clippy::type_complexity,
    reason = "keep it simple for internal usage without extra types"
)]
pub struct ExplicitEncoder {
    /// Vertex buffer layout for geometry streams.
    #[cfg(feature = "__private")]
    pub vertex_buffer_type: crate::encoder::VertexBufferType,
    /// Return the [`IntEncoder`] for a stream.
    /// Arguments: `(kind, name, subname)` where `kind` is `"id"`, `"geo"`, or `"prop"`;
    /// `name` is the stream/column name; `subname` is the shared-dict suffix when applicable.
    pub get_int_encoder: Box<dyn Fn(&str, &str, Option<&str>) -> IntEncoder>,
    /// Return the string encoding strategy for a string property column.
    pub get_str_encoding: Box<dyn Fn(&str, &str) -> StrEncoding>,
    /// Override the auto-detected [`IdWidth`].
    /// Arguments: auto-detected `IdWidth`. Return the width to use.
    #[cfg(feature = "__private")]
    pub override_id_width: Box<dyn Fn(crate::encoder::IdWidth) -> crate::encoder::IdWidth>,
    /// Override whether a presence stream is written for an all-present column,
    /// or if the column is written at all if all values are null.
    /// Arguments: `(kind, name, subname)` — same convention as [`Self::get_int_encoder`]
    pub override_presence: Box<dyn Fn(&str, &str, Option<&str>) -> bool>,
}

impl fmt::Debug for ExplicitEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExplicitEncoder").finish_non_exhaustive()
    }
}
