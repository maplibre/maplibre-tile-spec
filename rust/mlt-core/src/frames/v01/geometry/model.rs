use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::v01::{EncodedStream, RawStream};
use crate::{DecodeState, Lazy};

/// Geometry column representation, parameterized by decode state.
///
/// - `Geometry<'a>` / `Geometry<'a, Lazy>` — either raw bytes or decoded, in an [`crate::LazyParsed`] enum.
/// - `Geometry<'a, Parsed>` — decoded [`GeometryValues`] directly (no enum wrapper).
pub type Geometry<'a, S = Lazy> = <S as DecodeState>::LazyOrParsed<RawGeometry<'a>, GeometryValues>;

/// Raw geometry data as read directly from the tile (borrows from input bytes)
#[derive(Debug, PartialEq, Clone)]
pub struct RawGeometry<'a> {
    pub meta: RawStream<'a>,
    pub items: Vec<RawStream<'a>>,
}

/// Parsed (decoded) geometry data
#[derive(Clone, Default, PartialEq, Eq)]
pub struct GeometryValues {
    pub vector_types: Vec<GeometryType>,
    pub geometry_offsets: Option<Vec<u32>>,
    pub part_offsets: Option<Vec<u32>>,
    pub ring_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

/// Wire-ready encoded geometry data (owns its byte buffers)
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedGeometry {
    pub meta: EncodedStream,
    pub items: Vec<EncodedStream>,
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
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
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
