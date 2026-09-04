use std::borrow::Cow;
use std::collections::HashSet;

use derive_debug::Dbg;

#[cfg(feature = "unstable-v2")]
use crate::decoder::RleLayout;
use crate::decoder::{DictionaryType, FastPForKind, GeometryValues, PhysicalEncoding, StreamType};
use crate::encoder::geometry::VertexBufferType;
use crate::encoder::{IntEncoder, StagedId, StagedProperty};
use crate::tile::Extent;
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
        // Column names must be unique within a layer. A shared dictionary's `name()` is
        // only its prefix (which may repeat); its real columns are `{prefix}{suffix}`.
        // Scoped so `seen` releases its borrow of `properties` before the move below.
        {
            let mut seen: HashSet<Cow<str>> = HashSet::new();
            for property in &properties {
                let actual = property.feature_count();
                if actual != feature_count {
                    return Err(MltError::StagedFeatureCountMismatch {
                        column: property.name().to_string(),
                        expected: feature_count,
                        actual,
                    });
                }
                if let StagedProperty::SharedDict(sd) = property {
                    for item in &sd.items {
                        if !seen.insert(Cow::Owned(format!("{}{}", sd.prefix, item.suffix))) {
                            return Err(MltError::DuplicatePropertyName(format!(
                                "{}{}",
                                sd.prefix, item.suffix
                            )));
                        }
                    }
                } else if !seen.insert(Cow::Borrowed(property.name())) {
                    return Err(MltError::DuplicatePropertyName(property.name().to_string()));
                }
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

/// Which wire format layers are encoded to.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum WireVersion {
    /// Tag `0x01` - the stable v1 format.
    #[default]
    V01,
    /// Tag `0x02` - the experimental v2 format (see `docs/migrating-to-v2.md`).
    ///
    /// Currently limited to ID, scalar, string, shared-dictionary and geometry
    /// columns, the last without a tessellation that has only part of its
    /// outline topology.
    V02,
}

#[cfg(feature = "unstable-v2")]
impl WireVersion {
    /// The layer tag byte identifying this format on the wire.
    #[must_use]
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::V01 => 1,
            Self::V02 => 2,
        }
    }

    /// The `FastPFor` block size and word order used by this format.
    #[must_use]
    pub(crate) fn fastpfor_kind(self) -> FastPForKind {
        match self {
            Self::V01 => FastPForKind::Block256Be,
            Self::V02 => FastPForKind::Block128Le,
        }
    }

    /// The RLE stream data layout used by this format.
    #[must_use]
    pub(crate) fn rle_layout(self) -> RleLayout {
        match self {
            Self::V01 => RleLayout::Split,
            Self::V02 => RleLayout::Interleaved,
        }
    }
}

/// Global encoder settings controlling which optimization strategies are attempted.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "enums would not model this better, not a state machine"
)]
pub struct EncoderConfig {
    /// The wire format to encode layers to.
    #[cfg(feature = "unstable-v2")]
    wire_version: WireVersion,
    /// Generate tessellation data for polygons and multi-polygons.
    tessellate: bool,
    /// Try sorting features by the Z-order (Morton) curve index of their first vertex.
    attempt_spatial_morton_sort: bool,
    /// Try sorting features by the Hilbert curve index of their first vertex.
    attempt_spatial_hilbert_sort: bool,
    /// Try sorting features by their feature ID in ascending order.
    attempt_id_sort: bool,
    /// Allow `FSST` string compression
    allow_fsst: bool,
    /// Allow `FastPFOR` integer compression
    allow_fastpfor: bool,
    /// Allow string grouping into shared dictionaries
    allow_shared_dict: bool,
    /// Allow the v2-only float dictionary encoding
    #[cfg(feature = "unstable-v2")]
    allow_float_dict: bool,
    /// Allow the v2-only ALP float encoding
    #[cfg(feature = "unstable-v2")]
    allow_float_alp: bool,
    /// Allow the v2-only bit-packed physical encoding for dictionary code streams
    #[cfg(feature = "unstable-v2")]
    allow_packed_dict_codes: bool,
}
impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "unstable-v2")]
            wire_version: WireVersion::V01,
            tessellate: false,
            attempt_spatial_morton_sort: true,
            attempt_spatial_hilbert_sort: true,
            attempt_id_sort: true,
            allow_fsst: true,
            allow_fastpfor: true,
            allow_shared_dict: true,
            // Off by default while the encoding is still being measured.
            #[cfg(feature = "unstable-v2")]
            allow_float_dict: false,
            #[cfg(feature = "unstable-v2")]
            allow_float_alp: false,
            #[cfg(feature = "unstable-v2")]
            allow_packed_dict_codes: false,
        }
    }
}

impl EncoderConfig {
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn wire_version(self) -> WireVersion {
        self.wire_version
    }

    #[must_use]
    pub fn tessellate(self) -> bool {
        self.tessellate
    }

    #[must_use]
    pub fn attempt_spatial_morton_sort(self) -> bool {
        self.attempt_spatial_morton_sort
    }

    #[must_use]
    pub fn attempt_spatial_hilbert_sort(self) -> bool {
        self.attempt_spatial_hilbert_sort
    }

    #[must_use]
    pub fn attempt_id_sort(self) -> bool {
        self.attempt_id_sort
    }

    #[must_use]
    pub fn allow_fsst(self) -> bool {
        self.allow_fsst
    }

    #[must_use]
    pub fn allow_fastpfor(self) -> bool {
        self.allow_fastpfor
    }

    /// The `FastPFor` encoding to race, or `None` when `FastPFor` is switched off.
    #[must_use]
    pub(crate) fn fastpfor(self) -> Option<PhysicalEncoding> {
        #[cfg(feature = "unstable-v2")]
        let kind = self.wire_version.fastpfor_kind();
        #[cfg(not(feature = "unstable-v2"))]
        let kind = FastPForKind::Block256Be;
        self.allow_fastpfor
            .then_some(PhysicalEncoding::FastPFor(kind))
    }

    #[must_use]
    pub fn allow_shared_dict(self) -> bool {
        self.allow_shared_dict
    }

    /// Whether float columns may use a dictionary, which only v2 can express.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn allow_float_dict(self) -> bool {
        self.allow_float_dict && self.wire_version != WireVersion::V01
    }

    /// Whether float columns may use ALP, which only v2 can express.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn allow_float_alp(self) -> bool {
        self.allow_float_alp && self.wire_version != WireVersion::V01
    }

    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn with_wire_version(mut self, version: WireVersion) -> Self {
        self.wire_version = version;
        self
    }

    #[must_use]
    pub fn with_tessellation(mut self, enabled: bool) -> Self {
        self.tessellate = enabled;
        self
    }

    #[must_use]
    pub fn with_spatial_morton_sort(mut self, enabled: bool) -> Self {
        self.attempt_spatial_morton_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_spatial_hilbert_sort(mut self, enabled: bool) -> Self {
        self.attempt_spatial_hilbert_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_id_sort(mut self, enabled: bool) -> Self {
        self.attempt_id_sort = enabled;
        self
    }

    #[must_use]
    pub fn with_fsst(mut self, enabled: bool) -> Self {
        self.allow_fsst = enabled;
        self
    }

    #[must_use]
    pub fn with_fastpfor(mut self, enabled: bool) -> Self {
        self.allow_fastpfor = enabled;
        self
    }

    #[must_use]
    pub fn with_shared_dict(mut self, enabled: bool) -> Self {
        self.allow_shared_dict = enabled;
        self
    }

    /// Allow float columns to store one code per value into a dictionary of the distinct ones.
    /// Off by default, and only v2 can express it.
    /// A column takes it only when it comes out strictly smaller in stored bytes.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn with_float_dict(mut self, enabled: bool) -> Self {
        self.allow_float_dict = enabled;
        self
    }

    /// Allow float columns to store each value as a decimal-scaled integer.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn with_float_alp(mut self, enabled: bool) -> Self {
        self.allow_float_alp = enabled;
        self
    }

    /// Whether dictionary code streams may be stored bit-packed.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn allow_packed_dict_codes(self) -> bool {
        self.allow_packed_dict_codes && self.wire_version != WireVersion::V01
    }

    /// Allow dictionary code streams to store every code in the same
    /// `ceil(log2(dict_len))` bits instead of a varint each.
    #[cfg(feature = "unstable-v2")]
    #[must_use]
    pub fn with_packed_dict_codes(mut self, enabled: bool) -> Self {
        self.allow_packed_dict_codes = enabled;
        self
    }
}

/// How to encode a string column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrEncoding {
    Plain,
    Dict,
    Fsst,
    FsstDict,
}

/// How to encode a float column, pinned rather than costed against the alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatEncoding {
    None,
    #[cfg(feature = "unstable-v2")]
    Dict,
    #[cfg(feature = "unstable-v2")]
    Alp,
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
/// All encoding choices are caller-specified via callbacks so one struct can cover any combination without per-stream boilerplate.
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
    /// Return the logical encoding for a float property column.
    #[dbg(skip)]
    pub get_float_encoding: Box<dyn Fn(&str) -> FloatEncoding>,
}
