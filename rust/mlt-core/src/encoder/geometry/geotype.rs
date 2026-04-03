// geo crate is not supported with WASM until next version
#[cfg(feature = "tessellate")]
use geo::{Convert as _, TriangulateEarcut as _};
use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Polygon};

use crate::decoder::{GeometryType, GeometryValues};
use crate::geojson::{Coord32, Geom32};

impl TryFrom<&Geom32> for GeometryType {
    type Error = ();

    fn try_from(geom: &Geom32) -> Result<Self, Self::Error> {
        Ok(match geom {
            Geom32::Point(_) => Self::Point,
            Geom32::MultiPoint(_) => Self::MultiPoint,
            Geom32::LineString(_) => Self::LineString,
            Geom32::MultiLineString(_) => Self::MultiLineString,
            Geom32::Polygon(_) => Self::Polygon,
            Geom32::MultiPolygon(_) => Self::MultiPolygon,
            Geom32::Line(_)
            | Geom32::GeometryCollection(_)
            | Geom32::Rect(_)
            | Geom32::Triangle(_) => {
                return Err(());
            }
        })
    }
}

/// Run the Earcut algorithm on `polygon`, append the remapped triangle indices (shifted by
/// `vertex_offset`) into `index_buf`, and return the number of triangles produced.
///
/// Geo's earcut includes the closing vertex of each ring; MLT (and Java's `earcut4j`) omit it.
/// Any index that would refer to a ring's closing vertex is remapped to that ring's start index,
/// producing identical index buffers to Java.
#[cfg(feature = "tessellate")]
fn earcut_into(polygon: &Polygon<i32>, vertex_offset: u32, index_buf: &mut Vec<u32>) -> u32 {
    let polygon_f64: Polygon<f64> = polygon.convert();
    let raw = polygon_f64.earcut_triangles_raw();
    let num_triangles = u32::try_from(raw.triangle_indices.len() / 3).expect("too many triangles");

    let mut geo_to_mlt = Vec::with_capacity(raw.vertices.len() / 2);
    let mut mlt_offset = 0usize;

    let mut push_ring = |ring: &LineString<i32>| {
        let len = ring.0.len();
        let mlt_len = if len > 1 && ring.0.first() == ring.0.last() {
            len - 1
        } else {
            len
        };
        for i in 0..len {
            geo_to_mlt.push(if i == len - 1 && mlt_len < len {
                mlt_offset
            } else {
                mlt_offset + i
            });
        }
        mlt_offset += mlt_len;
    };

    push_ring(polygon.exterior());
    for interior in polygon.interiors() {
        push_ring(interior);
    }

    for i in raw.triangle_indices {
        let mlt_idx = geo_to_mlt.get(i).copied().unwrap_or(i);
        let base = u32::try_from(mlt_idx).expect("mlt vertex index overflow");
        let idx = base
            .checked_add(vertex_offset)
            .expect("vertex index overflow");
        index_buf.push(idx);
    }

    num_triangles
}

impl GeometryValues {
    /// Returns a [`GeometryValues`] with an empty `triangles` buffer pre-initialized.
    ///
    /// When `triangles` is `Some`, polygon push methods automatically compute and store
    /// Earcut tessellation data as geometries are added.
    /// Use [`Self::default`] when tessellation is not required.
    #[must_use]
    pub fn new_tessellated() -> Self {
        Self {
            triangles: Some(vec![]),
            ..Default::default()
        }
    }

    /// Tessellate `polygon` using the Earcut algorithm and append the results directly into
    /// `self.index_buffer` and `self.triangles`.
    ///
    /// Geo's earcut includes the closing vertex in each ring; MLT (and Java's `earcut4j`) omit it.
    /// Triangle indices that would refer to a ring's closing vertex are remapped to that ring's
    /// start index, producing identical index buffers to Java's `earcut4j`.
    ///
    /// The per-feature triangle count is pushed into `self.triangles`.
    #[cfg(feature = "tessellate")]
    fn tessellate_polygon(&mut self, polygon: &Polygon<i32>) {
        if let Some(triangles) = self.triangles.as_mut() {
            let val = earcut_into(polygon, 0, self.index_buffer.get_or_insert_with(Vec::new));
            triangles.push(val);
        }
    }
    #[cfg(not(feature = "tessellate"))]
    #[allow(unused_variables, clippy::unused_self)]
    fn tessellate_polygon(&mut self, polygon: &Polygon<i32>) {}

    /// Tessellate all polygons in `mp` and append the combined results into
    /// `self.index_buffer` and `self.triangles`.
    ///
    /// Indices for each constituent polygon are offset by the cumulative vertex count of all
    /// preceding polygons so they reference the correct positions in the shared vertex buffer.
    /// A single total triangle count (summed over all constituent polygons) is pushed into
    /// `self.triangles`.
    #[cfg(feature = "tessellate")]
    fn tessellate_multi_polygon(&mut self, mp: &MultiPolygon<i32>) {
        if let Some(triangles) = self.triangles.as_mut() {
            let mut total_triangles = 0u32;
            let mut vertex_offset = 0u32;
            let index_buffer = self.index_buffer.get_or_insert_with(Vec::new);
            for poly in &mp.0 {
                total_triangles += earcut_into(poly, vertex_offset, index_buffer);
                let ext_verts = poly.exterior().0.len().saturating_sub(1);
                let int_verts: usize = poly
                    .interiors()
                    .iter()
                    .map(|r| r.0.len().saturating_sub(1))
                    .sum();
                vertex_offset += u32::try_from(ext_verts + int_verts).expect("vertex overflow");
            }
            triangles.push(total_triangles);
        }
    }

    #[cfg(not(feature = "tessellate"))]
    #[allow(unused_variables, clippy::unused_self)]
    fn tessellate_multi_polygon(&mut self, _mp: &MultiPolygon<i32>) {}

    /// Add a geometry to this decoded geometry collection.
    /// This is the reverse of `to_geojson` - it converts a `Geom32`
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
        self.tessellate_polygon(poly);
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
        self.tessellate_multi_polygon(mp);
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
    use insta::assert_snapshot;
    use proptest::prelude::*;

    use super::*;
    use crate::LazyParsed;
    use crate::decoder::{
        DictionaryType, IntEncoding, LengthType, LogicalEncoding, MortonMeta, OffsetType,
        RawGeometry, StreamMeta, StreamType,
    };
    use crate::encoder::{EncodedStream, Encoder, GeometryEncoder, IntEncoder};
    use crate::geojson::Coord32;
    use crate::test_helpers::{assert_empty, dec, parser};
    use crate::utils::BinarySerializer as _;

    /// Encode, serialize, parse, and decode a `GeometryValues`.
    /// The input must already be in the dense canonical form that `from_encoded`
    /// produces (i.e. built via a previous `roundtrip` call, not via `push_*`).
    fn roundtrip(decoded: &GeometryValues, encoder: GeometryEncoder) -> GeometryValues {
        let mut enc = Encoder::default();
        decoded
            .clone()
            .write_to_with(&mut enc, encoder)
            .expect("Failed to encode");

        let parsed = assert_empty(RawGeometry::from_bytes(&enc.data, &mut parser()));

        LazyParsed::Raw(parsed)
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
        use integer_encoding::VarIntWriter as _;
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
        let items = [parts, vertex_offsets_stream, morton_dict];
        let mut enc = Encoder::default();
        enc.write_varint(u64::try_from(items.len() + 1).unwrap())
            .unwrap();
        enc.write_stream(&meta).unwrap();
        for item in &items {
            enc.write_stream(item).unwrap();
        }
        let buffer = enc.data;

        let mut p = parser();
        let parsed = assert_empty(RawGeometry::from_bytes(&buffer, &mut p));
        assert_snapshot!(p.reserved(), @"72");

        let mut d = dec();
        let decoded = LazyParsed::Raw(parsed).into_parsed(&mut d).unwrap();
        assert_snapshot!(d.consumed(), @"100");
        assert_eq!(decoded.vertices, Some(vec![0i32, 0, 4, 0, 0, 4, 4, 0]));

        let geom = decoded.to_geojson(0).unwrap();
        assert_eq!(geom, wkt!(LINESTRING(0 0,4 0,0 4,4 0)).into());
    }
}

#[cfg(all(test, feature = "tessellate"))]
mod tessellation_tests {
    use geo_types::{LineString, MultiPolygon, Polygon};

    use crate::decoder::GeometryValues;
    use crate::geojson::Geom32;

    #[test]
    fn earcut_closing_vertex_index_remap() {
        let exterior = LineString::from(vec![(0_i32, 0), (10, 0), (10, 10), (0, 10), (0, 0)]);
        let polygon = Polygon::new(exterior, vec![]);
        let mut g = GeometryValues::new_tessellated();
        g.push_geom(&Geom32::Polygon(polygon));
        let tris = g.triangles().expect("triangles");
        let n = tris[0];
        assert!(n > 0, "expected at least one triangle");
        let ib = g.index_buffer().expect("index buffer");
        assert_eq!(ib.len(), usize::try_from(n).unwrap() * 3);
        // 4 unique (non-closing) vertices → indices in 0..4
        assert!(ib.iter().all(|&i| i < 4));
    }

    #[test]
    fn earcut_vertex_offset_for_multi_polygon_parts() {
        let exterior1 = LineString::from(vec![(0_i32, 0), (10, 0), (10, 10), (0, 10), (0, 0)]);
        let poly1 = Polygon::new(exterior1, vec![]);
        let exterior2 = LineString::from(vec![(20, 0), (30, 0), (30, 10), (20, 10), (20, 0)]);
        let poly2 = Polygon::new(exterior2, vec![]);
        let mut g = GeometryValues::new_tessellated();
        g.push_geom(&Geom32::MultiPolygon(MultiPolygon(vec![poly1, poly2])));
        let ib = g.index_buffer().expect("index buffer");
        let tris = g.triangles().expect("triangles");
        assert_eq!(tris.len(), 1);
        let total = usize::try_from(tris[0]).unwrap();
        assert_eq!(ib.len(), total * 3);
        // First quad: 4 verts → 2 triangles, 6 indices
        let split = 6;
        let (first, second) = ib.split_at(split);
        assert!(
            first.iter().all(|&i| i < 4),
            "first polygon indices should reference verts 0..4: {first:?}"
        );
        assert!(
            second.iter().all(|&i| (4..8).contains(&i)),
            "second polygon indices should reference verts 4..8: {second:?}"
        );
    }
}
