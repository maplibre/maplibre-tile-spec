//! Round-trip and differential tests for the experimental v2 (tag `0x02`) wire format.
//!
//! Every case encodes the same [`TileLayer`] as both v1 and v2, decodes both,
//! and asserts the decoded layers are identical. This exercises the full v2
//! pipeline (envelope, geometry layouts, presence bitfields, stream headers,
//! interleaved RLE) against the v1 implementation as the reference.

use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{Decoder, Layer, Parser, PropValue, TileFeature, TileLayer};

// ── Helpers ────────────────────────────────────────────────────────────────

/// Deterministic feature order so v1 and v2 outputs are directly comparable
/// (sort-strategy trials could otherwise pick different winners per format).
fn cfg_v1() -> EncoderConfig {
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
}

fn cfg_v2() -> EncoderConfig {
    cfg_v1().with_wire_version(WireVersion::V02)
}

/// Parse a single-layer tile, returning the wire tag and the decoded layer.
fn decode(bytes: &[u8]) -> (u8, TileLayer) {
    let mut parser = Parser::default();
    let layers = parser.parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1, "expected a single layer");
    let tag = match &layers[0] {
        Layer::Tag01(_) => 1,
        Layer::Tag02(_) => 2,
        _ => panic!("unexpected layer kind"),
    };
    let layer = layers
        .into_iter()
        .next()
        .unwrap()
        .into_layer01()
        .expect("layer01 representation");
    let mut dec = Decoder::default();
    (tag, layer.into_tile(&mut dec).expect("into_tile"))
}

/// Encode `layer` as v1 and v2, assert both decode to the identical
/// [`TileLayer`], and return the encoded sizes `(v1_len, v2_len)`.
fn assert_differential(layer: &TileLayer) -> (usize, usize) {
    let v1_bytes = layer.clone().encode(cfg_v1()).expect("v1 encode");
    let v2_bytes = layer.clone().encode(cfg_v2()).expect("v2 encode");
    let (tag1, tile1) = decode(&v1_bytes);
    let (tag2, tile2) = decode(&v2_bytes);
    assert_eq!(tag1, 1);
    assert_eq!(tag2, 2);
    assert_eq!(tile1, tile2, "v1 and v2 decoded layers must be identical");
    (v1_bytes.len(), v2_bytes.len())
}

/// Build a layer from geometries, optional per-feature IDs, and property columns.
#[expect(clippy::needless_pass_by_value, reason = "test ergonomics")]
fn layer(
    geoms: Vec<Geometry<i32>>,
    ids: Option<Vec<Option<u64>>>,
    props: &[(&str, Vec<PropValue>)],
) -> TileLayer {
    let mut builder = TileLayer::builder("test_layer", 4096).unwrap();
    let keys: Vec<_> = props
        .iter()
        .map(|(name, values)| builder.add_property(*name, values[0].kind()).unwrap())
        .collect();
    for (i, geom) in geoms.into_iter().enumerate() {
        let mut feature = builder.feature(geom);
        if let Some(ids) = &ids {
            feature.id(ids[i]);
        }
        for (key, (_, values)) in keys.iter().zip(props) {
            feature.property(*key, values[i].clone()).unwrap();
        }
        feature.finish().unwrap();
    }
    builder.finish()
}

fn pt(x: i32, y: i32) -> Geometry<i32> {
    Geometry::Point(Point::new(x, y))
}

fn coords(pts: &[(i32, i32)]) -> Vec<Coord<i32>> {
    pts.iter().map(|&(x, y)| Coord { x, y }).collect()
}

fn line(pts: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(LineString::new(coords(pts)))
}

fn ring(pts: &[(i32, i32)]) -> LineString<i32> {
    let mut ls = LineString::new(coords(pts));
    ls.close();
    ls
}

// ── Geometry type coverage ─────────────────────────────────────────────────

#[test]
fn points() {
    assert_differential(&layer(vec![pt(0, 0), pt(10, 20), pt(-5, 4000)], None, &[]));
}

#[test]
fn single_point() {
    assert_differential(&layer(vec![pt(7, 9)], None, &[]));
}

#[test]
fn multipoints() {
    let geoms = vec![
        Geometry::MultiPoint(MultiPoint(vec![Point::new(1, 2), Point::new(3, 4)])),
        Geometry::MultiPoint(MultiPoint(vec![Point::new(5, 6)])),
        Geometry::MultiPoint(MultiPoint(vec![
            Point::new(7, 8),
            Point::new(9, 10),
            Point::new(11, 12),
        ])),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn linestrings() {
    let geoms = vec![
        line(&[(0, 0), (10, 10), (20, 5)]),
        line(&[(100, 100), (150, 200)]),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn multilinestrings() {
    let geoms = vec![
        Geometry::MultiLineString(MultiLineString(vec![
            LineString::new(coords(&[(0, 0), (5, 5)])),
            LineString::new(coords(&[(10, 10), (20, 20), (30, 15)])),
        ])),
        Geometry::MultiLineString(MultiLineString(vec![LineString::new(coords(&[
            (50, 50),
            (60, 40),
        ]))])),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn polygons_with_hole() {
    let geoms = vec![
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (100, 0), (100, 100), (0, 100)]),
            vec![ring(&[(20, 20), (40, 20), (40, 40), (20, 40)])],
        )),
        Geometry::Polygon(Polygon::new(
            ring(&[(200, 200), (300, 200), (250, 300)]),
            vec![],
        )),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn multipolygons() {
    let geoms = vec![
        Geometry::MultiPolygon(MultiPolygon(vec![
            Polygon::new(ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]), vec![]),
            Polygon::new(ring(&[(20, 20), (30, 20), (30, 30)]), vec![]),
        ])),
        Geometry::MultiPolygon(MultiPolygon(vec![Polygon::new(
            ring(&[(50, 50), (60, 50), (60, 60), (50, 60)]),
            vec![ring(&[(52, 52), (57, 52), (57, 57)])],
        )])),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn mixed_points_and_lines() {
    let geoms = vec![pt(5, 5), line(&[(0, 0), (10, 10), (20, 0)]), pt(30, 30)];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn mixed_points_and_polygons() {
    let geoms = vec![
        pt(5, 5),
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (100, 0), (100, 100), (0, 100)]),
            vec![],
        )),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

#[test]
fn mixed_polygon_and_multipolygon() {
    let geoms = vec![
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]),
            vec![],
        )),
        Geometry::MultiPolygon(MultiPolygon(vec![
            Polygon::new(ring(&[(20, 20), (30, 20), (30, 30)]), vec![]),
            Polygon::new(ring(&[(40, 40), (50, 40), (50, 50)]), vec![]),
        ])),
    ];
    assert_differential(&layer(geoms, None, &[]));
}

// ── ID column coverage ─────────────────────────────────────────────────────

#[test]
fn ids_u32() {
    let geoms = vec![pt(0, 0), pt(1, 1), pt(2, 2)];
    let ids = Some(vec![Some(1), Some(2), Some(3)]);
    assert_differential(&layer(geoms, ids, &[]));
}

#[test]
fn ids_sequential_delta_friendly() {
    let geoms: Vec<_> = (0..50).map(|i| pt(i, i * 2)).collect();
    let ids = Some((0..50_u64).map(|i| Some(1_000_000 + i)).collect());
    assert_differential(&layer(geoms, ids, &[]));
}

#[test]
fn ids_u64_large() {
    let geoms = vec![pt(0, 0), pt(1, 1)];
    let ids = Some(vec![Some(u64::from(u32::MAX) + 10), Some(u64::MAX - 5)]);
    assert_differential(&layer(geoms, ids, &[]));
}

#[test]
fn ids_optional_with_nulls() {
    let geoms = vec![pt(0, 0), pt(1, 1), pt(2, 2), pt(3, 3)];
    let ids = Some(vec![None, Some(7), None, Some(9)]);
    assert_differential(&layer(geoms, ids, &[]));
}

// ── Scalar property coverage ───────────────────────────────────────────────

#[test]
fn all_scalar_types_non_optional() {
    let geoms = vec![pt(0, 0), pt(1, 1), pt(2, 2)];
    let props = [
        (
            "b",
            vec![true, false, true]
                .into_iter()
                .map(|v| PropValue::Bool(Some(v)))
                .collect(),
        ),
        (
            "i8",
            vec![-1_i8, 0, 127]
                .into_iter()
                .map(|v| PropValue::I8(Some(v)))
                .collect(),
        ),
        (
            "u8",
            vec![0_u8, 128, 255]
                .into_iter()
                .map(|v| PropValue::U8(Some(v)))
                .collect(),
        ),
        (
            "i32",
            vec![-100_000_i32, 0, 100_000]
                .into_iter()
                .map(|v| PropValue::I32(Some(v)))
                .collect(),
        ),
        (
            "u32",
            vec![0_u32, 70_000, u32::MAX]
                .into_iter()
                .map(|v| PropValue::U32(Some(v)))
                .collect(),
        ),
        (
            "i64",
            vec![i64::MIN, 0, i64::MAX]
                .into_iter()
                .map(|v| PropValue::I64(Some(v)))
                .collect(),
        ),
        (
            "u64",
            vec![0_u64, 1, u64::MAX]
                .into_iter()
                .map(|v| PropValue::U64(Some(v)))
                .collect(),
        ),
        (
            "f32",
            vec![-1.5_f32, 0.0, 3.25]
                .into_iter()
                .map(|v| PropValue::F32(Some(v)))
                .collect(),
        ),
        (
            "f64",
            vec![-2.5_f64, 0.0, 1e100]
                .into_iter()
                .map(|v| PropValue::F64(Some(v)))
                .collect(),
        ),
    ];
    assert_differential(&layer(geoms, None, &props));
}

#[test]
fn all_scalar_types_optional_with_nulls() {
    let geoms = vec![pt(0, 0), pt(1, 1), pt(2, 2), pt(3, 3)];
    // Null in the first position exercises typed-null handling.
    let props = [
        (
            "b",
            vec![
                PropValue::Bool(None),
                PropValue::Bool(Some(true)),
                PropValue::Bool(None),
                PropValue::Bool(Some(false)),
            ],
        ),
        (
            "i8",
            vec![
                PropValue::I8(None),
                PropValue::I8(Some(-5)),
                PropValue::I8(Some(5)),
                PropValue::I8(None),
            ],
        ),
        (
            "u8",
            vec![
                PropValue::U8(Some(9)),
                PropValue::U8(None),
                PropValue::U8(None),
                PropValue::U8(Some(200)),
            ],
        ),
        (
            "i32",
            vec![
                PropValue::I32(None),
                PropValue::I32(Some(-1)),
                PropValue::I32(Some(1)),
                PropValue::I32(None),
            ],
        ),
        (
            "u32",
            vec![
                PropValue::U32(Some(1)),
                PropValue::U32(Some(2)),
                PropValue::U32(None),
                PropValue::U32(Some(3)),
            ],
        ),
        (
            "i64",
            vec![
                PropValue::I64(None),
                PropValue::I64(Some(i64::MIN)),
                PropValue::I64(None),
                PropValue::I64(Some(i64::MAX)),
            ],
        ),
        (
            "u64",
            vec![
                PropValue::U64(Some(u64::MAX)),
                PropValue::U64(None),
                PropValue::U64(Some(0)),
                PropValue::U64(None),
            ],
        ),
        (
            "f32",
            vec![
                PropValue::F32(None),
                PropValue::F32(Some(1.5)),
                PropValue::F32(Some(-1.5)),
                PropValue::F32(None),
            ],
        ),
        (
            "f64",
            vec![
                PropValue::F64(Some(2.5)),
                PropValue::F64(None),
                PropValue::F64(None),
                PropValue::F64(Some(-2.5)),
            ],
        ),
    ];
    assert_differential(&layer(geoms, None, &props));
}

/// A constant column exercises the v2 interleaved RLE stream layout.
#[test]
fn rle_friendly_constant_column() {
    let n = 100;
    let geoms: Vec<_> = (0..n).map(|i| pt(i, i)).collect();
    let props = [
        (
            "const",
            (0..n).map(|_| PropValue::I32(Some(42))).collect::<Vec<_>>(),
        ),
        (
            "runs",
            (0..n)
                .map(|i| PropValue::U32(Some(u32::from(i > 50))))
                .collect(),
        ),
    ];
    assert_differential(&layer(geoms, None, &props));
}

/// Presence bitfields around byte boundaries (1, 7, 8, 9, 17 features).
#[test]
fn presence_bitfield_byte_boundaries() {
    for n in [1_u32, 7, 8, 9, 17] {
        #[expect(clippy::cast_possible_wrap, reason = "tiny test values")]
        let geoms: Vec<_> = (0..n).map(|i| pt(i as i32, 0)).collect();
        let values: Vec<PropValue> = (0..n)
            .map(|i| PropValue::U32((i % 2 == 0).then_some(i)))
            .collect();
        assert_differential(&layer(geoms, None, &[("alt", values)]));
    }
}

/// Two layers in one tile, both v2.
#[test]
fn multiple_layers() {
    let a = layer(vec![pt(0, 0)], None, &[]);
    let b = layer(vec![pt(5, 5), pt(6, 6)], Some(vec![Some(1), Some(2)]), &[]);
    let mut tile = a.encode(cfg_v2()).unwrap();
    tile.extend_from_slice(&b.encode(cfg_v2()).unwrap());

    let mut parser = Parser::default();
    let layers = parser.parse_layers(&tile).expect("parse");
    assert_eq!(layers.len(), 2);
    let mut dec = Decoder::default();
    for l in layers {
        assert!(matches!(l, Layer::Tag02(_)));
        l.into_layer01().unwrap().into_tile(&mut dec).unwrap();
    }
}

/// Sort trials enabled: the v2 output must still decode to the same feature set.
#[test]
fn default_config_with_sort_trials() {
    let geoms: Vec<_> = (0..30).map(|i| pt(i * 13 % 100, i * 7 % 100)).collect();
    let ids = Some((0..30_u64).map(Some).collect());
    let l = layer(geoms, ids, &[]);

    let bytes = l
        .clone()
        .encode(EncoderConfig::default().with_wire_version(WireVersion::V02))
        .unwrap();
    let (tag, tile) = decode(&bytes);
    assert_eq!(tag, 2);
    assert_eq!(tile.feature_count(), l.feature_count());
    // Feature order may differ (sorting); compare as multisets by id.
    let mut expected: Vec<_> = l.features().iter().map(TileFeature::id).collect();
    let mut actual: Vec<_> = tile.features().iter().map(TileFeature::id).collect();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(expected, actual);
}

// ── Size expectations ──────────────────────────────────────────────────────

/// v2's headline savings: no `stream_type` bytes, no redundant counts, raw
/// presence bitfields instead of bool-RLE streams with full headers.
#[test]
fn v2_is_smaller_for_typical_layer() {
    let n = 100_u16;
    let geoms: Vec<_> = (0..n)
        .map(|i| pt(i32::from(i) * 3, i32::from(i) * 5))
        .collect();
    let ids = Some((0..u64::from(n)).map(|i| Some(1000 + i)).collect());
    // Alternating presence is RLE-hostile, favoring v2's raw bitfields.
    let props = [
        (
            "opt_a",
            (0..n)
                .map(|i| PropValue::U32((i % 2 == 0).then(|| u32::from(i))))
                .collect::<Vec<_>>(),
        ),
        (
            "opt_b",
            (0..n)
                .map(|i| PropValue::I32((i % 2 == 1).then(|| -i32::from(i))))
                .collect(),
        ),
        (
            "clazz",
            (0..n)
                .map(|i| PropValue::U32(Some(u32::from(i % 7 == 0))))
                .collect(),
        ),
        (
            "height",
            (0..n)
                .map(|i| PropValue::F32(Some(f32::from(i) * 0.5)))
                .collect(),
        ),
    ];
    let l = layer(geoms, ids, &props);
    let (v1_len, v2_len) = assert_differential(&l);
    assert!(
        v2_len < v1_len,
        "v2 ({v2_len} B) should be smaller than v1 ({v1_len} B)"
    );
}

// ── Unsupported features fail cleanly ──────────────────────────────────────

#[test]
fn string_columns_not_yet_supported() {
    let l = layer(
        vec![pt(0, 0), pt(1, 1)],
        None,
        &[(
            "name",
            vec![
                PropValue::Str(Some("a".to_string())),
                PropValue::Str(Some("b".to_string())),
            ],
        )],
    );
    let err = l.encode(cfg_v2()).unwrap_err();
    assert!(err.to_string().contains("not"), "unexpected error: {err}");
}

#[test]
fn tessellation_not_yet_supported() {
    let l = layer(
        vec![Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]),
            vec![],
        ))],
        None,
        &[],
    );
    let err = l.encode(cfg_v2().with_tessellation(true)).unwrap_err();
    assert!(err.to_string().contains("not"), "unexpected error: {err}");
}
