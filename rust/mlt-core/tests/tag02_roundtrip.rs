//! Round-trip and differential tests for the experimental v2 (tag `0x02`) wire format.

use mlt_core::dump::{RenderOpts, annotate_tile, render};
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{Decoder, Layer, Parser, PropValue, TileFeature, TileLayer};
use rstest::rstest;

fn cfg_v1() -> EncoderConfig {
    // sorting because sort trials could otherwise pick different winners per format.
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
}

fn cfg_v2() -> EncoderConfig {
    cfg_v1().with_wire_version(WireVersion::V02)
}

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

fn assert_differential(layer: &TileLayer) -> (usize, usize) {
    let v1_bytes = layer.clone().encode(cfg_v1()).expect("v1 encode");
    let v2_bytes = layer.clone().encode(cfg_v2()).expect("v2 encode");
    let (tag1, tile1) = decode(&v1_bytes);
    let (tag2, tile2) = decode(&v2_bytes);
    assert_eq!(tag1, 1);
    assert_eq!(tag2, 2);
    assert_eq!(tile1, tile2, "v1 and v2 decoded layers must be identical");
    assert_dump_covers(&v1_bytes);
    assert_dump_covers(&v2_bytes);
    (v1_bytes.len(), v2_bytes.len())
}

/// The annotated dump of `bytes`, rendered as the `mlt dump` CLI would show it.
fn dump_text(bytes: &[u8]) -> String {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let mut out = Vec::new();
    render(&tree, bytes, &RenderOpts::default(), &mut out).expect("render");
    String::from_utf8(out).expect("dump is utf8")
}

fn assert_dump_covers(bytes: &[u8]) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let mut leaves: Vec<(usize, usize)> = tree
        .regions
        .iter()
        .filter(|r| !r.container)
        .map(|r| (r.offset, r.len))
        .collect();
    leaves.sort_unstable();

    let mut cursor = 0;
    for (offset, len) in &leaves {
        assert_eq!(*offset, cursor, "gap/overlap at offset {offset}");
        cursor += len;
    }
    assert_eq!(
        cursor,
        bytes.len(),
        "dump covers {cursor} of {} bytes",
        bytes.len()
    );

    render(&tree, bytes, &RenderOpts::default(), &mut Vec::new()).expect("render");
}

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

/// One point per character of a presence mask, so masks alone size a layer.
fn points(mask: &str) -> Vec<Geometry<i32>> {
    (0..mask.len())
        .map(|i| pt(i32::try_from(i).unwrap(), 0))
        .collect()
}

/// An optional `u32` column holding a value at every `'1'` of `mask`.
fn opt_col(mask: &str) -> Vec<PropValue> {
    mask.bytes()
        .enumerate()
        .map(|(i, b)| PropValue::U32((b == b'1').then(|| u32::try_from(i).unwrap())))
        .collect()
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

#[rstest]
#[case::points(vec![pt(0, 0), pt(10, 20), pt(-5, 4000)])]
#[case::single_point(vec![pt(7, 9)])]
#[case::multipoints(vec![
    Geometry::MultiPoint(MultiPoint(vec![Point::new(1, 2), Point::new(3, 4)])),
    Geometry::MultiPoint(MultiPoint(vec![Point::new(5, 6)])),
    Geometry::MultiPoint(MultiPoint(vec![
        Point::new(7, 8),
        Point::new(9, 10),
        Point::new(11, 12),
    ])),
])]
#[case::linestrings(vec![
    line(&[(0, 0), (10, 10), (20, 5)]),
    line(&[(100, 100), (150, 200)]),
])]
#[case::multilinestrings(vec![
    Geometry::MultiLineString(MultiLineString(vec![
        LineString::new(coords(&[(0, 0), (5, 5)])),
        LineString::new(coords(&[(10, 10), (20, 20), (30, 15)])),
    ])),
    Geometry::MultiLineString(MultiLineString(vec![LineString::new(coords(&[
        (50, 50),
        (60, 40),
    ]))])),
])]
#[case::polygons_with_hole(vec![
    Geometry::Polygon(Polygon::new(
        ring(&[(0, 0), (100, 0), (100, 100), (0, 100)]),
        vec![ring(&[(20, 20), (40, 20), (40, 40), (20, 40)])],
    )),
    Geometry::Polygon(Polygon::new(
        ring(&[(200, 200), (300, 200), (250, 300)]),
        vec![],
    )),
])]
#[case::multipolygons(vec![
    Geometry::MultiPolygon(MultiPolygon(vec![
        Polygon::new(ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]), vec![]),
        Polygon::new(ring(&[(20, 20), (30, 20), (30, 30)]), vec![]),
    ])),
    Geometry::MultiPolygon(MultiPolygon(vec![Polygon::new(
        ring(&[(50, 50), (60, 50), (60, 60), (50, 60)]),
        vec![ring(&[(52, 52), (57, 52), (57, 57)])],
    )])),
])]
#[case::mixed_points_and_lines(vec![pt(5, 5), line(&[(0, 0), (10, 10), (20, 0)]), pt(30, 30)])]
#[case::mixed_points_and_polygons(vec![
    pt(5, 5),
    Geometry::Polygon(Polygon::new(
        ring(&[(0, 0), (100, 0), (100, 100), (0, 100)]),
        vec![],
    )),
])]
#[case::mixed_polygon_and_multipolygon(vec![
    Geometry::Polygon(Polygon::new(
        ring(&[(0, 0), (10, 0), (10, 10), (0, 10)]),
        vec![],
    )),
    Geometry::MultiPolygon(MultiPolygon(vec![
        Polygon::new(ring(&[(20, 20), (30, 20), (30, 30)]), vec![]),
        Polygon::new(ring(&[(40, 40), (50, 40), (50, 50)]), vec![]),
    ])),
])]
fn geometry_types(#[case] geoms: Vec<Geometry<i32>>) {
    assert_differential(&layer(geoms, None, &[]));
}

#[rstest]
#[case::u32(vec![pt(0, 0), pt(1, 1), pt(2, 2)], vec![Some(1), Some(2), Some(3)])]
#[case::sequential_delta_friendly(
    (0..50).map(|i| pt(i, i * 2)).collect(),
    (0..50_u64).map(|i| Some(1_000_000 + i)).collect(),
)]
#[case::u64_large(
    vec![pt(0, 0), pt(1, 1)],
    vec![Some(u64::from(u32::MAX) + 10), Some(u64::MAX - 5)],
)]
#[case::optional_with_nulls(
    vec![pt(0, 0), pt(1, 1), pt(2, 2), pt(3, 3)],
    vec![None, Some(7), None, Some(9)],
)]
fn ids(#[case] geoms: Vec<Geometry<i32>>, #[case] feature_ids: Vec<Option<u64>>) {
    assert_differential(&layer(geoms, Some(feature_ids), &[]));
}

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

#[test]
fn bool_column_is_packed_bitfield() {
    let n = 256_i32;
    let geoms: Vec<_> = (0..n).map(|i| pt(i, 0)).collect();
    let values: Vec<PropValue> = (0..n).map(|i| PropValue::Bool(Some(i % 2 == 0))).collect();
    let l = layer(geoms, None, &[("flag", values)]);
    let (_v1, v2) = assert_differential(&l);
    // Ceiling still fails if the 32 B bitfield regresses to a byte per bool.
    assert!(v2 < 900, "v2 tile unexpectedly large: {v2} B");
}

#[rstest]
fn presence_bitfield_byte_boundaries(#[values(1_u32, 7, 8, 9, 17)] n: u32) {
    #[expect(clippy::cast_possible_wrap, reason = "tiny test values")]
    let geoms: Vec<_> = (0..n).map(|i| pt(i as i32, 0)).collect();
    let values: Vec<PropValue> = (0..n)
        .map(|i| PropValue::U32((i % 2 == 0).then_some(i)))
        .collect();
    assert_differential(&layer(geoms, None, &[("alt", values)]));
}

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

#[test]
fn columns_with_the_same_nulls_share_one_presence_bitfield() {
    const SAME: &str = "100100100100100100";
    const ODD_ONE_OUT: &str = "100010001000100010";
    let props = [
        ("a", opt_col(SAME)),
        ("b", opt_col(SAME)),
        ("c", opt_col(SAME)),
        ("d", opt_col(ODD_ONE_OUT)),
    ];
    let l = layer(points(SAME), None, &props);
    assert_differential(&l);

    let dump = dump_text(&l.encode(cfg_v2()).unwrap());
    assert!(dump.contains("shared presence bitfields = 1"), "{dump}");
    assert_eq!(dump.matches("presence = Shared(0)").count(), 3, "{dump}");
    // A mask only one column has stays with that column.
    assert_eq!(dump.matches("presence = Inline").count(), 1, "{dump}");
    // One bitfield for the three columns that agree, one for the odd one out.
    assert_eq!(dump.matches("[Present ").count(), 2, "{dump}");
}

#[test]
fn shared_presence_count_is_capped_by_the_layout_byte() {
    // Nine features give more than seven distinct masks to go around.
    let masks: Vec<String> = (0..8)
        .map(|i| {
            let mut mask = vec![b'0'; 9];
            mask[0] = b'1';
            mask[i + 1] = b'1';
            String::from_utf8(mask).unwrap()
        })
        .collect();
    let names: Vec<String> = (0..masks.len() * 2).map(|i| format!("c{i}")).collect();
    let props: Vec<(&str, Vec<PropValue>)> = masks
        .iter()
        .flat_map(|mask| [opt_col(mask), opt_col(mask)])
        .zip(&names)
        .map(|(values, name)| (name.as_str(), values))
        .collect();
    let l = layer(points(&masks[0]), None, &props);
    assert_differential(&l);

    let dump = dump_text(&l.encode(cfg_v2()).unwrap());
    assert!(dump.contains("shared presence bitfields = 7"), "{dump}");
    assert_eq!(dump.matches("presence = Shared(6)").count(), 2, "{dump}");
    // Every group is shared by two columns, so the last one loses the tie-break.
    assert_eq!(dump.matches("presence = Inline").count(), 2, "{dump}");
    assert_eq!(dump.matches("[Present ").count(), 9, "{dump}");
}

#[test]
fn an_optional_id_shares_its_presence_bitfield_with_a_column() {
    const MASK: &str = "10110010";
    let ids = MASK
        .bytes()
        .enumerate()
        .map(|(i, b)| (b == b'1').then(|| i as u64 + 1))
        .collect();
    let l = layer(points(MASK), Some(ids), &[("val", opt_col(MASK))]);
    assert_differential(&l);

    let dump = dump_text(&l.encode(cfg_v2()).unwrap());
    assert!(dump.contains("shared presence bitfields = 1"), "{dump}");
    assert_eq!(dump.matches("presence = Shared(0)").count(), 2, "{dump}");
    assert_eq!(dump.matches("[Present ").count(), 1, "{dump}");
}

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
