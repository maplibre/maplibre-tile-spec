mod decode;
mod encode;

use std::fmt::Debug;
use std::io::Write;
use std::ops::Range;

use borrowme::borrowme;
use derive_builder::Builder;
use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::MltError::{
    GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds, IntegerOverflow,
    NoGeometryOffsets, NoPartOffsets, NoRingOffsets, NotImplemented, UnexpectedOffsetCombination,
};
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::encode::impl_encodable;
use crate::geojson::{Coord32, Geom32 as GeoGeom};
use crate::utils::{BinarySerializer as _, OptSeq, SetOptionOnce as _};
use crate::v01::column::ColumnType;
use crate::v01::geometry::decode::{
    decode_geometry_types, decode_level1_length_stream,
    decode_level1_without_ring_buffer_length_stream, decode_level2_length_stream,
    decode_root_length_stream,
};
use crate::v01::{
    DictionaryType, LengthType, LogicalCodec, LogicalEncoding, OffsetType, OwnedStream,
    PhysicalCodec, PhysicalEncoding, PhysicalStreamType, Stream, StreamMeta,
};
use crate::{FromDecoded, MltError};

/// Geometry column representation, either encoded or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Geometry<'a> {
    Encoded(EncodedGeometry<'a>),
    Decoded(DecodedGeometry),
}

impl Analyze for Geometry<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(g) => g.collect_statistic(stat),
            Self::Decoded(g) => g.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Encoded(g) => g.for_each_stream(cb),
            Self::Decoded(g) => g.for_each_stream(cb),
        }
    }
}

impl OwnedGeometry {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(_) => OwnedEncodedGeometry::write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

/// Unparsed geometry data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedGeometry<'a> {
    pub meta: Stream<'a>,
    pub items: Vec<Stream<'a>>,
}

impl Default for OwnedEncodedGeometry {
    fn default() -> Self {
        Self {
            meta: OwnedStream::empty_without_codec(),
            items: Vec::new(),
        }
    }
}

impl Analyze for EncodedGeometry<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.meta.for_each_stream(cb);
        self.items.for_each_stream(cb);
    }
}

impl<'a> EncodedGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams)
    pub fn parse(input: &'a [u8]) -> crate::MltRefResult<'a, Self> {
        use crate::utils::parse_varint;

        let (input, stream_count) = parse_varint::<u64>(input)?;
        let stream_count = usize::try_from(stream_count)?;
        if stream_count == 0 {
            return Ok((
                input,
                Self {
                    meta: Stream::new(
                        StreamMeta {
                            physical_type: PhysicalStreamType::Data(DictionaryType::None),
                            num_values: 0,
                            logical_codec: LogicalCodec::None,
                            physical_codec: PhysicalCodec::None,
                        },
                        crate::v01::EncodedData::new(&[]),
                    ),
                    items: Vec::new(),
                },
            ));
        }

        let (input, meta) = Stream::parse(input)?;
        let (input, items) = Stream::parse_multiple(input, stream_count - 1)?;

        Ok((input, Self { meta, items }))
    }
}

impl OwnedEncodedGeometry {
    pub(crate) fn write_columns_meta_to<W: Write>(writer: &mut W) -> Result<(), MltError> {
        ColumnType::Geometry.write_to(writer)?;
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let items_len = u64::try_from(self.items.len())?;
        let items_len = items_len.checked_add(1).ok_or(IntegerOverflow)?;
        writer.write_varint(items_len)?;
        writer.write_stream(&self.meta)?;
        for item in &self.items {
            writer.write_stream(item)?;
        }
        Ok(())
    }
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
    pub vertex_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

impl Analyze for DecodedGeometry {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedDataSize => {
                self.vector_types.collect_statistic(stat)
                    + self.geometry_offsets.collect_statistic(stat)
                    + self.part_offsets.collect_statistic(stat)
                    + self.ring_offsets.collect_statistic(stat)
                    + self.vertex_offsets.collect_statistic(stat)
                    + self.index_buffer.collect_statistic(stat)
                    + self.triangles.collect_statistic(stat)
                    + self.vertices.collect_statistic(stat)
            }
            StatType::DecodedMetaSize => 0,
            StatType::FeatureCount => self.vector_types.len(),
        }
    }
}

impl DecodedGeometry {
    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> Result<GeoGeom, MltError> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();
        let vo = self.vertex_offsets.as_deref();

        let off = |s: &[u32], idx: usize, field: &'static str| -> Result<usize, MltError> {
            match s.get(idx) {
                Some(&v) => Ok(v as usize),
                None => Err(GeometryOutOfBounds {
                    index,
                    field,
                    idx,
                    len: s.len(),
                }),
            }
        };

        let geom_off = |s: &[u32], idx: usize| off(s, idx, "geometry_offsets");
        let part_off = |s: &[u32], idx: usize| off(s, idx, "part_offsets");
        let ring_off = |s: &[u32], idx: usize| off(s, idx, "ring_offsets");

        let geom_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(geom_off(s, i)?..geom_off(s, i + 1)?)
        };
        let part_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(part_off(s, i)?..part_off(s, i + 1)?)
        };
        let ring_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(ring_off(s, i)?..ring_off(s, i + 1)?)
        };

        let v = |idx: usize| -> Result<Coord32, MltError> {
            let vertex = match vo {
                Some(vo) => off(vo, idx, "vertex_offsets")?,
                None => idx,
            };
            let s = match verts.get(vertex * 2..(vertex * 2) + 2) {
                Some(v) => v,
                None => Err(GeometryVertexOutOfBounds {
                    index,
                    vertex,
                    count: verts.len() / 2,
                })?,
            };
            Ok(Coord { x: s[0], y: s[1] })
        };
        let line = |r: Range<usize>| -> Result<LineString<i32>, MltError> { r.map(&v).collect() };
        let closed_ring = |r: Range<usize>| -> Result<LineString<i32>, MltError> {
            let start = r.start;
            let mut coords: Vec<Coord32> = r.map(&v).collect::<Result<_, _>>()?;
            coords.push(v(start)?);
            Ok(LineString(coords))
        };
        let rings_in =
            |part_range: Range<usize>, rings: &[u32]| -> Result<Polygon<i32>, MltError> {
                let ring_vecs: Vec<LineString<i32>> = part_range
                    .map(|r| closed_ring(ring_off_pair(rings, r)?))
                    .collect::<Result<_, _>>()?;
                let mut iter = ring_vecs.into_iter();
                let exterior = iter.next().unwrap_or_else(|| LineString(vec![]));
                let interiors: Vec<LineString<i32>> = iter.collect();
                Ok(Polygon::new(exterior, interiors))
            };

        let geom_type = *self
            .vector_types
            .get(index)
            .ok_or(GeometryIndexOutOfBounds(index))?;

        match geom_type {
            GeometryType::Point => {
                let pt = match (geoms, parts, rings) {
                    (Some(g), Some(p), Some(r)) => {
                        v(ring_off(r, part_off(p, geom_off(g, index)?)?)?)?
                    }
                    (Some(g), Some(p), None) => v(part_off(p, geom_off(g, index)?)?)?,
                    (None, Some(p), Some(r)) => v(ring_off(r, part_off(p, index)?)?)?,
                    (None, Some(p), None) => v(part_off(p, index)?)?,
                    (None, None, None) => v(index)?,
                    _ => {
                        return Err(UnexpectedOffsetCombination(index, geom_type));
                    }
                };
                Ok(GeoGeom::Point(Point(pt)))
            }
            GeometryType::LineString => {
                let r = match (parts, rings) {
                    (Some(p), Some(r)) => ring_off_pair(r, part_off(p, index)?)?,
                    (Some(p), None) => part_off_pair(p, index)?,
                    _ => return Err(NoPartOffsets(index, geom_type)),
                };
                line(r).map(GeoGeom::LineString)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let i = geoms
                    .map(|g| geom_off(g, index))
                    .transpose()?
                    .unwrap_or(index);
                rings_in(part_off_pair(parts, i)?, rings).map(GeoGeom::Polygon)
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                geom_off_pair(geoms, index)?
                    .map(&v)
                    .collect::<Result<Vec<Coord32>, _>>()
                    .map(|cs| GeoGeom::MultiPoint(MultiPoint(cs.into_iter().map(Point).collect())))
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                geom_off_pair(geoms, index)?
                    .map(|p| line(part_off_pair(parts, p)?))
                    .collect::<Result<Vec<LineString<i32>>, _>>()
                    .map(|ls| GeoGeom::MultiLineString(MultiLineString(ls)))
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                geom_off_pair(geoms, index)?
                    .map(|p| rings_in(part_off_pair(parts, p)?, rings))
                    .collect::<Result<Vec<Polygon<i32>>, _>>()
                    .map(|ps| GeoGeom::MultiPolygon(MultiPolygon(ps)))
            }
        }
    }
}

/// Types of geometries supported in MLT
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord, TryFromPrimitive, strum::Display,
)]
#[repr(u8)]
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
    pub fn is_linestring(self) -> bool {
        matches!(
            self,
            GeometryType::LineString | GeometryType::MultiLineString
        )
    }
}

impl Analyze for GeometryType {
    fn collect_statistic(&self, _stat: StatType) -> usize {
        size_of::<Self>()
    }
}

// /// Vertex buffer type used for geometry columns
// #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
// pub enum VertexBufferType {
//     Morton,
//     Vec2,
//     Vec3,
// }

// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum VectorType {
//     Flat,
//     Const,
//     Sequence,
//     // Dictionary,
//     // FsstDictionary,
// }

impl_decodable!(Geometry<'a>, EncodedGeometry<'a>, DecodedGeometry);
impl_encodable!(OwnedGeometry, DecodedGeometry, OwnedEncodedGeometry);

/// How to encode Geometry
#[derive(Debug, Clone, Copy, Builder)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct GeometryEncodingStrategy {
    /// Logical encoding for the geometry types (meta) stream.
    meta_logical: LogicalEncoding,
    /// Physical codec for the geometry types (meta) stream.
    meta_physical: PhysicalEncoding,

    /// Logical encoding for the geometry length stream
    num_geometries_logical: LogicalEncoding,
    /// Physical codec for the geometry length stream
    num_geometries_physical: PhysicalEncoding,

    /// how to name this ???
    rings_logical: LogicalEncoding,
    rings2_logical: LogicalEncoding,
    no_rings_logical: LogicalEncoding,
    rings_physical: PhysicalEncoding,
    rings2_physical: PhysicalEncoding,
    no_rings_physical: PhysicalEncoding,

    /// how to name this ???
    parts_logical: LogicalEncoding,
    parts_ring_logical: LogicalEncoding,
    parts_physical: PhysicalEncoding,
    parts_ring_physical: PhysicalEncoding,

    only_parts_logical: LogicalEncoding,
    only_parts_physical: PhysicalEncoding,

    /// Logical codec for triangles stream for pre-tessellated polygons
    triangles_logical: LogicalEncoding,
    triangles_indexes_logical: LogicalEncoding,
    /// Physical codec for triangles stream for pre-tessellated polygons
    triangles_physical: PhysicalEncoding,
    triangles_indexes_physical: PhysicalEncoding,

    /// Physical codec for the vertex data stream.
    ///
    /// The logical codec is always [`LogicalCodec::ComponentwiseDelta`]
    vertex_physical: PhysicalEncoding,
    vertex_offsets_logical: LogicalEncoding,
    vertex_offsets_physical: PhysicalEncoding,
}

impl FromDecoded<'_> for OwnedEncodedGeometry {
    type Input = DecodedGeometry;
    type EncodingStrategy = GeometryEncodingStrategy;

    fn from_decoded(
        decoded: &Self::Input,
        config: Self::EncodingStrategy,
    ) -> Result<Self, MltError> {
        encode::encode_geometry(decoded, config)
    }
}

impl<'a> From<EncodedGeometry<'a>> for Geometry<'a> {
    fn from(value: EncodedGeometry<'a>) -> Self {
        Self::Encoded(value)
    }
}

impl<'a> Geometry<'a> {
    #[must_use]
    pub fn new_encoded(meta: Stream<'a>, items: Vec<Stream<'a>>) -> Self {
        Self::Encoded(EncodedGeometry { meta, items })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedGeometry, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedGeometry::from_encoded(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl Debug for DecodedGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedGeometry")
            .field("vector_types", &OptSeq(Some(&self.vector_types)))
            .field(
                "geometry_offsets",
                &OptSeq(self.geometry_offsets.as_deref()),
            )
            .field("part_offsets", &OptSeq(self.part_offsets.as_deref()))
            .field("ring_offsets", &OptSeq(self.ring_offsets.as_deref()))
            .field("vertex_offsets", &OptSeq(self.vertex_offsets.as_deref()))
            .field("index_buffer", &OptSeq(self.index_buffer.as_deref()))
            .field("triangles", &OptSeq(self.triangles.as_deref()))
            .field("vertices", &OptSeq(self.vertices.as_deref()))
            .finish()
    }
}

impl<'a> FromEncoded<'a> for DecodedGeometry {
    type Input = EncodedGeometry<'a>;

    fn from_encoded(
        EncodedGeometry { meta, items }: EncodedGeometry<'a>,
    ) -> Result<Self, MltError> {
        let vector_types = decode_geometry_types(meta)?;
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        for stream in items {
            match stream.meta.physical_type {
                PhysicalStreamType::Present => {}
                PhysicalStreamType::Data(v) => match v {
                    DictionaryType::Vertex => {
                        let v = stream.decode_bits_u32()?.decode_i32()?;
                        vertices.set_once(v)?;
                    }
                    _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
                },
                PhysicalStreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
                    };
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
                PhysicalStreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
                    };
                    // LogicalStream2<U> -> LogicalStream -> trait LogicalStreamCodec<T>
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
            }
        }

        if index_buffer.is_some() && part_offsets.is_none() {
            // Case when the indices of a Polygon outline are not encoded in the data so no
            // topology data are present in the tile
            //
            // return FlatGpuVector::new(vector_types, triangles, index_buffer, vertices);
            return Err(NotImplemented(
                "index_buffer.is_some() && part_offsets.is_none() case",
            ));
        }

        // Use decode_root_length_stream if geometry_offsets is present
        if let Some(offsets) = geometry_offsets.take() {
            geometry_offsets = Some(decode_root_length_stream(
                &vector_types,
                &offsets,
                GeometryType::Polygon,
            ));
            if let Some(part_offsets_copy) = part_offsets.take() {
                if let Some(ring_offsets_copy) = ring_offsets.take() {
                    part_offsets = Some(decode_level1_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                        false, // isLineStringPresent
                    ));
                    ring_offsets = Some(decode_level2_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        part_offsets.as_ref().unwrap(),
                        &ring_offsets_copy,
                    ));
                } else {
                    part_offsets = Some(decode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                    ));
                }
            }
        } else if let Some(offsets) = part_offsets.take() {
            if let Some(ring_offsets_copy) = ring_offsets.take() {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::LineString,
                ));
                ring_offsets = Some(decode_level1_length_stream(
                    &vector_types,
                    part_offsets.as_ref().unwrap(),
                    &ring_offsets_copy,
                    true, // isLineStringPresent
                ));
            } else {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::Point,
                ));
            }
        }

        // Case when the indices of a Polygon outline are encoded in the tile
        // This is handled by including index_buffer in the DecodedGeometry

        Ok(DecodedGeometry {
            // vertex_buffer_type: VertexBufferType::Vec2, // Morton not supported yet
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertex_offsets,
            index_buffer,
            triangles,
            vertices,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::Encodable as _;

    /// Helper function to encode, serialize, parse, and decode for roundtrip testing
    fn roundtrip(decoded: &DecodedGeometry, config: GeometryEncodingStrategy) -> DecodedGeometry {
        let encoded =
            OwnedEncodedGeometry::from_decoded(decoded, config).expect("Failed to encode");

        // Serialize to bytes (write_to includes the stream count varint)
        let mut buffer = Vec::new();
        encoded.write_to(&mut buffer).expect("Failed to serialize");

        // Now parse (parse expects varint stream count + streams)
        let (remaining, parsed) = EncodedGeometry::parse(&buffer).expect("Failed to parse");
        assert!(remaining.is_empty(), "Remaining bytes after parse");

        DecodedGeometry::from_encoded(parsed).expect("Failed to decode")
    }

    fn geometry_roundtrip(
        decoded: &DecodedGeometry,
        strategy: GeometryEncodingStrategy,
    ) -> DecodedGeometry {
        let encoded =
            OwnedEncodedGeometry::from_decoded(decoded, strategy).expect("encoding failed");
        let borrowed = borrowme::borrow(&encoded);
        DecodedGeometry::from_encoded(borrowed).expect("decoding failed")
    }

    proptest! {
        #[test]
        fn test_polygon_with_hole_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Polygon with exterior ring and one interior ring (hole)
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Polygon],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 2]), // One polygon with 2 rings
                ring_offsets: Some(vec![0, 4, 8]), // Exterior: 4 vertices, Interior: 4 vertices
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![
                    0, 0, 100, 0, 100, 100, 0, 100, // Exterior ring
                    25, 25, 75, 25, 75, 75, 25, 75, // Interior ring (hole)
                ]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_empty_geometry_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry::default();
            let output = roundtrip(&input, strategy);
            assert_eq!(output.vector_types, input.vector_types);
        }

        #[test]
        fn test_single_point_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Point],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![100, 200]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_multiple_points_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![
                    GeometryType::Point,
                    GeometryType::Point,
                    GeometryType::Point,
                ],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 100, 200, 50, 150]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_linestring_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::LineString],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 3]),
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 100, 0, 100, 100]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_polygon_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // A simple square polygon (exterior ring only)
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Polygon],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 1]),
                ring_offsets: Some(vec![0, 4]),
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 100, 0, 100, 100, 0, 100]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_multipoint_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::MultiPoint],
                geometry_offsets: Some(vec![0, 3]),
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 50, 50, 100, 100]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_multilinestring_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::MultiLineString],
                geometry_offsets: Some(vec![0, 2]),
                part_offsets: Some(vec![0, 2, 3]),
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 100, 100, 200, 200]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_multipolygon_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // MultiPolygon with 2 simple polygons
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::MultiPolygon],
                geometry_offsets: Some(vec![0, 2]),
                part_offsets: Some(vec![0, 1, 2]),
                ring_offsets: Some(vec![0, 4, 8]),
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![
                    0, 0, 10, 0, 10, 10, 0, 10, // First polygon
                    20, 20, 30, 20, 30, 30, 20, 30, // Second polygon
                ]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_mixed_geometry_types_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Mix of Point and LineString
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Point, GeometryType::LineString],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 1, 4]),
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 10, 10, 20, 20, 30, 30]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_encodable_trait_api(strategy in any::<GeometryEncodingStrategy>()) {
            let decoded = DecodedGeometry {
                vector_types: vec![GeometryType::Point],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![100, 200]),
            };

            let mut geom_enum = OwnedGeometry::Decoded(decoded.clone());
            geom_enum.encode_with(strategy).expect("Failed to encode");

            assert!(!geom_enum.is_decoded(), "Should be Encoded after encoding");
        }

        #[test]
        fn test_negative_coordinates(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Point, GeometryType::Point],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![-100, -200, 100, 200]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_large_coordinates(strategy in any::<GeometryEncodingStrategy>()) {
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Point],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![i32::MAX, i32::MIN]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_tessellated_polygon_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Pre-tessellated polygon with triangles and index buffer
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Polygon],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 1]),
                ring_offsets: Some(vec![0, 4]),
                vertex_offsets: None,
                index_buffer: Some(vec![0, 1, 2, 0, 2, 3]), // Two triangles
                triangles: Some(vec![2]),                   // One polygon with 2 triangles
                vertices: Some(vec![0, 0, 100, 0, 100, 100, 0, 100]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_vertex_offsets_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Dictionary-encoded vertices with vertex offsets
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::LineString, GeometryType::LineString],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 3, 6]),
                ring_offsets: None,
                vertex_offsets: Some(vec![0, 1, 2, 2, 1, 0]), // Indices into vertices
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![0, 0, 100, 100, 200, 200]), // Only 3 unique vertices
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_mixed_point_multipoint_polygon_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Mix of Point, MultiPoint, and Polygon to exercise level2 Point/MultiPoint branch
            let input = DecodedGeometry {
                vector_types: vec![
                    GeometryType::Point,
                    GeometryType::MultiPoint,
                    GeometryType::Polygon,
                ],
                geometry_offsets: Some(vec![0, 1, 4, 5]), // Point:1, MultiPoint:3 points, Polygon:1
                part_offsets: Some(vec![0, 1, 2, 3, 4, 5]), // Each geometry has one part
                ring_offsets: Some(vec![0, 1, 2, 3, 4, 8]), // Point/MultiPoint have 1 vertex each, Polygon has 4
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![
                    0, 0, // Point
                    10, 10, 20, 20, 30, 30, // MultiPoint (3 points)
                    100, 100, 200, 100, 200, 200, 100, 200, // Polygon
                ]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_geometry_no_vertices(strategy in any::<GeometryEncodingStrategy>()) {
            // Geometry with types but no vertices (edge case)
            let input = DecodedGeometry {
                vector_types: vec![GeometryType::Point],
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: None, // No vertices
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_multiple_linestrings_roundtrip(strategy in any::<GeometryEncodingStrategy>()) {
            // Multiple separate LineStrings (not a MultiLineString)
            let input = DecodedGeometry {
                vector_types: vec![
                    GeometryType::LineString,
                    GeometryType::LineString,
                    GeometryType::LineString,
                ],
                geometry_offsets: None,
                part_offsets: Some(vec![0, 3, 5, 8]),
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
                vertices: Some(vec![
                    0, 0, 10, 10, 20, 20, // Line 1: 3 vertices
                    30, 30, 40, 40, // Line 2: 2 vertices
                    50, 50, 60, 60, 70, 70, // Line 3: 3 vertices
                ]),
            };
            let output = roundtrip(&input, strategy);
            prop_assert_eq!(output, input);
        }

        #[test]
        fn test_point_roundtrip(
            coords in prop::collection::vec([any::<i32>(), any::<i32>()], 0..100),
            strategy in any::<GeometryEncodingStrategy>(),
        ) {
            let vector_types = vec![GeometryType::Point; coords.len()];
            let vertices: Vec<i32> = coords.into_iter().flatten().collect();

            let decoded = DecodedGeometry {
                vector_types,
                vertices: if vertices.is_empty() { None } else { Some(vertices) },
                geometry_offsets: None,
                part_offsets: None,
                ring_offsets: None,
                vertex_offsets: None,
                index_buffer: None,
                triangles: None,
            };
            prop_assert_eq!(geometry_roundtrip(&decoded, strategy), decoded);
        }
    }
}
