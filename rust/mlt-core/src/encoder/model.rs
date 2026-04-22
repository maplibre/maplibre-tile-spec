use std::fmt;

use crate::decoder::{GeometryValues, IdValues, StreamType};
use crate::encoder::geometry::VertexBufferType;
use crate::encoder::{IdWidth, IntEncoder, StagedProperty};

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

/// Parameters derived from the vertex set of a feature collection, used to
/// normalize coordinates before space-filling-curve key computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveParams {
    pub shift: u32,
    pub bits: u32,
}

/// Columnar layer data being prepared for encoding (stage 2 of the encoding pipeline).
///
/// Holds fully-owned columnar data. Constructed directly (synthetics, benches) or
/// converted from [`TileLayer01`](crate::TileLayer01).
/// Consumed by encoding via [`StagedLayer::encode_into`] or `StagedLayer01::encode_explicit`
/// (with explicit encoding mode enabled).
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnKind {
    Id,
    Geometry,
    Property,
}

/// Context for per-stream encoding decisions in [`ExplicitEncoder`] callbacks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamCtx<'a> {
    pub kind: ColumnKind,
    pub stream_type: StreamType,
    pub name: &'a str,
    pub subname: &'a str,
}
impl<'a> StreamCtx<'a> {
    /// Stream with a logical sub-part (e.g. string column `"lengths"` / `"offsets"`, shared-dict child suffix).
    #[inline]
    #[must_use]
    pub const fn new(
        kind: ColumnKind,
        stream_type: StreamType,
        name: &'a str,
        subname: &'a str,
    ) -> Self {
        Self {
            kind,
            stream_type,
            name,
            subname,
        }
    }

    #[inline]
    #[must_use]
    pub const fn id(stream_type: StreamType) -> Self {
        Self::new(ColumnKind::Id, stream_type, "", "")
    }

    #[inline]
    #[must_use]
    pub const fn geom(stream_type: StreamType, name: &'a str) -> Self {
        Self::new(ColumnKind::Geometry, stream_type, name, "")
    }

    #[inline]
    #[must_use]
    pub const fn prop(stream_type: StreamType, name: &'a str) -> Self {
        Self::new(ColumnKind::Property, stream_type, name, "")
    }

    #[inline]
    #[must_use]
    pub const fn prop2(stream_type: StreamType, prefix: &'a str, suffix: &'a str) -> Self {
        Self::new(ColumnKind::Property, stream_type, prefix, suffix)
    }
}

/// Explicit, deterministic encoding configuration for synthetics and tests.
///
/// All encoding choices are caller-specified via callbacks so one struct can cover
/// any combination without per-stream boilerplate.
///
/// Always compiled; publicly visible only when the `__private` feature is enabled
/// (re-exported from [`crate::encoder`]).
pub struct ExplicitEncoder {
    /// Vertex buffer layout for geometry streams.
    pub vertex_buffer_type: VertexBufferType,
    /// Per-stream override for the skip-empty-stream rule used by `write_geo_u32_stream`.
    pub force_stream: Box<dyn for<'a> Fn(&'a StreamCtx<'a>) -> bool>,
    /// Return the [`IntEncoder`] for a stream identified by [`StreamCtx`].
    pub get_int_encoder: Box<dyn for<'a> Fn(&'a StreamCtx<'a>) -> IntEncoder>,
    /// Return the string encoding strategy for a string property column.
    pub get_str_encoding: Box<dyn Fn(&str) -> StrEncoding>,
    /// Override the auto-detected [`IdWidth`].
    /// Arguments: auto-detected `IdWidth`. Return the width to use.
    pub override_id_width: Box<dyn Fn(IdWidth) -> IdWidth>,
    /// Override whether a presence stream is written for an all-present column,
    /// or if the column is written at all if all values are null.
    pub override_presence: Box<dyn for<'a> Fn(&'a StreamCtx<'a>) -> bool>,
}

impl fmt::Debug for ExplicitEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExplicitEncoder").finish_non_exhaustive()
    }
}
