use derive_debug::Dbg;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::decoder::RawStream;
use crate::utils::formatter::{opt_vec_seq, vec_seq};
use crate::{DecodeState, Lazy};

/// Geometry column representation, parameterized by decode state.
///
/// - `Geometry<'a>` / `Geometry<'a, Lazy>` — either raw bytes or decoded, in an [`crate::LazyParsed`] enum.
/// - `Geometry<'a, Parsed>` — decoded [`GeometryValues`] directly (no enum wrapper).
pub type Geometry<'a, S = Lazy> = <S as DecodeState>::LazyOrParsed<RawGeometry<'a>, GeometryValues>;

/// Coordinate dimensionality of a geometry column: 2D (X, Y) or 3D (X, Y, Z).
///
/// Corresponds to the [`crate::wire::ColumnType::Geometry`] / `GeometryZ` distinction (and the
/// spec's `ComplexType::GEOMETRY` / `GEOMETRY_Z`). The flat vertex buffer is interleaved at
/// [`Self::size`] components per vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordDim {
    /// 2D coordinates `(x, y)`.
    #[default]
    Xy,
    /// 3D coordinates `(x, y, z)`.
    Xyz,
}

impl CoordDim {
    /// Components per vertex: 2 for [`Self::Xy`], 3 for [`Self::Xyz`].
    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::Xy => 2,
            Self::Xyz => 3,
        }
    }

    /// The matching [`wkt::types::Dimension`] for building output geometries.
    #[must_use]
    pub fn to_wkt(self) -> wkt::types::Dimension {
        match self {
            Self::Xy => wkt::types::Dimension::XY,
            Self::Xyz => wkt::types::Dimension::XYZ,
        }
    }
}

/// Raw geometry data as read directly from the tile (borrows from input bytes)
#[derive(Debug, PartialEq, Clone)]
pub struct RawGeometry<'a> {
    pub(crate) meta: RawStream<'a>,
    pub(crate) items: Vec<RawStream<'a>>,
    /// Coordinate dimensionality, set at parse time from the column type
    /// ([`crate::wire::ColumnType::Geometry`] vs `GeometryZ`).
    pub(crate) dim: CoordDim,
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
    pub(crate) triangles: Option<Vec<u32>>,
    #[dbg(formatter = "opt_vec_seq")]
    pub(crate) vertices: Option<Vec<i32>>,
    /// Coordinate dimensionality (defaults to [`CoordDim::Xy`]). The flat `vertices` buffer is
    /// interleaved at `dim.size()` components per vertex; offsets count whole vertices regardless
    /// of stride.
    pub(crate) dim: CoordDim,
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
