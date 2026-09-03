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

#[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
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
    assert_differential_with(layer, cfg_v1())
}

/// As [`assert_differential`], with `cfg` deciding everything but the wire version.
fn assert_differential_with(layer: &TileLayer, cfg: EncoderConfig) -> (usize, usize) {
    let v1_bytes = layer.clone().encode(cfg).expect("v1 encode");
    let v2_bytes = layer
        .clone()
        .encode(cfg.with_wire_version(WireVersion::V02))
        .expect("v2 encode");
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
fn a_bitpacking_friendly_column_takes_fastpfor128() {
    let n = 2000_u32;
    let geoms: Vec<_> = (0..n).map(|i| pt(i32::try_from(i).unwrap(), 0)).collect();
    let values: Vec<PropValue> = (0..n)
        .map(|i| PropValue::U32(Some(i.wrapping_mul(2_654_435_761) % 4096)))
        .collect();
    let l = layer(geoms, None, &[("scattered", values)]);

    let v2 = l.clone().encode(cfg_v2()).expect("v2 encode");
    assert!(
        dump_text(&v2).contains("physical = FastPFor128"),
        "v2 should code the scattered column with FastPFor128"
    );
    assert_differential(&l);

    let without = l
        .encode(cfg_v2().with_fastpfor(false))
        .expect("v2 encode without fastpfor");
    assert!(
        v2.len() < without.len(),
        "FastPFor128 ({} B) should beat the varint fallback ({} B)",
        v2.len(),
        without.len()
    );
}

#[rstest]
fn a_fastpfor128_stream_round_trips_across_block_boundaries(
    #[values(127, 128, 129, 255, 256, 257, 383, 384, 385, 511, 512, 513)] n: u32,
) {
    let geoms: Vec<_> = (0..n).map(|i| pt(i32::try_from(i).unwrap(), 0)).collect();
    let values: Vec<PropValue> = (0..n)
        .map(|i| PropValue::U32(Some(i.wrapping_mul(2_654_435_761) % 4096)))
        .collect();
    let l = layer(geoms, None, &[("scattered", values)]);
    let v2 = l.clone().encode(cfg_v2()).expect("v2 encode");
    assert!(
        dump_text(&v2).contains("physical = FastPFor128"),
        "{n} scattered values should be coded with FastPFor128"
    );
    assert_differential(&l);
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

mod geometry_layouts {
    use super::*;

    fn cfg_tessellated() -> EncoderConfig {
        cfg_v1().with_tessellation(true)
    }

    fn square(x: i32, y: i32) -> Polygon<i32> {
        Polygon::new(
            ring(&[(x, y), (x + 10, y), (x + 10, y + 10), (x, y + 10)]),
            vec![],
        )
    }

    /// Points cycling through a handful of coordinates, so a vertex dictionary pays off.
    fn repeated_points(n: i32) -> Vec<Geometry<i32>> {
        (0..n).map(|i| pt((i % 7) * 10, (i % 5) * 10)).collect()
    }

    #[test]
    fn repeated_vertices_pick_a_dictionary_layout() {
        let l = layer(repeated_points(200), None, &[]);
        let dump = dump_text(&l.clone().encode(cfg_v2()).unwrap());
        assert!(dump.contains("geometry layout = PointsDict"), "{dump}");
        assert!(dump.contains("vertex_offsets"), "{dump}");
        assert_differential(&l);
    }

    #[test]
    fn distinct_vertices_stay_plain() {
        let l = layer((0..64).map(|i| pt(i * 7, i * 13)).collect(), None, &[]);
        let dump = dump_text(&l.clone().encode(cfg_v2()).unwrap());
        assert!(dump.contains("geometry layout = Points\n"), "{dump}");
        assert_differential(&l);
    }

    #[rstest]
    #[case::polygons(vec![
        Geometry::Polygon(square(0, 0)),
        Geometry::Polygon(Polygon::new(
            ring(&[(0, 0), (100, 0), (100, 100), (0, 100)]),
            vec![ring(&[(20, 20), (40, 20), (40, 40), (20, 40)])],
        )),
    ])]
    #[case::mixed_points_and_polygons(vec![pt(5, 5), Geometry::Polygon(square(20, 20))])]
    #[case::mixed_lines_and_polygons(vec![
        line(&[(0, 0), (10, 10), (20, 0)]),
        Geometry::Polygon(square(40, 40)),
    ])]
    #[case::multipolygons(vec![
        Geometry::MultiPolygon(MultiPolygon(vec![square(0, 0), square(20, 20)])),
        Geometry::Polygon(square(50, 50)),
    ])]
    fn tessellated_geometry(#[case] geoms: Vec<Geometry<i32>>) {
        let l = layer(geoms, None, &[]);
        let dump = dump_text(
            &l.clone()
                .encode(cfg_tessellated().with_wire_version(WireVersion::V02))
                .unwrap(),
        );
        assert!(
            dump.contains("geometry layout = TessPolygonsWithOutlines"),
            "{dump}"
        );
        assert_differential_with(&l, cfg_tessellated());
    }

    #[test]
    fn a_layer_without_polygons_is_not_tessellated() {
        let l = layer(vec![pt(5, 5), line(&[(0, 0), (10, 10)])], None, &[]);
        let dump = dump_text(
            &l.clone()
                .encode(cfg_tessellated().with_wire_version(WireVersion::V02))
                .unwrap(),
        );
        assert!(dump.contains("geometry layout = Lines"), "{dump}");
        assert_differential_with(&l, cfg_tessellated());
    }
}

mod strings {
    use super::*;

    /// A value long and repetitive enough for FSST to pay off its symbol table.
    /// The seed leads, so two of them share no prefix worth coding.
    fn long(seed: usize) -> String {
        let lead = char::from(b'a' + u8::try_from(seed).unwrap());
        format!("{lead}_residential_zone_north_sector_").repeat(16)
    }

    /// As [`long`], with the seed last, so two of them share all but their final bytes.
    fn long_shared(seed: usize) -> String {
        format!("residential_zone_north_sector_{seed:03}_").repeat(16)
    }

    /// Distinct short values sharing no prefix, which no dictionary or symbol table improves on.
    fn plain_values(n: usize) -> Vec<String> {
        (0..n)
            .map(|i| {
                let lead = char::from(b'a' + u8::try_from(i).unwrap());
                format!("{lead}_zone_marker")
            })
            .collect()
    }

    /// Two long values alternating, which is what a dictionary is for.
    /// They share no prefix, so front coding would only add a length per entry.
    fn dict_values(n: usize) -> Vec<String> {
        (0..n)
            .map(|i| {
                let lead = if i % 2 == 0 { "A" } else { "B" };
                lead.repeat(30)
            })
            .collect()
    }

    /// As [`dict_values`], with the two sharing all but their last byte, which front coding factors out.
    fn front_dict_values(n: usize) -> Vec<String> {
        (0..n)
            .map(|i| format!("{}{}", "A".repeat(30), i % 2))
            .collect()
    }

    /// Distinct values FSST compresses, so a dictionary of them would only add codes.
    fn fsst_values(n: usize) -> Vec<String> {
        (0..n).map(long).collect()
    }

    /// Four of those repeated, so the dictionary and the symbol table both pay off.
    fn fsst_dict_values(n: usize) -> Vec<String> {
        (0..n).map(|i| long(i % 4)).collect()
    }

    /// As [`fsst_dict_values`], with the four sharing prefixes for front coding to factor out.
    fn fsst_front_dict_values(n: usize) -> Vec<String> {
        (0..n).map(|i| long_shared(i % 4)).collect()
    }

    type Values = fn(usize) -> Vec<String>;

    fn column(values: impl IntoIterator<Item = Option<String>>) -> TileLayer {
        let values: Vec<PropValue> = values.into_iter().map(PropValue::Str).collect();
        layer(points(&"1".repeat(values.len())), None, &[("v", values)])
    }

    /// Whether the tile writes a front-coded dictionary blob.
    fn is_front_coded(bytes: &[u8]) -> bool {
        dump_text(bytes).contains("logical = FrontCoded")
    }

    /// The layout of every string column in the tile, in wire order.
    fn layouts(bytes: &[u8]) -> Vec<String> {
        dump_text(bytes)
            .lines()
            .filter_map(|line| line.split_once("string layout = "))
            .map(|(_, layout)| layout.trim().to_string())
            .collect()
    }

    #[rstest]
    #[case::plain(plain_values as Values, "Plain")]
    #[case::dict(dict_values as Values, "Dict")]
    #[case::front_dict(front_dict_values as Values, "Dict")]
    #[case::fsst(fsst_values as Values, "Fsst")]
    #[case::fsst_dict(fsst_dict_values as Values, "FsstDict")]
    #[case::fsst_front_dict(fsst_front_dict_values as Values, "FsstDict")]
    fn each_layout_round_trips(#[case] values: Values, #[case] layout: &str) {
        let l = column(values(9).into_iter().map(Some));
        assert_differential(&l);
        assert_eq!(layouts(&l.encode(cfg_v2()).unwrap()), [layout]);
    }

    #[rstest]
    #[case::plain(plain_values as Values, "Plain")]
    #[case::dict(dict_values as Values, "Dict")]
    #[case::front_dict(front_dict_values as Values, "Dict")]
    #[case::fsst(fsst_values as Values, "Fsst")]
    #[case::fsst_dict(fsst_dict_values as Values, "FsstDict")]
    #[case::fsst_front_dict(fsst_front_dict_values as Values, "FsstDict")]
    fn each_layout_round_trips_with_nulls(#[case] values: Values, #[case] layout: &str) {
        let l = column(
            values(12)
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i % 3 != 0).then_some(v)),
        );
        assert_differential(&l);
        assert_eq!(layouts(&l.encode(cfg_v2()).unwrap()), [layout]);
    }

    /// Front coding is raced like every other layout, so it should win exactly when the
    /// dictionary's entries share prefixes to factor out.
    #[rstest]
    #[case::entries_share_a_prefix(front_dict_values as Values, true)]
    #[case::entries_share_no_prefix(dict_values as Values, false)]
    fn front_coding_wins_on_shared_prefixes(#[case] values: Values, #[case] front_coded: bool) {
        let l = column(values(9).into_iter().map(Some));
        assert_differential(&l);
        assert_eq!(is_front_coded(&l.encode(cfg_v2()).unwrap()), front_coded);
    }

    #[test]
    fn empty_values_round_trip() {
        let l = column(["", "a", "", "bb", ""].map(|v| Some(v.to_string())));
        assert_differential(&l);
    }

    #[test]
    fn a_column_starting_with_a_null_round_trips() {
        let l = column([None, Some("a".to_string()), None, Some("b".to_string())]);
        assert_differential(&l);
    }

    #[test]
    fn non_ascii_values_round_trip() {
        let l = column(["日本語", "Ünïcödé", "🌍 🌏", ""].map(|v| Some(v.to_string())));
        assert_differential(&l);
    }

    #[test]
    fn a_column_of_one_repeated_value_round_trips() {
        let l = column((0..8).map(|_| Some("same".to_string())));
        assert_differential(&l);
    }

    #[test]
    fn a_string_column_shares_its_presence_bitfield_with_a_scalar_one() {
        const MASK: &str = "10110010";
        let strings: Vec<PropValue> = MASK
            .bytes()
            .enumerate()
            .map(|(i, b)| PropValue::Str((b == b'1').then(|| format!("v{i}"))))
            .collect();
        let l = layer(points(MASK), None, &[("s", strings), ("n", opt_col(MASK))]);
        assert_differential(&l);

        let dump = dump_text(&l.encode(cfg_v2()).unwrap());
        assert!(dump.contains("shared presence bitfields = 1"), "{dump}");
        assert_eq!(dump.matches("presence = Shared(0)").count(), 2, "{dump}");
    }

    /// A layer whose two string columns hold the same values, which both versions group.
    fn shared_dict_layer() -> TileLayer {
        let shared: Vec<PropValue> = dict_values(6)
            .into_iter()
            .map(|v| PropValue::Str(Some(v)))
            .collect();
        layer(
            points(&"1".repeat(6)),
            None,
            &[("a", shared.clone()), ("b", shared)],
        )
    }

    #[test]
    fn columns_v1_would_share_a_dictionary_share_one_in_v2_too() {
        let l = shared_dict_layer();
        assert!(dump_text(&l.clone().encode(cfg_v1()).unwrap()).contains("SharedDict"));
        assert!(dump_text(&l.clone().encode(cfg_v2()).unwrap()).contains("SharedDict"));
        assert_differential(&l);
    }

    #[test]
    fn a_shared_dictionary_is_smaller_than_per_column_ones() {
        let l = shared_dict_layer();
        let shared = l.clone().encode(cfg_v2()).unwrap().len();
        let separate = l.encode(cfg_v2().with_shared_dict(false)).unwrap().len();
        assert!(shared < separate, "shared {shared} vs separate {separate}");
    }

    #[test]
    fn a_shared_dictionary_with_nulls_round_trips() {
        let mask = "101101";
        let values = |offset: usize| -> Vec<PropValue> {
            mask.bytes()
                .enumerate()
                .map(|(i, b)| {
                    PropValue::Str((b == b'1').then(|| format!("name:{}", (i + offset) % 3)))
                })
                .collect()
        };
        let l = layer(
            points(mask),
            None,
            &[("name:de", values(0)), ("name:en", values(1))],
        );
        assert!(dump_text(&l.clone().encode(cfg_v2()).unwrap()).contains("SharedDict"));
        assert_differential(&l);
    }
}

mod float_codecs {
    use mlt_core::wire::{FloatLogical, LogicalEncoding, PhysicalEncoding};

    use super::*;

    pub fn cfg_dict() -> EncoderConfig {
        cfg_v2().with_float_dict(true)
    }

    pub fn cfg_alp() -> EncoderConfig {
        cfg_v2().with_float_alp(true)
    }

    pub fn cfg_both() -> EncoderConfig {
        cfg_dict().with_float_alp(true)
    }

    pub fn f64_col(values: &[f64]) -> Vec<PropValue> {
        values.iter().map(|&v| PropValue::F64(Some(v))).collect()
    }

    pub fn f32_col(values: &[f32]) -> Vec<PropValue> {
        values.iter().map(|&v| PropValue::F32(Some(v))).collect()
    }

    pub fn f32_column(values: &[f32]) -> TileLayer {
        layer(
            points(&"1".repeat(values.len())),
            None,
            &[("v", f32_col(values))],
        )
    }

    /// A few decimals repeated, which is what a float dictionary is for.
    pub fn repeated_decimals(n: usize) -> Vec<f64> {
        const PATTERN: [f64; 6] = [1.5, 2.5, 1.5, 1.5, 2.5, 3.5];
        (0..n).map(|i| PATTERN[i % PATTERN.len()]).collect()
    }

    /// Values no power of ten scales to an integer, so ALP cannot take them.
    pub fn irrational(n: usize) -> Vec<f64> {
        use std::f64::consts::{E, PI, SQRT_2};
        (0..n).map(|i| [PI, E, SQRT_2][i % 3]).collect()
    }

    /// The column's values as bit patterns, so NaN and `-0.0` compare usefully.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only float values are expected"
    )]
    pub fn value_bits(tile: &TileLayer) -> Vec<u64> {
        tile.features()
            .iter()
            .map(|f| match f.properties()[0] {
                PropValue::F64(Some(v)) => v.to_bits(),
                PropValue::F32(Some(v)) => u64::from(v.to_bits()),
                ref other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    /// The physical encoding each float stream in the tile carries, in wire order.
    pub fn float_physicals(bytes: &[u8]) -> Vec<PhysicalEncoding> {
        annotate_tile(bytes)
            .expect("annotate_tile")
            .regions
            .iter()
            .filter_map(|r| r.blob)
            .filter_map(|b| match b.meta.encoding.logical {
                LogicalEncoding::Float(_) => Some(b.meta.encoding.physical),
                LogicalEncoding::Int(_) | LogicalEncoding::Bool(_) | LogicalEncoding::Vertex(_) => {
                    None
                }
            })
            .collect()
    }

    /// The encoding each float stream in the tile carries, in wire order.
    pub fn float_encodings(bytes: &[u8]) -> Vec<FloatLogical> {
        annotate_tile(bytes)
            .expect("annotate_tile")
            .regions
            .iter()
            .filter_map(|r| r.blob)
            .filter_map(|b| match b.meta.encoding.logical {
                LogicalEncoding::Float(logical) => Some(logical),
                LogicalEncoding::Int(_) | LogicalEncoding::Bool(_) | LogicalEncoding::Vertex(_) => {
                    None
                }
            })
            .collect()
    }

    pub fn round_trip(l: &TileLayer, config: EncoderConfig) -> TileLayer {
        let bytes = l.clone().encode(config).expect("encode");
        let (tag, tile) = decode(&bytes);
        assert_eq!(tag, 2);
        assert_dump_covers(&bytes);
        tile
    }

    pub fn column(values: &[f64]) -> TileLayer {
        let mask = "1".repeat(values.len());
        layer(points(&mask), None, &[("v", f64_col(values))])
    }

    #[test]
    fn a_repetitive_column_round_trips_through_a_dictionary() {
        let l = column(&repeated_decimals(12));
        assert_eq!(round_trip(&l, cfg_dict()), round_trip(&l, cfg_v2()));
    }

    #[test]
    fn a_dictionary_column_is_smaller_than_the_raw_one() {
        let l = column(&repeated_decimals(12));
        let plain = l.clone().encode(cfg_v2()).unwrap();
        let dict = l.clone().encode(cfg_dict()).unwrap();
        assert!(
            dict.len() < plain.len(),
            "{} vs {}",
            dict.len(),
            plain.len()
        );
        assert_eq!(
            value_bits(&round_trip(&l, cfg_dict())),
            value_bits(&round_trip(&l, cfg_v2()))
        );
    }

    #[test]
    fn a_decimal_column_takes_the_dictionary_when_alp_is_off() {
        let bytes = column(&repeated_decimals(12)).encode(cfg_dict()).unwrap();
        assert_eq!(
            float_encodings(&bytes),
            [FloatLogical::Dict, FloatLogical::None]
        );
    }

    #[test]
    fn a_column_alp_cannot_carry_still_takes_the_dictionary() {
        let bytes = column(&irrational(12)).encode(cfg_both()).unwrap();
        assert_eq!(
            float_encodings(&bytes),
            [FloatLogical::Dict, FloatLogical::None]
        );
    }

    #[test]
    fn both_flags_off_keeps_a_repetitive_column_raw() {
        let bytes = column(&repeated_decimals(12)).encode(cfg_v2()).unwrap();
        assert_eq!(float_encodings(&bytes), [FloatLogical::None]);
    }

    #[test]
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only f64 values are expected"
    )]
    fn zero_and_negative_zero_stay_distinct_entries() {
        const VALUES: &[f64] = &[0.0, -0.0, 0.0, -0.0, 0.0, -0.0];
        let l = column(VALUES);
        let tile = round_trip(&l, cfg_dict());
        let signs: Vec<bool> = tile
            .features()
            .iter()
            .map(|f| match f.properties()[0] {
                PropValue::F64(Some(v)) => v.is_sign_negative(),
                ref other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(signs, [false, true, false, true, false, true]);
    }

    #[test]
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only f64 values are expected"
    )]
    fn every_nan_with_the_same_bits_shares_one_entry() {
        let values: Vec<f64> = (0..8)
            .map(|i| if i % 2 == 0 { f64::NAN } else { 7.5 })
            .collect();
        let l = column(&values);
        let tile = round_trip(&l, cfg_dict());
        let nans: Vec<bool> = tile
            .features()
            .iter()
            .map(|f| match f.properties()[0] {
                PropValue::F64(Some(v)) => v.is_nan(),
                ref other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(nans, [true, false, true, false, true, false, true, false]);
    }

    #[test]
    fn an_all_distinct_column_no_encoding_fits_stays_raw() {
        let values: Vec<f64> = (0..16)
            .map(|i| std::f64::consts::PI * f64::from(i + 1))
            .collect();
        let l = column(&values);
        assert_eq!(
            l.clone().encode(cfg_both()).unwrap(),
            l.encode(cfg_v2()).unwrap()
        );
    }

    #[test]
    fn f32_columns_round_trip_through_a_dictionary() {
        let values: Vec<f32> = (0..12).map(|i| [1.5_f32, 2.5, 3.5][i % 3]).collect();
        let l = f32_column(&values);
        assert_eq!(round_trip(&l, cfg_dict()), round_trip(&l, cfg_v2()));
    }

    #[test]
    fn an_optional_dictionary_column_round_trips() {
        let values: Vec<PropValue> = (0..12)
            .map(|i| PropValue::F64((i % 3 != 0).then(|| f64::from(i % 2) + 0.5)))
            .collect();
        let l = layer(points(&"1".repeat(12)), None, &[("v", values)]);
        assert_eq!(round_trip(&l, cfg_dict()), round_trip(&l, cfg_v2()));
    }

    #[rstest]
    #[case::dict(cfg_v1().with_float_dict(true))]
    #[case::alp(cfg_v1().with_float_alp(true))]
    #[case::both(cfg_v1().with_float_dict(true).with_float_alp(true))]
    fn v1_ignores_the_flags(#[case] config: EncoderConfig) {
        const VALUES: &[f64] = &[1.5, 1.5, 1.5, 1.5];
        let l = column(VALUES);
        assert_eq!(
            l.clone().encode(config).unwrap(),
            l.encode(cfg_v1()).unwrap()
        );
    }
}

mod alp {
    use mlt_core::wire::{FastPForKind, FloatLogical, PhysicalEncoding};

    use super::float_codecs::*;
    use super::*;

    #[test]
    fn a_decimal_column_is_smaller_through_alp() {
        let values: Vec<f64> = (0..64).map(|i| f64::from(i) * 0.25 - 8.0).collect();
        let l = column(&values);
        let plain = l.clone().encode(cfg_v2()).unwrap();
        let coded = l.clone().encode(cfg_alp()).unwrap();

        assert_eq!(round_trip(&l, cfg_alp()), round_trip(&l, cfg_v2()));
        assert!(
            coded.len() < plain.len(),
            "{} vs {}",
            coded.len(),
            plain.len()
        );
    }

    #[rstest]
    #[case::short_near_zero((0..8).map(|i| f64::from(i) * 0.25).collect())]
    #[case::long_near_zero((0..1024).map(|i| f64::from(i % 97) * 0.25).collect())]
    #[case::long_far_from_zero((0..1024).map(|i| 52_500_000.0 + f64::from(i % 97) * 0.25).collect())]
    #[case::negative_only((0..600).map(|i| -1000.0 - f64::from(i % 53) * 0.5).collect())]
    #[case::straddling_zero((0..600).map(|i| f64::from(i % 53) * 0.5 - 13.0).collect())]
    #[case::single_value(vec![1.25])]
    #[case::all_equal(vec![4.0; 600])]
    #[case::wide_spread((0..600).map(|i| if i % 2 == 0 { -1e12 } else { 1e12 }).collect())]
    // The widest offsets an exception-free column can hold: the two ends of the `i64` code range,
    // whose distance only just fits `u64`.
    #[case::offsets_nearly_fill_u64(vec![-9.223_372_036_854_775e18, 9.223_372_036_854_775e18])]
    fn any_alp_column_shape_round_trips_bit_for_bit(#[case] values: Vec<f64>) {
        let l = column(&values);
        assert_eq!(
            value_bits(&round_trip(&l, cfg_alp())),
            value_bits(&round_trip(&l, cfg_v2()))
        );
    }

    /// A long column of narrow offsets is what bitpacking is for.
    #[test]
    fn a_long_alp_column_takes_fastpfor_over_varint() {
        let values: Vec<f64> = (0..1024)
            .map(|i| 1000.0 + f64::from(i % 97) * 0.25)
            .collect();
        let l = column(&values);
        let packed = l.clone().encode(cfg_alp()).unwrap();
        let varint = l.clone().encode(cfg_alp().with_fastpfor(false)).unwrap();

        assert_eq!(
            float_physicals(&packed)[..],
            [PhysicalEncoding::FastPFor(FastPForKind::Block128Le)]
        );
        assert!(
            packed.len() < varint.len(),
            "{} vs {}",
            packed.len(),
            varint.len()
        );
        assert_eq!(
            value_bits(&round_trip(&l, cfg_alp())),
            value_bits(&round_trip(&l, cfg_v2()))
        );
    }

    /// Block framing does not amortise over a handful of values, so varint must keep it.
    #[test]
    fn a_short_alp_column_keeps_varint() {
        let values: Vec<f64> = (0..8).map(|i| f64::from(i) * 0.25).collect();
        let bytes = column(&values).encode(cfg_alp()).unwrap();
        assert_eq!(float_physicals(&bytes)[..], [PhysicalEncoding::VarInt]);
    }

    /// `FastPFOR` codes `u32` words, so a column whose offsets overflow one is not a candidate.
    #[test]
    fn a_column_whose_offsets_overflow_u32_keeps_varint() {
        let values: Vec<f64> = (0..1024)
            .map(|i| if i % 2 == 0 { 0.5 } else { 1e15 + 0.5 })
            .collect();
        let bytes = column(&values).encode(cfg_alp()).unwrap();
        assert_eq!(float_physicals(&bytes)[..], [PhysicalEncoding::VarInt]);
        assert_eq!(
            value_bits(&round_trip(&column(&values), cfg_alp())),
            value_bits(&round_trip(&column(&values), cfg_v2()))
        );
    }

    /// A frame-of-reference base means only the spread costs bytes, not the magnitude.
    #[test]
    fn shifting_a_narrow_column_far_from_zero_costs_only_the_base() {
        let near_zero: Vec<f64> = (0..256).map(|i| f64::from(i) * 0.25).collect();
        let shifted: Vec<f64> = near_zero.iter().map(|v| v + 52_500_000.0).collect();
        let here = column(&near_zero).encode(cfg_alp()).unwrap().len();
        let far = column(&shifted).encode(cfg_alp()).unwrap().len();
        assert!(far <= here + 8, "{far} vs {here}");
    }

    #[test]
    fn a_decimal_column_takes_alp_when_the_dictionary_is_off() {
        let values: Vec<f64> = (0..12).map(|i| f64::from(i) * 0.25).collect();
        let bytes = column(&values).encode(cfg_alp()).unwrap();
        assert!(matches!(
            float_encodings(&bytes)[..],
            [FloatLogical::Alp(_)]
        ));
    }

    #[test]
    fn alp_beats_the_dictionary_on_a_repetitive_decimal_column() {
        let bytes = column(&repeated_decimals(12)).encode(cfg_both()).unwrap();
        assert!(matches!(
            float_encodings(&bytes)[..],
            [FloatLogical::Alp(_)]
        ));
    }

    #[test]
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only f64 values are expected"
    )]
    fn coordinates_round_trip_bit_for_bit() {
        const VALUES: &[f64] = &[13.404_954, 52.520_008, -74.006, 40.712_776, 0.0, 180.0];
        let l = column(VALUES);
        let tile = round_trip(&l, cfg_alp());
        let bits: Vec<u64> = tile
            .features()
            .iter()
            .map(|f| match f.properties()[0] {
                PropValue::F64(Some(v)) => v.to_bits(),
                ref other => panic!("unexpected {other:?}"),
            })
            .collect();
        let expected: Vec<u64> = VALUES.iter().map(|v| v.to_bits()).collect();
        assert_eq!(bits, expected);
    }

    #[rstest]
    #[case::nan(f64::NAN)]
    #[case::infinity(f64::INFINITY)]
    #[case::negative_zero(-0.0)]
    fn one_value_alp_cannot_carry_keeps_the_whole_column_off_alp(#[case] odd: f64) {
        let mut values: Vec<f64> = (0..16).map(|i| f64::from(i) * 0.5).collect();
        values.push(odd);
        let l = column(&values);
        assert_eq!(
            value_bits(&round_trip(&l, cfg_alp())),
            value_bits(&round_trip(&l, cfg_v2()))
        );
    }

    #[test]
    fn an_optional_alp_column_round_trips() {
        let values: Vec<PropValue> = (0..24)
            .map(|i| PropValue::F64((i % 4 != 0).then(|| f64::from(i) * 0.125)))
            .collect();
        let l = layer(points(&"1".repeat(24)), None, &[("v", values)]);
        assert_eq!(round_trip(&l, cfg_alp()), round_trip(&l, cfg_v2()));
    }

    #[test]
    fn f32_columns_round_trip_through_alp() {
        let values: Vec<f32> = (0_i16..32).map(|i| f32::from(i) * 0.5 - 8.0).collect();
        let l = f32_column(&values);
        assert_eq!(round_trip(&l, cfg_alp()), round_trip(&l, cfg_v2()));
    }
}
