use derive_debug::Dbg;

use crate::decoder::{GeometryValues, StreamType};
use crate::encoder::geometry::VertexBufferType;
use crate::encoder::{IntEncoder, StagedId, StagedProperty};

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

impl Default for CurveParams {
    fn default() -> Self {
        Self { shift: 0, bits: 1 }
    }
}

impl CurveParams {
    /// Compute params from a flat `[x0, y0, x1, y1, …]` vertex slice.
    #[must_use]
    pub fn from_vertices(vertices: &[i32]) -> Self {
        if vertices.is_empty() {
            return Self::default();
        }
        let (min, max) = vertices
            .iter()
            .fold((i32::MAX, i32::MIN), |(mn, mx), &v| (mn.min(v), mx.max(v)));
        crate::codecs::hilbert::hilbert_curve_params_from_bounds(min, max)
    }
}

/// Columnar layer data being prepared for encoding (stage 2 of the encoding pipeline).
///
/// Holds fully-owned columnar data. Constructed directly (synthetics, benches) or
/// converted from [`TileLayer`](crate::TileLayer).
/// Consumed by encoding via [`StagedLayer::encode_into`] or `StagedLayer::encode_explicit`
/// (with explicit encoding mode enabled).
#[derive(Debug, PartialEq, Clone)]
pub struct StagedLayer {
    pub name: String,
    pub extent: u32,
    pub id: StagedId,
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
#[derive(Dbg)]
pub struct ExplicitEncoder {
    /// Vertex buffer layout for geometry streams.
    pub vertex_buffer_type: VertexBufferType,
    /// Per-stream override for the skip-empty-stream rule used by `write_geo_u32_stream`.
    #[dbg(skip)]
    pub force_stream: Box<dyn for<'a> Fn(&'a StreamCtx<'a>) -> bool>,
    /// Return the [`IntEncoder`] for a stream identified by [`StreamCtx`].
    #[dbg(skip)]
    pub get_int_encoder: Box<dyn for<'a> Fn(&'a StreamCtx<'a>) -> IntEncoder>,
    /// Return the string encoding strategy for a string property column.
    #[dbg(skip)]
    pub get_str_encoding: Box<dyn Fn(&str) -> StrEncoding>,
}
