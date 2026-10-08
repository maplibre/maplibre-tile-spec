//! Feature reordering for the optimizer

use crate::codecs::hilbert::{hilbert_curve_params_from_bounds, hilbert_sort_key};
use crate::codecs::morton::morton_sort_key;
use crate::encoder::model::CurveParams;
use crate::encoder::source::{LayerSource, Order};
use crate::tile::TileLayer;

/// Controls how features inside a layer are reordered before encoding.
///
/// Reordering features changes their position in every parallel column
/// (geometry, ID, and all properties simultaneously), so the caller must
/// opt in explicitly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::EnumIter, strum::EnumCount)]
pub enum SortStrategy {
    /// Preserve the original feature order - no reordering is applied.
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

impl TileLayer {
    /// Reorder features by `strategy`, using `params` as the curve normalization
    /// for [`SortStrategy::SpatialMorton`] / [`SortStrategy::SpatialHilbert`].
    ///
    /// `params` is taken as a parameter (rather than recomputed here) so the
    /// same scan feeds the encoder's dictionary builders, see
    /// [`TileLayer::curve_params`].
    ///
    /// [`SortStrategy::Unsorted`] is a no-op; layers with ≤1 feature are
    /// trivially unchanged.
    #[hotpath::measure]
    pub fn sort(&mut self, strategy: SortStrategy, params: CurveParams) {
        if strategy == SortStrategy::Unsorted {
            return;
        }
        let order = sort_order(self, strategy, params);
        let mut features: Vec<_> = std::mem::take(&mut self.features)
            .into_iter()
            .map(Some)
            .collect();
        self.features = order
            .iter()
            .map(|f| features[f].take().expect("each feature appears once"))
            .collect();
    }

    /// Hilbert/Morton `CurveParams` for this layer.
    /// Bounds are order-invariant, so the optimizer computes this once per layer
    /// and reuses it across every sort trial and the encoder's dictionary builders.
    #[must_use]
    pub fn curve_params(&self) -> CurveParams {
        curve_params(self)
    }
}

/// The order `strategy` puts the features of `source` in.
#[hotpath::measure]
pub(crate) fn sort_order(
    source: &impl LayerSource,
    strategy: SortStrategy,
    params: CurveParams,
) -> Order {
    let len = source.feature_count();
    match strategy {
        SortStrategy::Unsorted => Order::Stored(len),
        SortStrategy::SpatialMorton | SortStrategy::SpatialHilbert => {
            let curve_key = if strategy == SortStrategy::SpatialMorton {
                morton_sort_key
            } else {
                hilbert_sort_key
            };
            Order::Permuted(permutation(len, |f| {
                source
                    .first_coord(f)
                    .map_or(u64::MAX, |c| u64::from(curve_key(c, params)))
            }))
        }
        SortStrategy::Id => Order::Permuted(permutation(len, |f| source.id(f))),
    }
}

/// Feature indexes ordered by `key`, with ties in stored order, as a stable sort leaves them.
fn permutation<K: Ord>(len: usize, key: impl Fn(usize) -> K) -> Vec<u32> {
    let mut keyed: Vec<(K, u32)> = (0..len)
        .map(|f| (key(f), u32::try_from(f).expect("feature count fits in u32")))
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, f)| f).collect()
}

/// Hilbert/Morton `CurveParams` covering every coordinate of `source`.
#[hotpath::measure]
pub(crate) fn curve_params(source: &impl LayerSource) -> CurveParams {
    let (min_val, max_val) = source.coords().fold((i32::MAX, i32::MIN), |(min, max), c| {
        (min.min(c.x).min(c.y), max.max(c.x).max(c.y))
    });
    hilbert_curve_params_from_bounds(min_val, max_val)
}

#[cfg(test)]
mod tests {
    use geo_types::{
        Coord, Geometry as GeoGeom, Geometry, GeometryCollection, Line, LineString,
        MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, Rect, Triangle,
    };
    use rstest::rstest;

    use crate::decoder::{GeometryType, GeometryValues, RawGeometry};
    use crate::encoder::model::CurveParams;
    use crate::encoder::{
        Codecs, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder, SortStrategy, stage_tile,
    };
    use crate::test_helpers::{assert_empty, dec, into_layer01, parser};
    use crate::tile::{TileFeature, TileLayer};
    use crate::{Layer, LazyParsed};

    fn pt(x: i32, y: i32) -> Geometry<i32> {
        GeoGeom::Point(Point::new(x, y))
    }

    fn ls(coords: &[(i32, i32)]) -> Geometry<i32> {
        GeoGeom::LineString(LineString::new(
            coords.iter().map(|&(x, y)| Coord { x, y }).collect(),
        ))
    }

    fn poly_square(x0: i32, y0: i32, side: i32) -> Geometry<i32> {
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
        let mut codecs = Codecs::default();
        decoded
            .clone()
            .write_to(&mut enc, &mut codecs)
            .expect("encode failed");
        let buf = enc.data().to_vec();

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
    fn canonical(geoms: &[Geometry<i32>]) -> GeometryValues {
        let mut decoded = GeometryValues::default();
        for g in geoms {
            decoded.push_geom(g);
        }
        roundtrip_geom(&decoded)
    }

    /// Build a `TileLayer` from `geoms` and `ids`, apply `reorder_features`,
    /// and return it.
    fn layer_after_sort(geoms: &[Geometry<i32>], ids: &[u64], strategy: SortStrategy) -> TileLayer {
        let features: Vec<TileFeature> = geoms
            .iter()
            .zip(ids.iter())
            .map(|(g, &id)| TileFeature {
                id: Some(id),
                geometry: g.clone(),
                properties: vec![],
                #[cfg(feature = "unstable-v2")]
                m_values: vec![],
                #[cfg(feature = "unstable-v2")]
                nested: vec![],
                #[cfg(feature = "unstable-v2")]
                z: Vec::new(),
            })
            .collect();

        let mut layer = TileLayer::from_parts("test", 4096, vec![], features).unwrap();

        let params = layer.curve_params();
        layer.sort(strategy, params);
        layer
    }

    /// Sort, then encode+decode the result and compare to `canonical(expected)`.
    fn assert_sort_roundtrip(
        geoms: &[Geometry<i32>],
        ids: &[u64],
        strategy: SortStrategy,
        expected: &[Geometry<i32>],
    ) {
        let layer = layer_after_sort(geoms, ids, strategy);

        let mut sorted_decoded = GeometryValues::default();
        for f in layer.features() {
            sorted_decoded.push_geom(f.geometry());
        }

        let after_roundtrip = roundtrip_geom(&sorted_decoded);
        let expected_canonical = canonical(expected);

        assert_eq!(
            after_roundtrip, expected_canonical,
            "\nsorted geometry did not match expected after encode->decode round-trip\
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

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(1u64), Some(2), Some(3)]);

        // Verify geometry types match expected order
        let geom_types: Vec<&str> = layer
            .features()
            .iter()
            .map(|f| GeometryType::try_from(f.geometry()).unwrap().into())
            .collect();
        assert_eq!(geom_types, vec!["LineString", "Point", "Point"]);
    }

    /// Build row-oriented tile layer from geometries and IDs (one feature per geometry).
    fn build_tile_layer(geoms: &[Geometry<i32>], ids: &[Option<u64>]) -> TileLayer {
        assert_eq!(geoms.len(), ids.len());
        TileLayer::from_parts(
            "test",
            4096,
            vec![],
            geoms
                .iter()
                .zip(ids.iter())
                .map(|(g, &id)| TileFeature {
                    id,
                    geometry: g.clone(),
                    properties: vec![],
                    #[cfg(feature = "unstable-v2")]
                    m_values: vec![],
                    #[cfg(feature = "unstable-v2")]
                    nested: vec![],
                    #[cfg(feature = "unstable-v2")]
                    z: Vec::new(),
                })
                .collect(),
        )
        .unwrap()
    }

    /// Encode the layer with a given sort strategy, decode it back, and return the `TileLayer`.
    /// This tests the full encode->decode roundtrip, verifying that sorting was applied.
    fn sort_encode_decode(tile: &TileLayer, sort: SortStrategy) -> TileLayer {
        let enc_cfg = EncoderConfig::default();
        let enc = Encoder::with_explicit(enc_cfg, ExplicitEncoder::for_id(IntEncoder::varint()));
        let mut codecs = Codecs::default();
        let enc = stage_tile(tile, sort, false, enc_cfg.tessellate())
            .encode_into(enc, &mut codecs)
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

    fn encode_decode(tile: &TileLayer, cfg: EncoderConfig) -> TileLayer {
        let buf = tile.encode(cfg).expect("encode failed");
        let layer = assert_empty(Layer::from_bytes(&buf, &mut parser()));
        into_layer01(layer)
            .into_tile(&mut dec())
            .expect("decode failed")
    }

    fn scattered_points(count: u32) -> Vec<Geometry<i32>> {
        (0..count)
            .map(|i| {
                let x = i32::try_from(i.wrapping_mul(2_654_435_761) % 4096).unwrap();
                let y = i32::try_from(i.wrapping_mul(40_503) % 4096).unwrap();
                pt(x, y)
            })
            .collect()
    }

    #[test]
    fn default_config_keeps_the_source_order() {
        let geoms = scattered_points(600);
        let ids: Vec<Option<u64>> = (0..600).rev().map(Some).collect();
        let decoded = encode_decode(&build_tile_layer(&geoms, &ids), EncoderConfig::default());

        let decoded_ids: Vec<Option<u64>> =
            decoded.features().iter().map(TileFeature::id).collect();
        assert_eq!(decoded_ids, ids);
    }

    #[test]
    fn hilbert_sort_reaches_a_layer_spread_over_the_whole_extent() {
        let geoms = scattered_points(600);
        let ids = vec![None; geoms.len()];
        let mut expected = build_tile_layer(&geoms, &ids);
        let params = expected.curve_params();
        expected.sort(SortStrategy::SpatialHilbert, params);

        let decoded = encode_decode(
            &build_tile_layer(&geoms, &ids),
            EncoderConfig::default().with_spatial_hilbert_sort(true),
        );

        assert_eq!(
            vertices_from_source(&decoded),
            vertices_from_source(&expected)
        );
    }

    /// Rebuild a flat vertex buffer from the feature geometries in source order.
    fn vertices_from_source(source: &TileLayer) -> Vec<i32> {
        let mut geom = GeometryValues::default();
        for f in source.features() {
            geom.push_geom(f.geometry());
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
        let source = sort_encode_decode(&tile, SortStrategy::SpatialMorton);

        let verts = vertices_from_source(&source);
        assert_eq!(verts, vec![0, -10, -10, 0]);
    }

    #[test]
    fn test_id_sort_nulls_first() {
        let tile = build_tile_layer(&[pt(2, 2), pt(1, 1), pt(0, 0)], &[Some(10), None, Some(5)]);
        let source = sort_encode_decode(&tile, SortStrategy::Id);

        let ids: Vec<Option<u64>> = source.features().iter().map(TileFeature::id).collect();
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
        let source = sort_encode_decode(&tile, SortStrategy::SpatialMorton);

        let types: Vec<_> = source
            .features()
            .iter()
            .map(|f| GeometryType::try_from(f.geometry()).unwrap())
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

    #[test]
    fn id_sort_separates_the_two_largest_ids() {
        let layer = layer_after_sort(
            &[pt(0, 0), pt(1, 1)],
            &[u64::MAX, u64::MAX - 1],
            SortStrategy::Id,
        );

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![Some(u64::MAX - 1), Some(u64::MAX)]);
    }

    #[test]
    fn id_sort_puts_the_missing_id_before_zero() {
        let layer = build_tile_layer(&[pt(0, 0), pt(1, 1)], &[Some(0), None]);
        let mut layer = layer;
        layer.sort(SortStrategy::Id, CurveParams { shift: 0, bits: 1 });

        let ids: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(ids, vec![None, Some(0)]);
    }

    #[test]
    fn spatial_sort_reads_the_first_vertex_of_every_geometry_kind() {
        let at = |k: i32| Coord { x: k, y: k };
        let geoms = [
            GeoGeom::Rect(Rect::new(at(0), at(20))),
            GeoGeom::Triangle(Triangle::new(at(1), at(11), at(21))),
            GeoGeom::MultiPolygon(MultiPolygon(vec![Polygon::new(
                LineString(vec![at(2), at(12), at(22), at(2)]),
                vec![],
            )])),
            GeoGeom::MultiLineString(MultiLineString(vec![LineString(vec![at(3), at(13)])])),
            GeoGeom::MultiPoint(MultiPoint(vec![Point::from(at(4)), Point::from(at(14))])),
            GeoGeom::Polygon(Polygon::new(
                LineString(vec![at(5), at(15), at(25), at(5)]),
                vec![],
            )),
            GeoGeom::LineString(LineString(vec![at(6), at(16)])),
            GeoGeom::Line(Line::new(at(7), at(17))),
            GeoGeom::Point(Point::from(at(8))),
            GeoGeom::GeometryCollection(GeometryCollection(vec![GeoGeom::Point(Point::from(at(
                9,
            )))])),
            GeoGeom::LineString(LineString(vec![])),
        ];
        let ids: Vec<Option<u64>> = (0..u64::try_from(geoms.len()).unwrap()).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        // Morton keys interleave the bits of x and y
        // This means the diagonal they rise with the coordinate and the sorted order is the vertex order.
        layer.sort(SortStrategy::SpatialMorton, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(sorted, ids);
    }

    #[test]
    fn curve_params_of_a_vertexless_layer_is_the_degenerate_grid() {
        let layer = build_tile_layer(&[GeoGeom::LineString(LineString(vec![]))], &[None]);
        assert_eq!(layer.curve_params(), CurveParams { shift: 0, bits: 1 });
    }

    #[rstest]
    #[case::morton(SortStrategy::SpatialMorton, &[0, 1, 2, 3])]
    #[case::hilbert(SortStrategy::SpatialHilbert, &[0, 2, 3, 1])]
    fn spatial_sort_of_the_unit_square_follows_the_chosen_curve(
        #[case] strategy: SortStrategy,
        #[case] expected: &[u64],
    ) {
        let geoms = [pt(0, 0), pt(1, 0), pt(0, 1), pt(1, 1)];
        let ids: Vec<Option<u64>> = (0..4).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        assert_eq!(params, CurveParams { shift: 0, bits: 1 });
        layer.sort(strategy, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(
            sorted,
            expected.iter().copied().map(Some).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unsorted_leaves_the_feature_order_alone() {
        let geoms = [pt(9, 9), pt(0, 0), pt(5, 5)];
        let ids = [Some(3u64), Some(1), Some(2)];

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        layer.sort(SortStrategy::Unsorted, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(sorted, ids.to_vec());
    }

    #[test]
    fn every_vertexless_geometry_kind_sorts_after_the_one_feature_with_a_vertex() {
        let empty_ring = || Polygon::new(LineString(vec![]), vec![]);
        let geoms = [
            GeoGeom::LineString(LineString(vec![])),
            GeoGeom::Polygon(empty_ring()),
            GeoGeom::MultiPoint(MultiPoint(vec![])),
            GeoGeom::MultiLineString(MultiLineString(vec![])),
            GeoGeom::MultiLineString(MultiLineString(vec![LineString(vec![])])),
            GeoGeom::MultiPolygon(MultiPolygon(vec![])),
            GeoGeom::MultiPolygon(MultiPolygon(vec![empty_ring()])),
            GeoGeom::GeometryCollection(GeometryCollection(vec![])),
            GeoGeom::GeometryCollection(GeometryCollection(vec![GeoGeom::LineString(LineString(
                vec![],
            ))])),
            pt(1, 1),
        ];
        let ids: Vec<Option<u64>> = (0..u64::try_from(geoms.len()).unwrap()).map(Some).collect();

        let mut layer = build_tile_layer(&geoms, &ids);
        let params = layer.curve_params();
        layer.sort(SortStrategy::SpatialMorton, params);

        let sorted: Vec<Option<u64>> = layer.features().iter().map(TileFeature::id).collect();
        assert_eq!(
            sorted,
            vec![
                Some(9),
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8)
            ]
        );
    }
}
