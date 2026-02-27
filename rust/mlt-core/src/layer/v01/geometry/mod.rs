mod decode;
mod encode;

use std::fmt::Debug;
use std::io::Write;
use std::ops::Range;

use borrowme::borrowme;
use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

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
pub use crate::v01::geometry::encode::GeometryEncoder;
use crate::v01::{
    DictionaryType, LengthType, LogicalEncoding, OffsetType, OwnedStream, PhysicalEncoding, Stream,
    StreamMeta, StreamType,
};
use crate::{FromDecoded, MltError};

/// Geometry column representation, either encoded or decoded
#[borrowme]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
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
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedGeometry<'a> {
    pub meta: Stream<'a>,
    pub items: Vec<Stream<'a>>,
}

impl Default for OwnedEncodedGeometry {
    fn default() -> Self {
        Self {
            meta: OwnedStream::empty_without_encoding(),
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
                        StreamMeta::new(
                            StreamType::Data(DictionaryType::None),
                            LogicalEncoding::None,
                            PhysicalEncoding::None,
                            0,
                        ),
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
                // When geometry_offsets exist (mixed LineString/MultiLineString), use them to
                // find the correct part index. When rings exist (polygon geometry present),
                // the vertex range comes from ring_offsets via part_offsets.
                let r = match (geoms, parts, rings) {
                    (Some(g), Some(p), Some(r)) => {
                        ring_off_pair(r, part_off(p, geom_off(g, index)?)?)?
                    }
                    (Some(g), Some(p), None) => part_off_pair(p, geom_off(g, index)?)?,
                    (None, Some(p), Some(r)) => ring_off_pair(r, part_off(p, index)?)?,
                    (None, Some(p), None) => part_off_pair(p, index)?,
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
                let geom_range = geom_off_pair(geoms, index)?;
                // When ring_offsets exist (polygon geometry present), geometry_offsets indexes
                // into part_offsets which indexes into ring_offsets for vertex indices.
                // When only part_offsets exist, geometry_offsets indexes into part_offsets
                // which gives direct vertex indices.
                // When neither exist, geometry_offsets gives direct vertex indices.
                match (parts, rings) {
                    (Some(parts), Some(rings)) => geom_range
                        .map(|p| v(ring_off(rings, part_off(parts, p)?)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|cs| {
                            GeoGeom::MultiPoint(MultiPoint(cs.into_iter().map(Point).collect()))
                        }),
                    (Some(parts), None) => geom_range
                        .map(|p| v(part_off(parts, p)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|cs| {
                            GeoGeom::MultiPoint(MultiPoint(cs.into_iter().map(Point).collect()))
                        }),
                    (None, _) => geom_range.map(&v).collect::<Result<Vec<_>, _>>().map(|cs| {
                        GeoGeom::MultiPoint(MultiPoint(cs.into_iter().map(Point).collect()))
                    }),
                }
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let geom_range = geom_off_pair(geoms, index)?;
                // geometry_offsets indexes into part_offsets for each linestring.
                // When ring_offsets exist (polygon geometry present), part_offsets indexes
                // into ring_offsets for vertex ranges. Otherwise, part_offsets directly
                // gives vertex ranges.
                if let Some(rings) = rings {
                    geom_range
                        .map(|p| line(ring_off_pair(rings, part_off(parts, p)?)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|ls| GeoGeom::MultiLineString(MultiLineString(ls)))
                } else {
                    geom_range
                        .map(|p| line(part_off_pair(parts, p)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|ls| GeoGeom::MultiLineString(MultiLineString(ls)))
                }
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

    /// Add a geometry to this decoded geometry collection.
    /// This is the reverse of `to_geojson` - it converts a `geo_types::Geometry<i32>`
    /// into the internal MLT representation with offset arrays.
    #[must_use]
    pub fn with_geom(mut self, geom: &GeoGeom) -> Self {
        self.push_geom(geom);
        self
    }

    /// Add a geometry to this decoded geometry collection (mutable version).
    pub fn push_geom(&mut self, geom: &GeoGeom) {
        match geom {
            GeoGeom::Point(p) => self.push_point(p.0),
            GeoGeom::Line(l) => {
                self.push_linestring(&LineString(vec![l.start, l.end]));
            }
            GeoGeom::LineString(ls) => self.push_linestring(ls),
            GeoGeom::Polygon(p) => self.push_polygon(p),
            GeoGeom::MultiPoint(mp) => self.push_multi_point(mp),
            GeoGeom::MultiLineString(mls) => self.push_multi_linestring(mls),
            GeoGeom::MultiPolygon(mp) => self.push_multi_polygon(mp),
            GeoGeom::Triangle(t) => {
                self.push_polygon(&Polygon::new(LineString(vec![t.0, t.1, t.2]), vec![]));
            }
            GeoGeom::Rect(r) => {
                self.push_polygon(&r.to_polygon());
            }
            GeoGeom::GeometryCollection(gc) => {
                for g in gc {
                    self.push_geom(g);
                }
            }
        }
    }

    fn push_point(&mut self, coord: Coord32) {
        self.vector_types.push(GeometryType::Point);
        self.vertices
            .get_or_insert_with(Vec::new)
            .extend([coord.x, coord.y]);
    }

    fn push_linestring(&mut self, ls: &LineString<i32>) {
        self.vector_types.push(GeometryType::LineString);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let start_idx = u32::try_from(verts.len() / 2).expect("vertex count overflow");

        for coord in ls.coords() {
            verts.extend([coord.x, coord.y]);
        }

        let end_idx = u32::try_from(verts.len() / 2).expect("vertex count overflow");

        // If ring_offsets exists (i.e., there's a Polygon in the layer),
        // add LineString vertex indices to ring_offsets instead of part_offsets.
        // This matches Java's behavior where LineString adds to numRings when containsPolygon.
        if let Some(rings) = &mut self.ring_offsets {
            // Add to ring_offsets - LineString vertices go here when Polygon is present
            if rings.is_empty() {
                rings.push(start_idx);
            }
            rings.push(end_idx);
        } else {
            // No polygon yet - add to part_offsets as vertex indices
            let parts = self.part_offsets.get_or_insert_with(Vec::new);
            if parts.is_empty() {
                parts.push(start_idx);
            }
            parts.push(end_idx);
        }
    }

    fn push_polygon(&mut self, poly: &Polygon<i32>) {
        self.vector_types.push(GeometryType::Polygon);

        let verts = self.vertices.get_or_insert_with(Vec::new);

        // Only on the very first polygon: if LineStrings were pushed before us,
        // their vertex offsets are sitting in part_offsets. Move them to
        // ring_offsets now, before we set up ring_offsets for polygon use.
        // On subsequent polygons ring_offsets is already initialised and
        // part_offsets holds polygon ring-range data — leave both alone.
        if self.ring_offsets.is_none()
            && let Some(linestring_parts) = self.part_offsets.take()
        {
            self.ring_offsets = Some(linestring_parts);
        }

        let rings = self.ring_offsets.get_or_insert_with(Vec::new);
        let parts = self.part_offsets.get_or_insert_with(Vec::new);

        // parts[i] stores the ring index where polygon i starts
        // Number of existing rings = rings.len() - 1 (since rings is an offset array)
        let ring_count = if rings.is_empty() { 0 } else { rings.len() - 1 };
        if parts.is_empty() {
            parts.push(u32::try_from(ring_count).expect("ring count overflow"));
        }

        // Push exterior ring (without closing vertex - MLT omits it)
        let ext = poly.exterior();
        let ext_coords: Vec<_> = if ext.0.last() == ext.0.first() && ext.0.len() > 1 {
            ext.0[..ext.0.len() - 1].to_vec()
        } else {
            ext.0.clone()
        };

        let ring_start = u32::try_from(verts.len() / 2).expect("vertex count overflow");
        if rings.is_empty() {
            rings.push(ring_start);
        }
        for coord in &ext_coords {
            verts.extend([coord.x, coord.y]);
        }
        rings.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));

        // Push interior rings (holes)
        for hole in poly.interiors() {
            let hole_coords: Vec<_> = if hole.0.last() == hole.0.first() && hole.0.len() > 1 {
                hole.0[..hole.0.len() - 1].to_vec()
            } else {
                hole.0.clone()
            };
            for coord in &hole_coords {
                verts.extend([coord.x, coord.y]);
            }
            rings.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));
        }

        // After adding this polygon's rings, record the new ring count
        let new_ring_count = rings.len() - 1;
        parts.push(u32::try_from(new_ring_count).expect("ring count overflow"));
    }

    fn push_multi_point(&mut self, mp: &MultiPoint<i32>) {
        self.vector_types.push(GeometryType::MultiPoint);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);

        let start_idx = u32::try_from(verts.len() / 2).expect("vertex count overflow");
        if geoms.is_empty() {
            geoms.push(start_idx);
        }

        for point in mp {
            verts.extend([point.0.x, point.0.y]);
        }

        geoms.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));
    }

    fn push_multi_linestring(&mut self, mls: &MultiLineString<i32>) {
        self.vector_types.push(GeometryType::MultiLineString);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);
        let parts = self.part_offsets.get_or_insert_with(Vec::new);

        // geoms stores indices into parts (linestring count)
        // Current linestring count = parts.len() - 1 (since parts is offset array)
        let ls_count = if parts.is_empty() { 0 } else { parts.len() - 1 };
        if geoms.is_empty() {
            geoms.push(u32::try_from(ls_count).expect("part count overflow"));
        }

        for ls in mls {
            let start_idx = u32::try_from(verts.len() / 2).expect("vertex count overflow");
            if parts.is_empty() {
                parts.push(start_idx);
            }
            for coord in ls.coords() {
                verts.extend([coord.x, coord.y]);
            }
            parts.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));
        }

        // After adding all linestrings, record the new linestring count
        let new_ls_count = parts.len() - 1;
        geoms.push(u32::try_from(new_ls_count).expect("part count overflow"));
    }

    fn push_multi_polygon(&mut self, mp: &MultiPolygon<i32>) {
        self.vector_types.push(GeometryType::MultiPolygon);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);
        let parts = self.part_offsets.get_or_insert_with(Vec::new);
        let rings = self.ring_offsets.get_or_insert_with(Vec::new);

        // geoms stores indices into parts (polygon count)
        // Current polygon count = parts.len() - 1 (since parts is offset array)
        let poly_count = if parts.is_empty() { 0 } else { parts.len() - 1 };
        if geoms.is_empty() {
            geoms.push(u32::try_from(poly_count).expect("part count overflow"));
        }

        for poly in mp {
            // parts stores indices into rings (ring count for each polygon)
            let ring_count = if rings.is_empty() { 0 } else { rings.len() - 1 };
            if parts.is_empty() {
                parts.push(u32::try_from(ring_count).expect("ring count overflow"));
            }

            // Push exterior ring (without closing vertex)
            let ext = poly.exterior();
            let ext_coords: Vec<_> = if ext.0.last() == ext.0.first() && ext.0.len() > 1 {
                ext.0[..ext.0.len() - 1].to_vec()
            } else {
                ext.0.clone()
            };

            let ring_start = u32::try_from(verts.len() / 2).expect("vertex count overflow");
            if rings.is_empty() {
                rings.push(ring_start);
            }
            for coord in &ext_coords {
                verts.extend([coord.x, coord.y]);
            }
            rings.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));

            // Push interior rings (holes)
            for hole in poly.interiors() {
                let hole_coords: Vec<_> = if hole.0.last() == hole.0.first() && hole.0.len() > 1 {
                    hole.0[..hole.0.len() - 1].to_vec()
                } else {
                    hole.0.clone()
                };
                for coord in &hole_coords {
                    verts.extend([coord.x, coord.y]);
                }
                rings.push(u32::try_from(verts.len() / 2).expect("vertex count overflow"));
            }

            // After adding this polygon's rings, record the new ring count
            let new_ring_count = rings.len() - 1;
            parts.push(u32::try_from(new_ring_count).expect("ring count overflow"));
        }

        // After adding all polygons, record the new polygon count
        let new_poly_count = parts.len() - 1;
        geoms.push(u32::try_from(new_poly_count).expect("part count overflow"));
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
#[derive(Debug, Clone, PartialEq, PartialOrd, arbitrary::Arbitrary)]
enum ArbitraryGeometry {
    Point((i32, i32)),
    // FIXME: Add LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, once supported upstream
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl From<ArbitraryGeometry> for crate::geojson::Geom32 {
    fn from(value: ArbitraryGeometry) -> Self {
        use crate::geojson::Geom32 as G;
        let cord = |(x, y)| Coord { x, y };
        match value {
            ArbitraryGeometry::Point((x, y)) => G::Point(Point(cord((x, y)))),
            // FIXME: once fully working, add the rest
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for DecodedGeometry {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let geoms = u.arbitrary_iter::<ArbitraryGeometry>()?;
        let mut decoded = DecodedGeometry::default();
        for geo in geoms {
            let geo = crate::geojson::Geom32::from(geo?);
            decoded.push_geom(&geo);
        }
        Ok(decoded)
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedEncodedGeometry {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded = u.arbitrary()?;
        let enc = u.arbitrary()?;
        let geom =
            Self::from_decoded(&decoded, enc).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(geom)
    }
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

impl FromDecoded<'_> for OwnedEncodedGeometry {
    type Input = DecodedGeometry;
    type Encoder = GeometryEncoder;

    fn from_decoded(decoded: &Self::Input, config: Self::Encoder) -> Result<Self, MltError> {
        encode::encode_geometry(decoded, &config)
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
            match stream.meta.stream_type {
                StreamType::Present => {}
                StreamType::Data(v) => match v {
                    DictionaryType::Vertex => {
                        let v = stream.decode_bits_u32()?.decode_i32()?;
                        vertices.set_once(v)?;
                    }
                    _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                },
                StreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
                StreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    // LogicalStream2<U> -> LogicalStream -> trait LogicalStreamEncoding<T>
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
    use crate::v01::geometry::encode::GeometryEncoder;

    /// Encode, serialize, parse, and decode a `DecodedGeometry`.
    /// The input must already be in the dense canonical form that `from_encoded`
    /// produces (i.e. built via a previous `roundtrip` call, not via `push_*`).
    fn roundtrip(decoded: &DecodedGeometry, encoder: GeometryEncoder) -> DecodedGeometry {
        let encoded_geom = OwnedEncodedGeometry::from_decoded(decoded, encoder);
        let encoded_geom = encoded_geom.expect("Failed to encode");

        // Serialize to bytes (write_to includes the stream count varint)
        let mut buffer = Vec::new();
        encoded_geom
            .write_to(&mut buffer)
            .expect("Failed to serialize");

        // Now parse (parse expects varint stream count + streams)
        let (remaining, parsed) = EncodedGeometry::parse(&buffer).expect("Failed to parse");
        assert!(remaining.is_empty(), "Remaining bytes after parse");

        DecodedGeometry::from_encoded(parsed).expect("Failed to decode")
    }

    /// Build a `DecodedGeometry` from a sequence of `GeoGeom` values via
    /// `push_geom` and perform a two-cycle encode/decode:
    ///
    /// 1. push → encode → decode  (`canonical`): exercises `push_geom` and
    ///    `normalize_geometry_offsets`; normalises the sparse push_* layout to
    ///    the dense form that `from_encoded` always returns.
    /// 2. canonical → encode → decode  (`output`): verifies idempotency of
    ///    encode/decode on the canonical form
    ///
    /// Comparing `canonical == output` catches both panics in the push path
    /// and silent data corruption in encode/decode
    fn roundtrip_via_push(
        geoms: &[GeoGeom],
        encoder: GeometryEncoder,
    ) -> (DecodedGeometry, DecodedGeometry) {
        let mut pushed = DecodedGeometry::default();
        for g in geoms {
            pushed.push_geom(g);
        }
        let canonical = roundtrip(&pushed, encoder);
        let output = roundtrip(&canonical, encoder);
        (canonical, output)
    }

    fn arb_coord() -> impl Strategy<Value = Coord32> {
        (any::<i32>(), any::<i32>()).prop_map(|(x, y)| Coord32 { x, y })
    }

    fn arb_geom() -> impl Strategy<Value = GeoGeom> {
        prop_oneof![
            // Point
            arb_coord().prop_map(Point).prop_map(GeoGeom::Point),
            // LineString
            prop::collection::vec(arb_coord(), 2..10)
                .prop_map(|coords| GeoGeom::LineString(LineString(coords))),
            // Polygon (single exterior ring, no holes)
            prop::collection::vec(arb_coord(), 3..8).prop_map(|mut coords| {
                coords.push(coords[0]);
                GeoGeom::Polygon(Polygon::new(LineString(coords), vec![]))
            }),
            // MultiPoint
            prop::collection::vec(arb_coord(), 2..8).prop_map(|coords| {
                GeoGeom::MultiPoint(MultiPoint(coords.into_iter().map(Point).collect()))
            }),
            // MultiLineString
            prop::collection::vec(prop::collection::vec(arb_coord(), 2..6), 2..5,).prop_map(
                |lines| GeoGeom::MultiLineString(MultiLineString(
                    lines.into_iter().map(LineString).collect(),
                ))
            ),
            // MultiPolygon
            prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                coords.push(coords[0]);
                GeoGeom::MultiPolygon(MultiPolygon(vec![Polygon::new(LineString(coords), vec![])]))
            }),
        ]
    }

    /// Strategy for a mixed `LineString` + `MultiLineString` layer
    fn arb_mixed_linestring_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(arb_geom(), 2..12)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, GeoGeom::LineString(_) | GeoGeom::MultiLineString(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both LS and MLS", |geoms| {
                geoms.iter().any(|g| matches!(g, GeoGeom::LineString(_)))
                    && geoms
                        .iter()
                        .any(|g| matches!(g, GeoGeom::MultiLineString(_)))
            })
    }

    /// Strategy for a mixed `Point` + `MultiPoint` layer.
    fn arb_mixed_point_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(arb_geom(), 2..12)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, GeoGeom::Point(_) | GeoGeom::MultiPoint(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both P and MP", |geoms| {
                geoms.iter().any(|g| matches!(g, GeoGeom::Point(_)))
                    && geoms.iter().any(|g| matches!(g, GeoGeom::MultiPoint(_)))
            })
    }

    /// Strategy for a mixed `Polygon` + `MultiPolygon` layer.
    fn arb_mixed_polygon_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(arb_geom(), 2..8)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, GeoGeom::Polygon(_) | GeoGeom::MultiPolygon(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both Poly and MPoly", |geoms| {
                geoms.iter().any(|g| matches!(g, GeoGeom::Polygon(_)))
                    && geoms.iter().any(|g| matches!(g, GeoGeom::MultiPolygon(_)))
            })
    }

    proptest! {
        #[test]
        fn test_geometry_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geom in arb_geom(),
        ) {
            let (canonical, output) = roundtrip_via_push(&[geom], encoder);
            prop_assert_eq!(output, canonical);
        }

        #[test]
        fn test_mixed_linestring_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_mixed_linestring_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }

        #[test]
        fn test_mixed_point_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_mixed_point_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }

        #[test]
        fn test_mixed_polygon_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_mixed_polygon_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }
    }
}
