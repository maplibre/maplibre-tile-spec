use borrowme::borrowme;
use enum_dispatch::enum_dispatch;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::analyse::{Analyze, StatType};
use crate::v01::Stream;

/// Geometry column representation, either encoded or decoded
#[borrowme]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
#[enum_dispatch(Analyze)]
pub enum Geometry<'a> {
    Encoded(EncodedGeometry<'a>),
    Decoded(DecodedGeometry),
}

/// Unparsed geometry data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedGeometry<'a> {
    pub meta: Stream<'a>,
    pub items: Vec<Stream<'a>>,
}

/// Decoded geometry data
#[derive(Clone, Default, PartialEq)]
pub struct DecodedGeometry {
    // pub vector_type: VectorType,
    // pub vertex_buffer_type: VertexBufferType,
    pub vector_types: Vec<GeometryType>,
    pub geometry_offsets: Option<Vec<u32>>,
    pub part_offsets: Option<Vec<u32>>,
    pub ring_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum VectorType {
//     Flat,
//     Const,
//     Sequence,
//     // Dictionary,
//     // FsstDictionary,
// }

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
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}

impl GeometryType {
    #[must_use]
    pub fn is_polygon(self) -> bool {
        matches!(self, GeometryType::Polygon | GeometryType::MultiPolygon)
    }
    #[must_use]
    pub fn is_linestring(self) -> bool {
        matches!(
            self,
            GeometryType::LineString | GeometryType::MultiLineString
        )
    }
    #[must_use]
    pub fn is_multi(self) -> bool {
        matches!(
            self,
            GeometryType::MultiPoint | GeometryType::MultiLineString | GeometryType::MultiPolygon
        )
    }
}

/// Describes how the vertex buffer should be encoded.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum VertexBufferType {
    /// Standard 2D `(x, y)` pairs encoded with componentwise delta.
    #[default]
    Vec2,
    /// Morton (Z-order) dictionary encoding:
    /// Unique vertices are sorted by their Morton code and stored once.
    /// Each vertex position in the stream is replaced by its index into that dictionary.
    Morton,
}
