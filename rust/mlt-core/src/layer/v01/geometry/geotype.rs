use std::ops::Range;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

use crate::MltError::{
    self, GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds, NoGeometryOffsets,
    NoPartOffsets, NoRingOffsets,
};
use crate::geojson::Coord32;
use crate::v01::{DecodedGeometry, GeometryType};

impl DecodedGeometry {
    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> Result<crate::convert::geojson::Geom32, MltError> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

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
            let s = match verts.get(idx * 2..(idx * 2) + 2) {
                Some(v) => v,
                None => Err(GeometryVertexOutOfBounds {
                    index,
                    vertex: idx,
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
                // Resolve through hierarchy: geoms? -> parts? -> rings? -> vertex
                let idx = geoms.map_or(Ok(index), |g| geom_off(g, index))?;
                let idx = parts.map_or(Ok(idx), |p| part_off(p, idx))?;
                let idx = rings.map_or(Ok(idx), |r| ring_off(r, idx))?;
                Ok(crate::convert::geojson::Geom32::Point(Point(v(idx)?)))
            }
            GeometryType::LineString => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                // Get part index: use geoms[index] if present, else index directly
                let part_idx = geoms.map_or(Ok(index), |g| geom_off(g, index))?;
                // With rings: parts[part_idx] gives ring index, use ring_offsets for vertex range
                // Without rings: use part_offsets directly for vertex range
                let vertex_range = if let Some(r) = rings {
                    ring_off_pair(r, part_off(parts, part_idx)?)?
                } else {
                    part_off_pair(parts, part_idx)?
                };
                line(vertex_range).map(crate::convert::geojson::Geom32::LineString)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let i = geoms
                    .map(|g| geom_off(g, index))
                    .transpose()?
                    .unwrap_or(index);
                rings_in(part_off_pair(parts, i)?, rings)
                    .map(crate::convert::geojson::Geom32::Polygon)
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
                            crate::convert::geojson::Geom32::MultiPoint(MultiPoint(
                                cs.into_iter().map(Point).collect(),
                            ))
                        }),
                    (Some(parts), None) => geom_range
                        .map(|p| v(part_off(parts, p)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|cs| {
                            crate::convert::geojson::Geom32::MultiPoint(MultiPoint(
                                cs.into_iter().map(Point).collect(),
                            ))
                        }),
                    (None, _) => geom_range.map(&v).collect::<Result<Vec<_>, _>>().map(|cs| {
                        crate::convert::geojson::Geom32::MultiPoint(MultiPoint(
                            cs.into_iter().map(Point).collect(),
                        ))
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
                        .map(|ls| {
                            crate::convert::geojson::Geom32::MultiLineString(MultiLineString(ls))
                        })
                } else {
                    geom_range
                        .map(|p| line(part_off_pair(parts, p)?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|ls| {
                            crate::convert::geojson::Geom32::MultiLineString(MultiLineString(ls))
                        })
                }
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                geom_off_pair(geoms, index)?
                    .map(|p| rings_in(part_off_pair(parts, p)?, rings))
                    .collect::<Result<Vec<Polygon<i32>>, _>>()
                    .map(|ps| crate::convert::geojson::Geom32::MultiPolygon(MultiPolygon(ps)))
            }
        }
    }

    /// Add a geometry to this decoded geometry collection.
    /// This is the reverse of `to_geojson` - it converts a `geo_types::Geometry<i32>`
    /// into the internal MLT representation with offset arrays.
    #[must_use]
    pub fn with_geom(mut self, geom: &crate::convert::geojson::Geom32) -> Self {
        self.push_geom(geom);
        self
    }

    /// Add a geometry to this decoded geometry collection (mutable version).
    pub fn push_geom(&mut self, geom: &crate::convert::geojson::Geom32) {
        match geom {
            crate::convert::geojson::Geom32::Point(p) => self.push_point(p.0),
            crate::convert::geojson::Geom32::Line(l) => {
                self.push_linestring(&LineString(vec![l.start, l.end]));
            }
            crate::convert::geojson::Geom32::LineString(ls) => self.push_linestring(ls),
            crate::convert::geojson::Geom32::Polygon(p) => self.push_polygon(p),
            crate::convert::geojson::Geom32::MultiPoint(mp) => self.push_multi_point(mp),
            crate::convert::geojson::Geom32::MultiLineString(mls) => {
                self.push_multi_linestring(mls);
            }
            crate::convert::geojson::Geom32::MultiPolygon(mp) => self.push_multi_polygon(mp),
            crate::convert::geojson::Geom32::Triangle(t) => {
                self.push_polygon(&Polygon::new(LineString(vec![t.0, t.1, t.2]), vec![]));
            }
            crate::convert::geojson::Geom32::Rect(r) => {
                self.push_polygon(&r.to_polygon());
            }
            crate::convert::geojson::Geom32::GeometryCollection(gc) => {
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
        let num_vertices = u32::try_from(ls.0.len()).expect("vertex count overflow");

        for coord in ls.coords() {
            verts.extend([coord.x, coord.y]);
        }

        // If ring_offsets exists (i.e., there's a Polygon in the layer),
        // add LineString vertex count to ring_offsets instead of part_offsets.
        // This matches Java's behavior where LineString adds to numRings when containsPolygon.
        if let Some(rings) = &mut self.ring_offsets {
            // Add cumulative vertex count to ring_offsets
            let mut cumulative = if rings.is_empty() {
                0
            } else {
                *rings.last().unwrap()
            };
            if rings.is_empty() {
                rings.push(cumulative);
            }
            cumulative += num_vertices;
            rings.push(cumulative);
        } else {
            // No polygon yet - add cumulative vertex count to part_offsets
            let parts = self.part_offsets.get_or_insert_with(Vec::new);
            let mut cumulative = if parts.is_empty() {
                0
            } else {
                *parts.last().unwrap()
            };
            if parts.is_empty() {
                parts.push(cumulative);
            }
            cumulative += num_vertices;
            parts.push(cumulative);
        }
    }

    fn push_polygon(&mut self, poly: &Polygon<i32>) {
        self.vector_types.push(GeometryType::Polygon);

        let verts = self.vertices.get_or_insert_with(Vec::new);

        // Only on the very first polygon: if LineStrings were pushed before us,
        // their vertex offsets are sitting in part_offsets. Move them to
        // ring_offsets now, before we set up ring_offsets for polygon use.
        // On subsequent polygons ring_offsets is already initialized and
        // part_offsets holds polygon ring-range data — leave both alone.
        if self.ring_offsets.is_none()
            && let Some(linestring_parts) = self.part_offsets.take()
        {
            self.ring_offsets = Some(linestring_parts);
        }

        let rings = self.ring_offsets.get_or_insert_with(Vec::new);
        let parts = self.part_offsets.get_or_insert_with(Vec::new);

        // Track cumulative polygon ring count (not including LineString entries)
        // parts.last() gives the current polygon ring count if parts is non-empty
        let mut polygon_ring_count = if parts.is_empty() {
            0_u32
        } else {
            *parts.last().unwrap()
        };

        // Push starting ring count for this polygon (if first polygon)
        if parts.is_empty() {
            parts.push(polygon_ring_count);
        }

        // Track cumulative vertex count for ring_offsets (ignoring Point vertices)
        // ring_offsets stores cumulative vertex counts for polygon rings only
        let mut cumulative_ring_vertices = if rings.is_empty() {
            0
        } else {
            *rings.last().unwrap()
        };
        if rings.is_empty() {
            rings.push(cumulative_ring_vertices);
        }

        // Push exterior ring (without closing vertex - MLT omits it)
        let ext = poly.exterior();
        let ext_coords: Vec<_> = if ext.0.last() == ext.0.first() && ext.0.len() > 1 {
            ext.0[..ext.0.len() - 1].to_vec()
        } else {
            ext.0.clone()
        };

        for coord in &ext_coords {
            verts.extend([coord.x, coord.y]);
        }
        cumulative_ring_vertices += u32::try_from(ext_coords.len()).expect("vertex count overflow");
        rings.push(cumulative_ring_vertices);
        polygon_ring_count += 1;

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
            cumulative_ring_vertices +=
                u32::try_from(hole_coords.len()).expect("vertex count overflow");
            rings.push(cumulative_ring_vertices);
            polygon_ring_count += 1;
        }

        // After adding this polygon's rings, record the cumulative ring count
        parts.push(polygon_ring_count);
    }

    fn push_multi_point(&mut self, mp: &MultiPoint<i32>) {
        self.vector_types.push(GeometryType::MultiPoint);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);

        // geometry_offsets stores cumulative sub-geometry counts
        // Initialize with 0 if empty
        if geoms.is_empty() {
            geoms.push(0);
        }

        let num_points = u32::try_from(mp.0.len()).expect("point count overflow");
        for point in mp {
            verts.extend([point.0.x, point.0.y]);
        }

        // Add cumulative count
        let prev = *geoms.last().unwrap();
        geoms.push(prev + num_points);
    }

    fn push_multi_linestring(&mut self, mls: &MultiLineString<i32>) {
        self.vector_types.push(GeometryType::MultiLineString);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);
        let num_linestrings = u32::try_from(mls.0.len()).expect("linestring count overflow");

        // geometry_offsets stores cumulative sub-geometry counts
        if geoms.is_empty() {
            geoms.push(0);
        }

        // When a Polygon is present (ring_offsets exists), LineString vertex counts
        // go to ring_offsets instead of part_offsets. This matches Java's behavior.
        if let Some(rings) = &mut self.ring_offsets {
            // Polygon is present - add cumulative vertex counts to ring_offsets
            let mut cumulative = if rings.is_empty() {
                0
            } else {
                *rings.last().unwrap()
            };
            if rings.is_empty() {
                rings.push(cumulative);
            }
            for ls in mls {
                for coord in ls.coords() {
                    verts.extend([coord.x, coord.y]);
                }
                cumulative += u32::try_from(ls.0.len()).expect("vertex count overflow");
                rings.push(cumulative);
            }
        } else {
            // No Polygon - use part_offsets for cumulative vertex counts
            // This is a "virtual" offset array that tracks cumulative linestring vertex
            // counts, ignoring any Point vertices that may have been added in between.
            let parts = self.part_offsets.get_or_insert_with(Vec::new);

            // Get the current cumulative vertex count from the last entry
            let mut cumulative = if parts.is_empty() {
                0
            } else {
                *parts.last().unwrap()
            };
            if parts.is_empty() {
                parts.push(cumulative);
            }

            for ls in mls {
                for coord in ls.coords() {
                    verts.extend([coord.x, coord.y]);
                }
                cumulative += u32::try_from(ls.0.len()).expect("vertex count overflow");
                parts.push(cumulative);
            }
        }

        // Add cumulative count
        let prev = *geoms.last().unwrap();
        geoms.push(prev + num_linestrings);
    }

    fn push_multi_polygon(&mut self, mp: &MultiPolygon<i32>) {
        self.vector_types.push(GeometryType::MultiPolygon);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let geoms = self.geometry_offsets.get_or_insert_with(Vec::new);
        let num_polygons = u32::try_from(mp.0.len()).expect("polygon count overflow");

        // geometry_offsets stores cumulative sub-geometry counts
        if geoms.is_empty() {
            geoms.push(0);
        }

        // If LineStrings were pushed before us, their vertex offsets are in part_offsets.
        // Move them to ring_offsets before we use part_offsets for polygon ring counts.
        if self.ring_offsets.is_none()
            && let Some(linestring_parts) = self.part_offsets.take()
        {
            self.ring_offsets = Some(linestring_parts);
        }

        let parts = self.part_offsets.get_or_insert_with(Vec::new);
        let rings = self.ring_offsets.get_or_insert_with(Vec::new);

        // Track cumulative polygon ring count (not including LineString entries)
        // parts.last() gives the current polygon ring count if parts is non-empty
        let mut polygon_ring_count = if parts.is_empty() {
            0_u32
        } else {
            *parts.last().unwrap()
        };

        // Track cumulative vertex count for ring_offsets (ignoring Point vertices)
        let mut cumulative_ring_vertices = if rings.is_empty() {
            0
        } else {
            *rings.last().unwrap()
        };
        if rings.is_empty() {
            rings.push(cumulative_ring_vertices);
        }

        for poly in mp {
            // Push starting ring count for this polygon
            if parts.is_empty() {
                parts.push(polygon_ring_count);
            }

            // Push exterior ring (without closing vertex)
            let ext = poly.exterior();
            let ext_coords: Vec<_> = if ext.0.last() == ext.0.first() && ext.0.len() > 1 {
                ext.0[..ext.0.len() - 1].to_vec()
            } else {
                ext.0.clone()
            };

            for coord in &ext_coords {
                verts.extend([coord.x, coord.y]);
            }
            cumulative_ring_vertices +=
                u32::try_from(ext_coords.len()).expect("vertex count overflow");
            rings.push(cumulative_ring_vertices);
            polygon_ring_count += 1;

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
                cumulative_ring_vertices +=
                    u32::try_from(hole_coords.len()).expect("vertex count overflow");
                rings.push(cumulative_ring_vertices);
                polygon_ring_count += 1;
            }

            // After adding this polygon's rings, record the cumulative ring count
            parts.push(polygon_ring_count);
        }

        // Add cumulative count
        let prev = *geoms.last().unwrap();
        geoms.push(prev + num_polygons);
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
        let coord = |(x, y)| Coord { x, y };
        match value {
            ArbitraryGeometry::Point((x, y)) => G::Point(Point(coord((x, y)))),
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
impl arbitrary::Arbitrary<'_> for crate::v01::OwnedEncodedGeometry {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        use crate::encode::FromDecoded as _;
        let decoded = u.arbitrary()?;
        let enc = u.arbitrary()?;
        let geom =
            Self::from_decoded(&decoded, enc).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(geom)
    }
}

#[cfg(test)]
mod tests {
    use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, wkt};
    use proptest::prelude::*;

    use super::*;
    use crate::geojson::{Coord32, Geom32 as GeoGeom};
    use crate::v01::{EncodedGeometry, GeometryEncoder, IntEncoding, OwnedEncodedGeometry};
    use crate::{FromDecoded as _, FromEncoded as _};

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
    /// 1. push -> encode -> decode  (`canonical`): exercises `push_geom` and
    ///    `normalize_geometry_offsets`; normalizes the sparse push_* layout to
    ///    the dense form that `from_encoded` always returns.
    /// 2. canonical -> encode -> decode  (`output`): verifies idempotency of
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

    /// Mixing `LineString` with `MultiLineString`
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

    /// Mixing `Point` with `MultiPoint`
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

    /// Mixing `Polygon` with `MultiPolygon`
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

    /// Mixing `Point` with `MultiLineString`
    fn arb_cross_point_mls_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(
            prop_oneof![
                arb_coord().prop_map(Point).prop_map(GeoGeom::Point),
                prop::collection::vec(prop::collection::vec(arb_coord(), 2..6), 2..5).prop_map(
                    |lines| {
                        GeoGeom::MultiLineString(MultiLineString(
                            lines.into_iter().map(LineString).collect(),
                        ))
                    }
                ),
            ],
            2..12,
        )
        .prop_filter("needs both Point and MultiLineString", |geoms| {
            geoms.iter().any(|g| matches!(g, GeoGeom::Point(_)))
                && geoms
                    .iter()
                    .any(|g| matches!(g, GeoGeom::MultiLineString(_)))
        })
    }

    /// Mixing `Point` with `MultiPolygon`.
    fn arb_cross_point_mpoly_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(
            prop_oneof![
                arb_coord().prop_map(Point).prop_map(GeoGeom::Point),
                prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                    coords.push(coords[0]);
                    GeoGeom::MultiPolygon(MultiPolygon(vec![Polygon::new(
                        LineString(coords),
                        vec![],
                    )]))
                }),
            ],
            2..10,
        )
        .prop_filter("needs both Point and MultiPolygon", |geoms| {
            geoms.iter().any(|g| matches!(g, GeoGeom::Point(_)))
                && geoms.iter().any(|g| matches!(g, GeoGeom::MultiPolygon(_)))
        })
    }

    /// Mixing `LineString` with `MultiPolygon`
    fn arb_cross_ls_mpoly_geoms() -> impl Strategy<Value = Vec<GeoGeom>> {
        prop::collection::vec(
            prop_oneof![
                prop::collection::vec(arb_coord(), 2..8)
                    .prop_map(|coords| GeoGeom::LineString(LineString(coords))),
                prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                    coords.push(coords[0]);
                    GeoGeom::MultiPolygon(MultiPolygon(vec![Polygon::new(
                        LineString(coords),
                        vec![],
                    )]))
                }),
            ],
            2..10,
        )
        .prop_filter("needs both LineString and MultiPolygon", |geoms| {
            geoms.iter().any(|g| matches!(g, GeoGeom::LineString(_)))
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

        #[ignore = "encoder does not implement this correctly"]
        #[test]
        fn test_cross_point_mls_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_cross_point_mls_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }

        #[ignore = "encoder does not implement this correctly"]
        #[test]
        fn test_cross_point_mpoly_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_cross_point_mpoly_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }

        #[test]
        fn test_cross_ls_mpoly_roundtrip(
            encoder in any::<GeometryEncoder>(),
            geoms in arb_cross_ls_mpoly_geoms(),
        ) {
            let (canonical, output) = roundtrip_via_push(&geoms, encoder);
            prop_assert_eq!(output, canonical);
        }
    }

    /// Verifies that a Morton-encoded vertex dictionary is fully expanded inside `from_encoded`.
    /// This ensures `DecodedGeometry` always holds flat `(x, y)` pairs.
    #[test]
    fn test_morton_vertex_dictionary_expansion() {
        use crate::v01::{
            DictionaryType, IntEncoder, LengthType, LogicalEncoding, MortonMeta, OffsetType,
            OwnedStream, StreamMeta, StreamType,
        };

        // meta: single LineString
        let meta = OwnedStream::encode_u32s_of_type(
            &[GeometryType::LineString as u32],
            IntEncoder::varint(),
            StreamType::Length(LengthType::VarBinary),
        )
        .unwrap();

        // parts: one LineString of length 4
        let parts = OwnedStream::encode_u32s_of_type(
            &[4u32],
            IntEncoder::varint(),
            StreamType::Length(LengthType::Parts),
        )
        .unwrap();

        // vertex offsets: per-vertex indices into the Morton dictionary
        let vertex_offsets_stream = OwnedStream::encode_u32s_of_type(
            &[0u32, 1, 2, 1],
            IntEncoder::varint(),
            StreamType::Offset(OffsetType::Vertex),
        )
        .unwrap();

        // Morton vertex dictionary: 3 unique entries.
        // Raw codes [0, 16, 32] -> delta-encoded as [0, 16, 16].
        // The MortonDelta logical encoding means the decoder will undo the delta,
        // then decode each Morton code to an (x, y) pair.
        let morton_deltas = vec![0u32, 16, 16];
        let (data, physical_encoding) = IntEncoder::varint()
            .physical
            .encode_u32s(morton_deltas)
            .unwrap();
        let morton_dict = OwnedStream {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Morton),
                IntEncoding::new(
                    LogicalEncoding::MortonDelta(MortonMeta {
                        num_bits: 3,
                        coordinate_shift: 0,
                    }),
                    physical_encoding,
                ),
                3, // 3 dictionary entries -> 3 physical u32 values
            ),
            data,
        };

        // Assemble, serialize, parse, decode
        let owned = OwnedEncodedGeometry {
            meta,
            items: vec![parts, vertex_offsets_stream, morton_dict],
        };
        let mut buffer = Vec::new();
        owned.write_to(&mut buffer).unwrap();
        let (remaining, parsed) = EncodedGeometry::parse(&buffer).unwrap();
        assert!(remaining.is_empty());
        let decoded = DecodedGeometry::from_encoded(parsed).unwrap();

        assert_eq!(decoded.vertices, Some(vec![0i32, 0, 4, 0, 0, 4, 4, 0]));

        let geom = decoded.to_geojson(0).unwrap();
        assert_eq!(geom, wkt!(LINESTRING(0 0,4 0,0 4,4 0)).into());
    }
}
