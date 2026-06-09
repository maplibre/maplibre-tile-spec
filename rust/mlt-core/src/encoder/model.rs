use derive_debug::Dbg;

use crate::decoder::{DictionaryType, Extent, GeometryValues, StreamType};
use crate::encoder::geometry::VertexBufferType;
use crate::encoder::{IntEncoder, StagedId, StagedProperty};
use crate::{MltError, MltResult};

/// Owned variant of `Unknown`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncodedUnknown {
    pub(crate) tag: u8,
    pub(crate) value: Vec<u8>,
}

impl EncodedUnknown {
    pub fn new(tag: u8, value: Vec<u8>) -> MltResult<Self> {
        if tag == 1 {
            return Err(MltError::ParsingColumnType(tag));
        }
        Ok(Self { tag, value })
    }

    #[must_use]
    pub fn tag(&self) -> u32 {
        u32::from(self.tag)
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.value
    }
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
    pub(crate) name: String,
    pub(crate) extent: Extent,
    pub(crate) id: StagedId,
    pub(crate) geometry: GeometryValues,
    pub(crate) properties: Vec<StagedProperty>,
}

#[cfg_attr(not(feature = "__private"), allow(dead_code))]
impl StagedLayer {
    pub fn new(
        name: impl Into<String>,
        extent: u32,
        id: StagedId,
        geometry: GeometryValues,
        properties: Vec<StagedProperty>,
    ) -> MltResult<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(MltError::MissingLayerName);
        }
        let extent = Extent::new(extent)?;
        let feature_count = geometry.feature_count();
        if let Some(actual) = id.feature_count()
            && actual != feature_count
        {
            return Err(MltError::StagedFeatureCountMismatch {
                column: "id".into(),
                expected: feature_count,
                actual,
            });
        }
        for property in &properties {
            let actual = property.feature_count();
            if actual != feature_count {
                return Err(MltError::StagedFeatureCountMismatch {
                    column: property.name().to_string(),
                    expected: feature_count,
                    actual,
                });
            }
        }
        Ok(Self {
            name,
            extent,
            id,
            geometry,
            properties,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn extent(&self) -> Extent {
        self.extent
    }

    #[must_use]
    pub fn id(&self) -> &StagedId {
        &self.id
    }

    #[must_use]
    pub fn geometry(&self) -> &GeometryValues {
        &self.geometry
    }

    #[must_use]
    pub fn properties(&self) -> &[StagedProperty] {
        &self.properties
    }
}

/// Global encoder settings controlling which optimization strategies are attempted.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "enums would not model this better, not a state machine"
)]
pub struct EncoderConfig {
    /// Generate tessellation data for polygons and multi-polygons.
    tessellate: bool,
    /// Try sorting features by the Z-order (Morton) curve index of their first vertex.
    try_spatial_morton_sort: bool,
    /// Try sorting features by the Hilbert curve index of their first vertex.
    try_spatial_hilbert_sort: bool,
    /// Try sorting features by their feature ID in ascending order.
    try_id_sort: bool,
    /// Allow `FSST` string compression
    allow_fsst: bool,
    /// Allow `FastPFOR` integer compression
    allow_fpf: bool,
    /// Allow string grouping into shared dictionaries
    allow_shared_dict: bool,
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

impl EncoderConfig {
    #[must_use]
    pub fn tessellate(self) -> bool {
        self.tessellate
    }

    #[must_use]
    pub fn try_spatial_morton_sort(self) -> bool {
        self.try_spatial_morton_sort
    }

    #[must_use]
    pub fn try_spatial_hilbert_sort(self) -> bool {
        self.try_spatial_hilbert_sort
    }

    #[must_use]
    pub fn try_id_sort(self) -> bool {
        self.try_id_sort
    }

    #[must_use]
    pub fn allow_fsst(self) -> bool {
        self.allow_fsst
    }

    #[must_use]
    pub fn allow_fpf(self) -> bool {
        self.allow_fpf
    }

    #[must_use]
    pub fn allow_shared_dict(self) -> bool {
        self.allow_shared_dict
    }

    #[must_use]
    pub fn with_tessellation(mut self, enabled: bool) -> Self {
        self.tessellate = enabled;
        self
    }

    #[must_use]
    pub fn with_spatial_morton_sort(mut self, enabled: bool) -> Self {
        self.try_spatial_morton_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_spatial_hilbert_sort(mut self, enabled: bool) -> Self {
        self.try_spatial_hilbert_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_id_sort(mut self, enabled: bool) -> Self {
        self.try_id_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_fsst(mut self, enabled: bool) -> Self {
        self.allow_fsst = enabled;
        self
    }

    #[must_use]
    pub fn with_fastpfor(mut self, enabled: bool) -> Self {
        self.allow_fpf = enabled;
        self
    }

    #[must_use]
    pub fn with_shared_dict(mut self, enabled: bool) -> Self {
        self.allow_shared_dict = enabled;
        self
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
    pub const fn prop_data(name: &'a str) -> Self {
        let stream_type = StreamType::Data(DictionaryType::None);
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
