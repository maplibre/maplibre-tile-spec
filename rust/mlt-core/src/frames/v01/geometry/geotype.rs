use std::ops::Range;

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

use crate::MltError::{
    self, GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds,
    NoGeometryOffsets, NoPartOffsets, NoRingOffsets,
};
use crate::geojson::{Coord32, Geom32};
use crate::utils::AsUsize as _;
use crate::v01::{GeometryType, GeometryValues};

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

impl GeometryValues {
    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> Result<Geom32, MltError> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

        let off = |s: &[u32], idx: usize, field: &'static str| -> Result<usize, MltError> {
            s.get(idx)
                .map(|&v| v.as_usize())
                .ok_or(GeometryOutOfBounds {
                    index,
                    field,
                    idx,
                    len: s.len(),
                })
        };
        let off_pair =
            |s: &[u32], idx: usize, field: &'static str| -> Result<Range<usize>, MltError> {
                Ok(off(s, idx, field)?..off(s, idx + 1, field)?)
            };

        let geom_off = |s: &[u32], i: usize| off(s, i, "geometry_offsets");
        let part_off = |s: &[u32], i: usize| off(s, i, "part_offsets");
        let ring_off = |s: &[u32], i: usize| off(s, i, "ring_offsets");
        let geom_range = |s: &[u32], i: usize| off_pair(s, i, "geometry_offsets");
        let part_range = |s: &[u32], i: usize| off_pair(s, i, "part_offsets");
        let ring_range = |s: &[u32], i: usize| off_pair(s, i, "ring_offsets");

        let vert = |idx: usize| -> Result<Coord32, MltError> {
            verts
                .get(idx * 2..idx * 2 + 2)
                .map(|s| Coord { x: s[0], y: s[1] })
                .ok_or(GeometryVertexOutOfBounds {
                    index,
                    vertex: idx,
                    count: verts.len() / 2,
                })
        };
        let line =
            |r: Range<usize>| -> Result<LineString<i32>, MltError> { r.map(&vert).collect() };
        let closed_ring = |r: Range<usize>| -> Result<LineString<i32>, MltError> {
            let first = r.start;
            let mut coords: Vec<Coord32> = r.map(&vert).collect::<Result<_, _>>()?;
            coords.push(vert(first)?);
            Ok(LineString(coords))
        };
        let poly_from_rings =
            |part_rng: Range<usize>, r: &[u32]| -> Result<Polygon<i32>, MltError> {
                let mut rings = part_rng
                    .map(|idx| closed_ring(ring_range(r, idx)?))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter();
                Ok(Polygon::new(
                    rings.next().unwrap_or_else(|| LineString(vec![])),
                    rings.collect(),
                ))
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
                Ok(Geom32::Point(Point(vert(idx)?)))
            }
            GeometryType::LineString => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                // Get part index: use geoms[index] if present, else index directly
                let part_idx = geoms.map_or(Ok(index), |geom| geom_off(geom, index))?;
                // With rings: parts[part_idx] gives ring index, use ring_offsets for vertex range
                // Without rings: use part_offsets directly for vertex range
                let vert_range = match rings {
                    Some(ring) => ring_range(ring, part_off(parts, part_idx)?)?,
                    None => part_range(parts, part_idx)?,
                };
                line(vert_range).map(Geom32::LineString)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let idx = geoms
                    .map(|geom| geom_off(geom, index))
                    .transpose()?
                    .unwrap_or(index);
                poly_from_rings(part_range(parts, idx)?, rings).map(Geom32::Polygon)
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
                // Resolve vertex index through parts?->rings? hierarchy
                // When ring_offsets exist (polygon geometry present), geometry_offsets indexes
                // into part_offsets which indexes into ring_offsets for vertex indices.
                // When only part_offsets exist, geometry_offsets indexes into part_offsets
                // which gives direct vertex indices.
                // When neither exist, geometry_offsets gives direct vertex indices.
                let coords: Result<Vec<_>, _> = match (parts, rings) {
                    (Some(parts), Some(rings)) => geom_rng
                        .map(|idx| vert(ring_off(rings, part_off(parts, idx)?)?))
                        .collect(),
                    (Some(part), None) => geom_rng.map(|idx| vert(part_off(part, idx)?)).collect(),
                    (None, _) => geom_rng.map(&vert).collect(),
                };
                Ok(Geom32::MultiPoint(MultiPoint(
                    coords?.into_iter().map(Point).collect(),
                )))
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let geom_rng = geom_range(geoms, index)?;
                // geometry_offsets indexes into part_offsets for each linestring.
                // When ring_offsets exist (polygon geometry present), part_offsets indexes
                // into ring_offsets for vertex ranges. Otherwise, part_offsets directly
                // gives vertex ranges.
                let lines: Result<Vec<_>, _> = match rings {
                    Some(ring) => geom_rng
                        .map(|idx| line(ring_range(ring, part_off(parts, idx)?)?))
                        .collect(),
                    None => geom_rng.map(|idx| line(part_range(parts, idx)?)).collect(),
                };
                Ok(Geom32::MultiLineString(MultiLineString(lines?)))
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets(index, geom_type))?;
                let parts = parts.ok_or(NoPartOffsets(index, geom_type))?;
                let rings = rings.ok_or(NoRingOffsets(index, geom_type))?;
                let polys: Vec<_> = geom_range(geoms, index)?
                    .map(|idx| poly_from_rings(part_range(parts, idx)?, rings))
                    .collect::<Result<_, _>>()?;
                Ok(Geom32::MultiPolygon(MultiPolygon(polys)))
            }
        }
    }

    /// Add a geometry to this decoded geometry collection.
    /// This is the reverse of `to_geojson` - it converts a `geo_types::Geometry<i32>`
    /// into the internal MLT representation with offset arrays.
    #[must_use]
    pub fn with_geom(mut self, geom: &Geom32) -> Self {
        self.push_geom(geom);
        self
    }

    /// Add a geometry to this decoded geometry collection (mutable version).
    pub fn push_geom(&mut self, geom: &Geom32) {
        match geom {
            Geom32::Point(p) => self.push_point(p.0),
            Geom32::Line(l) => self.push_linestring(&LineString(vec![l.start, l.end])),
            Geom32::LineString(ls) => self.push_linestring(ls),
            Geom32::Polygon(p) => self.push_polygon(p),
            Geom32::MultiPoint(mp) => self.push_multi_point(mp),
            Geom32::MultiLineString(mls) => self.push_multi_linestring(mls),
            Geom32::MultiPolygon(mp) => self.push_multi_polygon(mp),
            Geom32::Triangle(t) => {
                self.push_polygon(&Polygon::new(LineString(vec![t.0, t.1, t.2]), vec![]));
            }
            Geom32::Rect(r) => self.push_polygon(&r.to_polygon()),
            Geom32::GeometryCollection(gc) => {
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
        // If ring_offsets exists (i.e., there's a Polygon in the layer),
        // add LineString vertex count to ring_offsets instead of part_offsets.
        // This matches Java's behavior where LineString adds to numRings when containsPolygon.
        let offsets = self
            .ring_offsets
            .as_mut()
            .unwrap_or_else(|| self.part_offsets.get_or_insert_with(Vec::new));

        push_linestrings(std::iter::once(ls), verts, offsets);
    }

    fn push_polygon(&mut self, poly: &Polygon<i32>) {
        // Only on the very first polygon: if LineStrings were pushed before us,
        // their vertex offsets are sitting in part_offsets. Move them to
        // ring_offsets now, before we set up ring_offsets for polygon use.
        // On subsequent polygons ring_offsets is already initialized and
        // part_offsets holds polygon ring-range data — leave both alone.
        self.vector_types.push(GeometryType::Polygon);
        self.init_polygon_offsets();

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let rings = self.ring_offsets.as_mut().unwrap();
        let parts = self.part_offsets.as_mut().unwrap();

        push_polygon_rings(poly, verts, rings, parts);
    }

    /// Initialize offset arrays for polygon storage. On the first polygon,
    /// moves any `LineString` vertex offsets from `part_offsets` to `ring_offsets`.
    fn init_polygon_offsets(&mut self) {
        if self.ring_offsets.is_none()
            && let Some(ls_parts) = self.part_offsets.take()
        {
            self.ring_offsets = Some(ls_parts);
        }
        init_offsets(self.ring_offsets.get_or_insert_with(Vec::new));
        init_offsets(self.part_offsets.get_or_insert_with(Vec::new));
    }

    fn push_multi_point(&mut self, mp: &MultiPoint<i32>) {
        self.vector_types.push(GeometryType::MultiPoint);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        for point in mp {
            verts.extend([point.0.x, point.0.y]);
        }

        self.push_geometry_count(u32::try_from(mp.0.len()).expect("point count overflow"));
    }

    fn push_multi_linestring(&mut self, mls: &MultiLineString<i32>) {
        self.vector_types.push(GeometryType::MultiLineString);

        let verts = self.vertices.get_or_insert_with(Vec::new);
        // When a Polygon is present (ring_offsets exists), LineString vertex counts
        // go to ring_offsets instead of part_offsets. This matches Java's behavior.
        let offsets = self
            .ring_offsets
            .as_mut()
            .unwrap_or_else(|| self.part_offsets.get_or_insert_with(Vec::new));

        push_linestrings(mls.iter(), verts, offsets);

        self.push_geometry_count(u32::try_from(mls.0.len()).expect("linestring count overflow"));
    }

    fn push_multi_polygon(&mut self, mp: &MultiPolygon<i32>) {
        self.vector_types.push(GeometryType::MultiPolygon);
        self.init_polygon_offsets();

        let verts = self.vertices.get_or_insert_with(Vec::new);
        let rings = self.ring_offsets.as_mut().unwrap();
        let parts = self.part_offsets.as_mut().unwrap();

        for poly in mp {
            push_polygon_rings(poly, verts, rings, parts);
        }

        self.push_geometry_count(u32::try_from(mp.0.len()).expect("polygon count overflow"));
    }

    /// Initialize and update `geometry_offsets` with a sub-geometry count.
    fn push_geometry_count(&mut self, count: u32) {
        let g = self.geometry_offsets.get_or_insert_with(Vec::new);
        init_offsets(g);
        g.push(g.last().unwrap() + count);
    }
}

/// Ensure offset array starts with 0.
fn init_offsets(v: &mut Vec<u32>) {
    if v.is_empty() {
        v.push(0);
    }
}

/// Push a single polygon's rings (exterior + interiors) to the offset arrays.
/// MLT omits closing vertices, so we strip them if present.
fn push_polygon_rings(
    poly: &Polygon<i32>,
    verts: &mut Vec<i32>,
    rings: &mut Vec<u32>,
    parts: &mut Vec<u32>,
) {
    let mut ring_count = *parts.last().unwrap();
    for ring in std::iter::once(poly.exterior()).chain(poly.interiors()) {
        push_ring(ring, verts, rings);
        ring_count += 1;
    }
    parts.push(ring_count);
}

/// Push a ring's coordinates (stripping closing vertex) to verts and update rings offset.
fn push_ring(ring: &LineString<i32>, verts: &mut Vec<i32>, rings: &mut Vec<u32>) {
    let coords = &ring.0;
    let len = if coords.len() > 1 && coords.last() == coords.first() {
        coords.len() - 1
    } else {
        coords.len()
    };
    for c in &coords[..len] {
        verts.extend([c.x, c.y]);
    }
    let prev = *rings.last().unwrap();
    rings.push(prev + u32::try_from(len).expect("vertex count overflow"));
}

/// Push linestrings to vertex buffer and offset array.
fn push_linestrings<'a>(
    iter: impl Iterator<Item = &'a LineString<i32>>,
    verts: &mut Vec<i32>,
    offsets: &mut Vec<u32>,
) {
    init_offsets(offsets);
    for ls in iter {
        for c in ls.coords() {
            verts.extend([c.x, c.y]);
        }
        let prev = *offsets.last().unwrap();
        offsets.push(prev + u32::try_from(ls.0.len()).expect("vertex count overflow"));
    }
}

#[cfg(test)]
mod tests {
    use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, wkt};
    use proptest::prelude::*;

    use super::*;
    use crate::geojson::Coord32;
    use crate::test_helpers::{dec, parser};
    use crate::v01::{EncodedGeometry, Geometry, GeometryEncoder, IntEncoding, RawGeometry};

    /// Encode, serialize, parse, and decode a `GeometryValues`.
    /// The input must already be in the dense canonical form that `from_encoded`
    /// produces (i.e. built via a previous `roundtrip` call, not via `push_*`).
    fn roundtrip(decoded: &GeometryValues, encoder: GeometryEncoder) -> GeometryValues {
        let enc_geom = decoded.clone().encode(encoder).expect("Failed to encode");

        let mut buffer = Vec::new();
        enc_geom.write_to(&mut buffer).expect("Failed to serialize");

        let (remaining, parsed) =
            RawGeometry::from_bytes(&buffer, &mut parser()).expect("Failed to parse");
        assert!(remaining.is_empty(), "Remaining bytes after parse");

        Geometry::Raw(parsed)
            .into_parsed(&mut dec())
            .expect("Failed to decode")
    }

    /// Build a `GeometryValues` from a sequence of `Geom32` values via
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
        geoms: &[Geom32],
        encoder: GeometryEncoder,
    ) -> (GeometryValues, GeometryValues) {
        let mut pushed = GeometryValues::default();
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

    fn arb_geom() -> impl Strategy<Value = Geom32> {
        prop_oneof![
            // Point
            arb_coord().prop_map(Point).prop_map(Geom32::Point),
            // LineString
            prop::collection::vec(arb_coord(), 2..10)
                .prop_map(|coords| Geom32::LineString(LineString(coords))),
            // Polygon (single exterior ring, no holes)
            prop::collection::vec(arb_coord(), 3..8).prop_map(|mut coords| {
                coords.push(coords[0]);
                Geom32::Polygon(Polygon::new(LineString(coords), vec![]))
            }),
            // MultiPoint
            prop::collection::vec(arb_coord(), 2..8).prop_map(|coords| {
                Geom32::MultiPoint(MultiPoint(coords.into_iter().map(Point).collect()))
            }),
            // MultiLineString
            prop::collection::vec(prop::collection::vec(arb_coord(), 2..6), 2..5,).prop_map(
                |lines| Geom32::MultiLineString(MultiLineString(
                    lines.into_iter().map(LineString).collect(),
                ))
            ),
            // MultiPolygon
            prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                coords.push(coords[0]);
                Geom32::MultiPolygon(MultiPolygon(vec![Polygon::new(LineString(coords), vec![])]))
            }),
        ]
    }

    /// Mixing `LineString` with `MultiLineString`
    fn arb_mixed_linestring_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(arb_geom(), 2..12)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, Geom32::LineString(_) | Geom32::MultiLineString(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both LS and MLS", |geoms| {
                geoms.iter().any(|g| matches!(g, Geom32::LineString(_)))
                    && geoms
                        .iter()
                        .any(|g| matches!(g, Geom32::MultiLineString(_)))
            })
    }

    /// Mixing `Point` with `MultiPoint`
    fn arb_mixed_point_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(arb_geom(), 2..12)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, Geom32::Point(_) | Geom32::MultiPoint(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both P and MP", |geoms| {
                geoms.iter().any(|g| matches!(g, Geom32::Point(_)))
                    && geoms.iter().any(|g| matches!(g, Geom32::MultiPoint(_)))
            })
    }

    /// Mixing `Polygon` with `MultiPolygon`
    fn arb_mixed_polygon_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(arb_geom(), 2..8)
            .prop_map(|geoms| {
                geoms
                    .into_iter()
                    .filter(|g| matches!(g, Geom32::Polygon(_) | Geom32::MultiPolygon(_)))
                    .collect::<Vec<_>>()
            })
            .prop_filter("needs both Poly and MPoly", |geoms| {
                geoms.iter().any(|g| matches!(g, Geom32::Polygon(_)))
                    && geoms.iter().any(|g| matches!(g, Geom32::MultiPolygon(_)))
            })
    }

    /// Mixing `Point` with `MultiLineString`
    fn arb_cross_point_mls_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(
            prop_oneof![
                arb_coord().prop_map(Point).prop_map(Geom32::Point),
                prop::collection::vec(prop::collection::vec(arb_coord(), 2..6), 2..5).prop_map(
                    |lines| {
                        Geom32::MultiLineString(MultiLineString(
                            lines.into_iter().map(LineString).collect(),
                        ))
                    }
                ),
            ],
            2..12,
        )
        .prop_filter("needs both Point and MultiLineString", |geoms| {
            geoms.iter().any(|g| matches!(g, Geom32::Point(_)))
                && geoms
                    .iter()
                    .any(|g| matches!(g, Geom32::MultiLineString(_)))
        })
    }

    /// Mixing `Point` with `MultiPolygon`.
    fn arb_cross_point_mpoly_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(
            prop_oneof![
                arb_coord().prop_map(Point).prop_map(Geom32::Point),
                prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                    coords.push(coords[0]);
                    Geom32::MultiPolygon(MultiPolygon(vec![Polygon::new(
                        LineString(coords),
                        vec![],
                    )]))
                }),
            ],
            2..10,
        )
        .prop_filter("needs both Point and MultiPolygon", |geoms| {
            geoms.iter().any(|g| matches!(g, Geom32::Point(_)))
                && geoms.iter().any(|g| matches!(g, Geom32::MultiPolygon(_)))
        })
    }

    /// Mixing `LineString` with `MultiPolygon`
    fn arb_cross_ls_mpoly_geoms() -> impl Strategy<Value = Vec<Geom32>> {
        prop::collection::vec(
            prop_oneof![
                prop::collection::vec(arb_coord(), 2..8)
                    .prop_map(|coords| Geom32::LineString(LineString(coords))),
                prop::collection::vec(arb_coord(), 3..6).prop_map(|mut coords| {
                    coords.push(coords[0]);
                    Geom32::MultiPolygon(MultiPolygon(vec![Polygon::new(
                        LineString(coords),
                        vec![],
                    )]))
                }),
            ],
            2..10,
        )
        .prop_filter("needs both LineString and MultiPolygon", |geoms| {
            geoms.iter().any(|g| matches!(g, Geom32::LineString(_)))
                && geoms.iter().any(|g| matches!(g, Geom32::MultiPolygon(_)))
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
    /// This ensures `GeometryValues` always holds flat `(x, y)` pairs.
    #[test]
    fn test_morton_vertex_dictionary_expansion() {
        use crate::v01::{
            DictionaryType, EncodedStream, IntEncoder, LengthType, LogicalEncoding, MortonMeta,
            OffsetType, StreamMeta, StreamType,
        };

        // meta: single LineString
        let meta = EncodedStream::encode_u32s_of_type(
            &[GeometryType::LineString as u32],
            IntEncoder::varint(),
            StreamType::Length(LengthType::VarBinary),
        )
        .unwrap();

        // parts: one LineString of length 4
        let parts = EncodedStream::encode_u32s_of_type(
            &[4u32],
            IntEncoder::varint(),
            StreamType::Length(LengthType::Parts),
        )
        .unwrap();

        // vertex offsets: per-vertex indices into the Morton dictionary
        let vertex_offsets_stream = EncodedStream::encode_u32s_of_type(
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
        let morton_dict = EncodedStream {
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
        let owned = EncodedGeometry {
            meta,
            items: vec![parts, vertex_offsets_stream, morton_dict],
        };
        let mut buffer = Vec::new();
        owned.write_to(&mut buffer).unwrap();
        let (remaining, parsed) =
            RawGeometry::from_bytes(&buffer, &mut parser()).unwrap();
        assert!(remaining.is_empty());
        let decoded = Geometry::Raw(parsed)
            .into_parsed(&mut dec())
            .unwrap();

        assert_eq!(decoded.vertices, Some(vec![0i32, 0, 4, 0, 0, 4, 4, 0]));

        let geom = decoded.to_geojson(0).unwrap();
        assert_eq!(geom, wkt!(LINESTRING(0 0,4 0,0 4,4 0)).into());
    }
}
