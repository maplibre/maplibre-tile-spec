use derive_debug::Dbg;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::decoder::RawStream;
#[cfg(feature = "unstable-v2")]
use crate::tile::ZStep;
use crate::utils::formatter::{opt_vec_seq, vec_seq};
use crate::{DecodeState, Lazy};

/// Geometry column representation, parameterized by decode state.
///
/// - `Geometry<'a>` / `Geometry<'a, Lazy>` - either raw bytes or decoded, in an [`crate::LazyParsed`] enum.
/// - `Geometry<'a, Parsed>` - decoded [`GeometryValues`] directly (no enum wrapper).
pub type Geometry<'a, S = Lazy> = <S as DecodeState>::LazyOrParsed<RawGeometry<'a>, GeometryValues>;

/// Raw geometry data as read directly from the tile (borrows from input bytes)
#[derive(Debug, PartialEq, Clone)]
pub struct RawGeometry<'a> {
    pub(crate) types: GeoTypes<'a>,
    pub(crate) index_base: IndexBase,
    pub(crate) items: Vec<RawStream<'a>>,
}

/// Which vertex a section's triangle index `0` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexBase {
    /// The first vertex of the index's own feature, as v1 stores it.
    Feature,
    /// The first vertex of the layer, as v2 stores it.
    #[cfg(feature = "unstable-v2")]
    Layer,
}

/// Where a geometry section's per-feature [`GeometryType`]s come from.
///
/// A v1 section always spells them out. A v2 section may instead name one type in
/// its [layer header byte](crate::decoder::LayerHeader02), which is all a layer of
/// a single geometry type has to say.
#[derive(Debug, PartialEq, Clone)]
pub enum GeoTypes<'a> {
    /// The leading stream of the section, one value per feature.
    Stream(RawStream<'a>),
    /// No stream: every one of `feature_count` features has this type.
    #[cfg(feature = "unstable-v2")]
    Uniform {
        geometry_type: GeometryType,
        feature_count: u32,
    },
}

/// Parsed (decoded) geometry data
#[derive(Clone, Dbg, Default, PartialEq, Eq)]
pub struct GeometryValues {
    #[dbg(formatter = "vec_seq")]
    pub(crate) vector_types: Vec<GeometryType>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) geometry_offsets: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) part_offsets: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) ring_offsets: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) index_buffer: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) triangle_offsets: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) vertices: Option<Vec<i32>>,
    /// The grid of each vertex's third word, or [`None`] when the vertices are `(x, y)` pairs.
    #[cfg(feature = "unstable-v2")]
    pub(crate) z_step: Option<ZStep>,
}

/// Types of geometries supported in MLT
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Eq,
    Hash,
    Ord,
    TryFromPrimitive,
    strum::Display,
    strum::IntoStaticStr,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum GeometryType {
    /*
        ATTENTION: Do not modify the order of this enum - it is being used in geometry decoding
    */
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}
