//! Feature reordering for the optimizer

use geo::CoordsIter as _;

use crate::codecs::hilbert::{hilbert_curve_params_from_bounds, hilbert_sort_key};
use crate::codecs::morton::morton_sort_key;
use crate::decoder::{TileFeature, TileLayer01};
use crate::{Coord32, Geom32};

/// Controls how features inside a layer are reordered before encoding.
///
/// Reordering features changes their position in every parallel column
/// (geometry, ID, and all properties simultaneously), so the caller must
/// opt in explicitly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::EnumIter, strum::EnumCount)]
pub enum SortStrategy {
    /// Preserve the original feature order — no reordering is applied.
    ///
    /// This is the default.
    #[default]
    Unsorted,

    /// Sort features by the Z-order (Morton) curve index of their first vertex.
    ///
    /// Fast to compute.  Spatially close features end up adjacent in the
    /// stream, improving RLE run lengths for location-correlated properties
    /// and CPU cache locality during client-side decoding.
    ///
    SpatialMorton,

    /// Sort features by the Hilbert curve index of their first vertex.
    ///
    /// Slower to compute than Morton but achieves superior spatial locality.
    SpatialHilbert,

    /// Sort features by their feature ID in ascending order.
    Id,
}

impl TileLayer01 {
    /// Reorder all features of `layer` according to `strategy`.
    ///
    /// [`SortStrategy::Unsorted`] is a no-op.
    /// Layers with zero or one features are trivially unchanged by any sort.
    #[hotpath::measure]
    pub fn sort(&mut self, strategy: SortStrategy) {
        match strategy {
            SortStrategy::SpatialMorton | SortStrategy::SpatialHilbert => {
                let params = curve_params_from_features(&self.features);
                let curve_key = if let SortStrategy::SpatialMorton = strategy {
                    morton_sort_key
                } else {
                    hilbert_sort_key
                };
                self.features.sort_by_cached_key(|f| {
                    first_vertex(&f.geometry).map_or(u64::MAX, |c| {
                        u64::from(curve_key(c, params.shift, params.bits))
                    })
                });
            }
            SortStrategy::Id => {
                self.features
                    .sort_by_cached_key(|f| f.id.map_or(0, |v| v.saturating_add(1)));
            }
            SortStrategy::Unsorted => {
                // do nothing
            }
        }
    }
}

/// Parameters derived from the vertex set of a feature collection, used to
/// normalize coordinates before space-filling-curve key computation.
struct CurveParams {
    shift: u32,
    bits: u32,
}

/// Compute the Hilbert/Morton curve parameters from all vertex coordinates
/// in `features` without allocating a temporary vertex buffer.
fn curve_params_from_features(features: &[TileFeature]) -> CurveParams {
    let (min_val, max_val) = features
        .iter()
        .flat_map(|f| f.geometry.coords_iter())
        .fold((i32::MAX, i32::MIN), |(min, max), c| {
            (min.min(c.x).min(c.y), max.max(c.x).max(c.y))
        });
    let (shift, bits) = hilbert_curve_params_from_bounds(min_val, max_val);
    CurveParams { shift, bits }
}

/// Extract the coordinate of the first vertex of a geometry.
fn first_vertex(geom: &Geom32) -> Option<Coord32> {
    match geom {
        Geom32::Point(p) => Some(p.0),
        Geom32::Line(l) => Some(l.start),
        Geom32::LineString(ls) => ls.0.first().copied(),
        Geom32::Polygon(p) => p.exterior().0.first().copied(),
        Geom32::MultiPoint(mp) => mp.0.first().map(|p| p.0),
        Geom32::MultiLineString(mls) => mls.0.first().and_then(|ls| ls.0.first().copied()),
        Geom32::MultiPolygon(mp) => mp.0.first().and_then(|p| p.exterior().0.first().copied()),
        Geom32::Triangle(t) => Some(t.v1()),
        Geom32::Rect(r) => Some(r.min()),
        Geom32::GeometryCollection(gc) => gc.0.first().and_then(first_vertex),
    }
}

/// Return `true` if a spatial sort is likely to reduce compressed size.
///
/// The heuristic: if the vertex bounding box spans more than
/// `SPATIAL_HELP_COVERAGE` of the layer's tile extent on **both** axes, the
/// features are too spread-out for locality clustering to help, so spatial
/// sorting is skipped.
pub(crate) fn spatial_sort_likely_to_help(layer: &TileLayer01) -> bool {
    const SPATIAL_HELP_COVERAGE: f64 = 0.8;

    let extent = f64::from(layer.extent);
    if extent <= 0.0 || layer.features.is_empty() {
        return true;
    }

    let (min_x, max_x, min_y, max_y) = layer
        .features
        .iter()
        .filter_map(|f| first_vertex(&f.geometry))
        .fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(min_x, max_x, min_y, max_y), Coord32 { x, y }| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );

    if min_x > max_x || min_y > max_y {
        return true;
    }

    let range_x = f64::from(max_x - min_x);
    let range_y = f64::from(max_y - min_y);

    let spread_x = range_x / extent;
    let spread_y = range_y / extent;

    !(spread_x > SPATIAL_HELP_COVERAGE && spread_y > SPATIAL_HELP_COVERAGE)
}

#[cfg(test)]
mod tests {
    use geo_types::{Coord, Geometry as GeoGeom, LineString, Point, Polygon};

    use crate::decoder::{GeometryType, GeometryValues, RawGeometry, TileFeature, TileLayer01};
    use crate::encoder::{
        Encoder, ExplicitEncoder, IdWidth, IntEncoder, SortStrategy, StagedLayer01,
    };
    use crate::test_helpers::{assert_empty, dec, into_layer01, parser};
    use crate::{Geom32, Layer, LazyParsed};

    // ── geometry test helpers ──────────────────────────────────────────────────

    fn pt(x: i32, y: i32) -> Geom32 {
        GeoGeom::Point(Point::new(x, y))
    }

    fn ls(coords: &[(i32, i32)]) -> Geom32 {
        GeoGeom::LineString(LineString::new(
            coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
        ))
    }

    fn poly_square(x0: i32, y0: i32, side: i32) -> Geom32 {
        let ring = LineString::new(vec![
            Coord { x: x0, y: y0 },
            Coord {
                x: x0 + side,
                y: y0,
            },
            Coord {
                x: x0 + side,
                y: y0 + side,
            },
            Coord {
                x: x0,
                y: y0 + side,
            },
            Coord { x: x0, y: y0 },
        ]);
        GeoGeom::Polygon(Polygon::new(ring, vec![]))
    }

    /// Encode + serialize + parse + decode a `GeometryValues` (round-trip).
    fn roundtrip_geom(decoded: &GeometryValues) -> GeometryValues {
        let mut enc = Encoder::default();
        decoded.clone().write_to(&mut enc).expect("encode failed");
        let buf = enc.data;

        let parsed = assert_empty(RawGeometry::from_bytes(&buf, &mut parser()));
        let mut d = dec();
        let result = LazyParsed::Raw(parsed)
            .into_parsed(&mut d)
            .expect("decode failed");
        assert!(
            d.consumed() > 0,
            "decoder should consume bytes after decode"
        );
        result
    }

    /// Build the canonical (dense, wire-decoded) form of an ordered geometry sequence.
    fn canonical(geoms: &[Geom32]) -> GeometryValues {
        let mut decoded = GeometryValues::default();
        for g in geoms {
            decoded.push_geom(g);
        }
        roundtrip_geom(&decoded)
    }

    /// Build a `TileLayer01` from `geoms` and `ids`, apply `reorder_features`,
    /// and return it.
    fn layer_after_sort(geoms: &[Geom32], ids: &[u64], strategy: SortStrategy) -> TileLayer01 {
        let features: Vec<TileFeature> = geoms
            .iter()
            .zip(ids.iter())
            .map(|(g, &id)| TileFeature {
                id: Some(id),
                geometry: g.clone(),
                properties: vec![],
            })
            .collect();

        let mut layer = TileLayer01 {
            name: "test".to_string(),
            extent: 4096,
            property_names: vec![],
            features,
        };

        layer.sort(strategy);
        layer
    }

    /// Sort, then encode+decode the result and compare to `canonical(expected)`.
    fn assert_sort_roundtrip(
        geoms: &[Geom32],
        ids: &[u64],
        strategy: SortStrategy,
        expected: &[Geom32],
    ) {
        let layer = layer_after_sort(geoms, ids, strategy);

        let mut sorted_decoded = GeometryValues::default();
        for f in &layer.features {
            sorted_decoded.push_geom(&f.geometry);
        }

        let after_roundtrip = roundtrip_geom(&sorted_decoded);
        let expected_canonical = canonical(expected);

        assert_eq!(
            after_roundtrip, expected_canonical,
            "\nsorted geometry did not match expected after encode→decode round-trip\
             \nvector_types after sort: {:?}\
             \nvector_types expected:   {:?}",
            sorted_decoded.vector_types, expected_canonical.vector_types,
        );
    }

    // ── pure Points ──────────────────────────────────────────────────────────

    #[test]
    fn pure_points_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), pt(1, 1), pt(2, 2)],
            &[3, 2, 1],
            SortStrategy::Id,
            &[pt(2, 2), pt(1, 1), pt(0, 0)],
        );
    }

    // ── pure LineStrings ─────────────────────────────────────────────────────

    #[test]
    fn pure_linestrings_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[ls(&[(0, 0), (0, 10)]), ls(&[(5, 5), (10, 10)])],
            &[2, 1],
            SortStrategy::Id,
            &[ls(&[(5, 5), (10, 10)]), ls(&[(0, 0), (0, 10)])],
        );
    }

    // ── [Point, LineString, Point] ────────────────────────────────────────────

    #[test]
    fn point_line_point_id_sort_to_line_point_point_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
            &[ls(&[(1, 0), (1, 5)]), pt(5, 5), pt(0, 0)],
        );
    }

    #[test]
    fn point_line_point_id_sort_to_point_point_line_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[1, 3, 2],
            SortStrategy::Id,
            &[pt(0, 0), pt(5, 5), ls(&[(1, 0), (1, 5)])],
        );
    }

    // ── [Point, Polygon, Point] ───────────────────────────────────────────────

    #[test]
    fn point_polygon_point_id_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(0, 0), poly_square(10, 10, 5), pt(5, 5)],
            &[2, 1, 3],
            SortStrategy::Id,
            &[poly_square(10, 10, 5), pt(0, 0), pt(5, 5)],
        );
    }

    // ── spatial Morton sort ───────────────────────────────────────────────────

    #[test]
    fn point_line_point_morton_sort_roundtrip() {
        assert_sort_roundtrip(
            &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
            &[1, 2, 3],
            SortStrategy::SpatialMorton,
            &[ls(&[(0, 0), (0, 5)]), pt(1, 0), pt(2, 0)],
        );
    }

    // ── already-sorted is identity ────────────────────────────────────────────

    #[test]
    fn id_sort_already_sorted_is_identity_roundtrip() {
        let geoms = &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)];
        assert_sort_roundtrip(geoms, &[1, 2, 3], SortStrategy::Id, geoms);
    }

    // ── ID column co-permuted with geometry ───────────────────────────────────

    #[test]
    fn id_column_co_permuted_with_geometry() {
        let layer = layer_after_sort(
            &[pt(0, 0), ls(&[(1, 0), (1, 5)]), pt(5, 5)],
            &[3, 1, 2],
            SortStrategy::Id,
        );

        let ids: Vec<Option<u64>> = layer.features.iter().map(|f| f.id).collect();
        assert_eq!(ids, vec![Some(1u64), Some(2), Some(3)]);

        // Verify geometry types match expected order
        let geom_types: Vec<&str> = layer
            .features
            .iter()
            .map(|f| GeometryType::try_from(&f.geometry).unwrap().into())
            .collect();
        assert_eq!(geom_types, vec!["LineString", "Point", "Point"]);
    }

    /// Build row-oriented tile layer from geometries and IDs (one feature per geometry).
    fn build_tile_layer(geoms: &[Geom32], ids: &[Option<u64>]) -> TileLayer01 {
        assert_eq!(geoms.len(), ids.len());
        TileLayer01 {
            name: "test".to_string(),
            extent: 4096,
            property_names: vec![],
            features: geoms
                .iter()
                .zip(ids.iter())
                .map(|(g, &id)| TileFeature {
                    id,
                    geometry: g.clone(),
                    properties: vec![],
                })
                .collect(),
        }
    }

    /// Encode the layer with a given sort strategy, decode it back, and return the `TileLayer01`.
    /// This tests the full encode→decode roundtrip, verifying that sorting was applied.
    fn sort_encode_decode(tile: TileLayer01, sort: SortStrategy) -> TileLayer01 {
        let enc_cfg = Encoder::default().cfg;
        let enc = Encoder::with_explicit(
            enc_cfg,
            ExplicitEncoder::for_id(IntEncoder::varint(), IdWidth::Id32),
        );
        let enc = StagedLayer01::from_tile(tile, sort, &[], enc_cfg.tessellate)
            .encode_into(enc)
            .expect("encode failed");

        // Serialize to bytes and reparse to get a `Layer01`.
        let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");

        let mut p = parser();
        let layer_back = assert_empty(Layer::from_bytes(&buf, &mut p));
        assert!(p.reserved() > 0, "parser should reserve bytes after parse");

        let layer01 = into_layer01(layer_back);

        let mut d = dec();
        let tile = layer01.into_tile(&mut d).expect("decode after sort failed");
        assert!(
            d.consumed() > 0,
            "decoder should consume bytes after decode"
        );
        tile
    }

    /// Rebuild a flat vertex buffer from the feature geometries in source order.
    fn vertices_from_source(source: &TileLayer01) -> Vec<i32> {
        let mut geom = GeometryValues::default();
        for f in &source.features {
            geom.push_geom(&f.geometry);
        }
        geom.vertices().unwrap_or_default().to_vec()
    }

    #[test]
    fn test_shared_morton_shift() {
        // P1 at (0, -10), P2 at (-10, 0).
        // With shared shift = 10:
        // P1 shifted: (10, 0) -> interleave(10, 0) = 68
        // P2 shifted: (0, 10) -> interleave(0, 10) = 136
        // P1 (key 68) < P2 (key 136), so expected order: [P1(0,-10), P2(-10,0)].

        let tile = build_tile_layer(&[pt(0, -10), pt(-10, 0)], &[Some(1), Some(2)]);
        let source = sort_encode_decode(tile, SortStrategy::SpatialMorton);

        let verts = vertices_from_source(&source);
        assert_eq!(verts, vec![0, -10, -10, 0]);
    }

    #[test]
    fn test_id_sort_nulls_first() {
        let tile = build_tile_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
        let source = sort_encode_decode(tile, SortStrategy::Id);

        let ids: Vec<Option<u64>> = source.features.iter().map(|f| f.id).collect();
        // Expected order: [None, Some(5), Some(10)]
        assert_eq!(ids, vec![None, Some(5), Some(10)]);

        let verts = vertices_from_source(&source);
        // Corresponding verts: [pt(1,1), pt(0,0), pt(2,2)] -> [1,1, 0,0, 2,2]
        assert_eq!(verts, vec![1, 1, 0, 0, 2, 2]);
    }

    #[test]
    fn test_mixed_geometry_morton_sort() {
        // [Point(2,0), LineString(0,0 -> 0,5), Point(1,0)]
        // Morton keys (assuming shift 0):
        // P1(2,0) -> 4
        // LS(0,0) -> 0
        // P2(1,0) -> 1
        // Expected order: [LS, P2, P1]

        let tile = build_tile_layer(
            &[pt(2, 0), ls(&[(0, 0), (0, 5)]), pt(1, 0)],
            &[Some(1), Some(2), Some(3)],
        );
        let source = sort_encode_decode(tile, SortStrategy::SpatialMorton);

        let types: Vec<_> = source
            .features
            .iter()
            .map(|f| GeometryType::try_from(&f.geometry).unwrap())
            .collect();

        assert_eq!(
            types,
            vec![
                GeometryType::LineString,
                GeometryType::Point,
                GeometryType::Point
            ]
        );

        let verts = vertices_from_source(&source);
        // Expected vertices: LS(0,0,0,5), P2(1,0), P1(2,0)
        assert_eq!(verts, vec![0, 0, 0, 5, 1, 0, 2, 0]);
    }
}
