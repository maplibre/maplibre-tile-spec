//! Round-trip and wire-level tests for v2 z coordinates, the third word of each vertex.

use insta::assert_snapshot;
use mlt_core::dump::{DumpTree, RenderOpts, annotate_tile, render};
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{
    Decoder, GeometryValues, Layer, MValue, Parser, PropKind, TileFeature, TileLayer, ZStep,
};
use rstest::rstest;

fn annotate(bytes: &[u8]) -> DumpTree {
    let (tree, err) = annotate_tile(bytes);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::default().with_wire_version(WireVersion::V02)
}

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1);
    let Layer::Tag02(layer) = layers.into_iter().next().expect("a layer") else {
        panic!("expected a v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).expect("into_tile")
}

fn assert_dump_covers(bytes: &[u8]) {
    let tree = annotate(bytes);
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
    assert_eq!(cursor, bytes.len());
    render(&tree, bytes, &RenderOpts::default(), &mut Vec::new()).expect("render");
}

fn assert_round_trips(layer: &TileLayer, cfg: EncoderConfig) -> Vec<u8> {
    let bytes = layer.encode(cfg).expect("v2 encode");
    assert_eq!(&decode(&bytes), layer);
    assert_dump_covers(&bytes);
    bytes
}

/// The geometry layout the layer's layout byte names.
fn geometry_layout(bytes: &[u8]) -> String {
    annotate(bytes)
        .regions
        .iter()
        .filter(|r| r.label == "layout")
        .flat_map(|r| r.bits.iter().map(|b| b.meaning().to_string()))
        .last()
        .expect("a layout byte")
}

fn step() -> ZStep {
    ZStep::new(-1).unwrap()
}

/// A layer on `step` whose features count their z up from `1`, one per stored vertex.
fn z_layer(geoms: Vec<Geometry<i32>>) -> TileLayer {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_step(step()).unwrap();
    let mut next = 0;
    for geom in geoms {
        let n = TileFeature::new(geom.clone()).vertex_count();
        let z = (next..next + n)
            .map(|i| 1 + i32::try_from(i).unwrap() * 7)
            .collect();
        next += n;
        let mut feature = builder.feature(geom);
        feature.z(z).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

fn pt(x: i32, y: i32) -> Geometry<i32> {
    Geometry::Point(Point::new(x, y))
}

fn ring(pts: &[(i32, i32)]) -> LineString<i32> {
    LineString(pts.iter().map(|&(x, y)| Coord { x, y }).collect())
}

fn line(pts: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(ring(pts))
}

fn square(x: i32, y: i32) -> Polygon<i32> {
    Polygon::new(
        ring(&[(x, y), (x + 10, y), (x + 10, y + 10), (x, y + 10), (x, y)]),
        vec![],
    )
}

#[rstest]
#[case::points(vec![pt(1, 2), pt(3, 4), pt(5, 6)])]
#[case::lines(vec![line(&[(0, 0), (10, 10), (20, 0)]), line(&[(5, 5), (6, 6)])])]
#[case::polygon_with_hole(vec![Geometry::Polygon(Polygon::new(
    ring(&[(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)]),
    vec![ring(&[(20, 20), (40, 20), (40, 40), (20, 40), (20, 20)])],
))])]
#[case::multi_points(vec![Geometry::MultiPoint(MultiPoint(vec![Point::new(1, 2), Point::new(3, 4)]))])]
#[case::multi_lines(vec![Geometry::MultiLineString(MultiLineString(vec![
    ring(&[(0, 0), (1, 1)]),
    ring(&[(2, 2), (3, 3), (4, 2)]),
]))])]
#[case::multi_polygons(vec![Geometry::MultiPolygon(MultiPolygon(vec![square(0, 0), square(20, 20)]))])]
#[case::mixed(vec![pt(5, 5), line(&[(0, 0), (10, 10)]), Geometry::Polygon(square(40, 40))])]
fn z_round_trips_on_every_geometry_type(#[case] geoms: Vec<Geometry<i32>>) {
    assert_round_trips(&z_layer(geoms), cfg_v2());
}

#[test]
fn z_round_trips_on_tessellated_outlines() {
    let l = z_layer(vec![
        Geometry::Polygon(square(0, 0)),
        Geometry::MultiPolygon(MultiPolygon(vec![square(20, 20), square(40, 40)])),
        line(&[(0, 0), (5, 5)]),
    ]);
    let bytes = assert_round_trips(&l, cfg_v2().with_tessellation(true));
    assert_eq!(
        geometry_layout(&bytes),
        "geometry layout = TessPolygonsWithOutlines"
    );
}

#[test]
fn a_triangles_only_feature_carries_the_z_of_each_corner_in_index_order() {
    let l = z_layer(vec![Geometry::Polygon(square(0, 0))]);
    let bytes = l
        .encode(cfg_v2().with_tessellation(true).with_triangles_only(true))
        .unwrap();
    assert_eq!(geometry_layout(&bytes), "geometry layout = TessPolygons");
    let tile = decode(&bytes);
    assert_eq!(tile.z_step(), Some(step()));
    assert_snapshot!(format!("{:?}", tile.features()[0].z()), @"[15, 22, 1, 1, 8, 15]");
    assert_round_trips(&tile, cfg_v2());
}

#[test]
fn repeated_vertices_pick_a_dictionary() {
    let l = {
        let mut builder = TileLayer::builder("z", 4096).unwrap();
        builder.set_z_step(step()).unwrap();
        for i in 0..200 {
            let mut feature = builder.feature(pt((i % 7) * 10, (i % 5) * 10));
            feature.z(vec![i % 3]).unwrap();
            feature.finish().unwrap();
        }
        builder.finish()
    };
    let bytes = assert_round_trips(&l, cfg_v2());
    assert_eq!(geometry_layout(&bytes), "geometry layout = PointsDict");
}

#[test]
fn distinct_vertices_stay_plain() {
    let l = z_layer((0..64).map(|i| pt(i * 7, i * 13)).collect());
    let bytes = assert_round_trips(&l, cfg_v2());
    assert_eq!(geometry_layout(&bytes), "geometry layout = Points");
}

#[test]
fn z_and_m_values_round_trip_together() {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_step(step()).unwrap();
    let key = builder.add_m_value("m", PropKind::U32).unwrap();
    let mut feature = builder.feature(line(&[(0, 0), (10, 10), (20, 0)]));
    feature.z(vec![3, 2, 1]).unwrap();
    feature
        .m_value(key, MValue::U32(Some(vec![7, 8, 9])))
        .unwrap();
    feature.finish().unwrap();
    assert_round_trips(&builder.finish(), cfg_v2());
}

#[test]
fn the_vertex_stream_header_names_its_step() {
    let bytes = z_layer(vec![pt(1, 2)]).encode(cfg_v2()).unwrap();
    let tree = annotate(&bytes);
    let fields: Vec<String> = tree
        .regions
        .iter()
        .skip_while(|r| r.label != "vertices")
        .skip(1)
        .take_while(|r| r.label != "data")
        .flat_map(|r| {
            std::iter::once(format!(
                "{} = {}",
                r.label,
                r.value.clone().unwrap_or_default()
            ))
            .chain(r.bits.iter().map(|b| format!("  {}", b.meaning())))
        })
        .collect();
    assert_snapshot!(fields.join("\n"));
}

#[test]
fn geometry_values_take_z_only_once() {
    let mut geometry = GeometryValues::default();
    geometry.push_geom(&pt(1, 2));
    geometry.add_z(step(), &[7]).unwrap();
    let err = geometry.add_z(step(), &[7]).unwrap_err();
    assert_snapshot!(err, @"the vertices already carry z coordinates");
}

#[test]
fn geometry_values_take_one_z_per_vertex() {
    let mut geometry = GeometryValues::default();
    geometry.push_geom(&line(&[(0, 0), (1, 1)]));
    let err = geometry.add_z(step(), &[7]).unwrap_err();
    assert_snapshot!(err, @"a feature carries 1 z coordinates, but the layer expects 2");
    assert_eq!(geometry.z_step(), None);
}

#[test]
fn v1_rejects_a_layer_with_z() {
    let err = z_layer(vec![pt(1, 2)])
        .encode(cfg_v2().with_wire_version(WireVersion::V01))
        .unwrap_err();
    assert_snapshot!(err, @"z coordinates are a v2 feature, so layer z cannot be written as v1");
}

#[test]
fn a_feature_with_the_wrong_number_of_z_is_rejected() {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_step(step()).unwrap();
    let err = builder
        .feature(line(&[(0, 0), (1, 1)]))
        .z(vec![1])
        .map(|_| ())
        .unwrap_err();
    assert_snapshot!(err, @"a feature carries 1 z coordinates, but the layer expects 2");
}

#[test]
fn a_feature_without_z_is_rejected_on_a_z_layer() {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_step(step()).unwrap();
    let err = builder.feature(pt(1, 2)).finish().unwrap_err();
    assert_snapshot!(err, @"a feature carries 0 z coordinates, but the layer expects 1");
}

#[test]
fn a_feature_with_z_is_rejected_on_a_flat_layer() {
    let mut layer = TileLayer::new("flat", 4096).unwrap();
    let mut feature = TileFeature::new(pt(1, 2));
    feature.set_z(vec![5]).unwrap();
    let err = layer.push_feature(feature).unwrap_err();
    assert_snapshot!(err, @"a feature carries 1 z coordinates, but the layer expects 0");
}

#[test]
fn a_layer_holding_features_cannot_take_a_step() {
    let mut layer = TileLayer::new("flat", 4096).unwrap();
    layer.push_feature(TileFeature::new(pt(1, 2))).unwrap();
    let err = layer.set_z_step(step()).unwrap_err();
    assert_snapshot!(err, @"layer flat already holds features without z coordinates, so it cannot be given a z step");
}

#[rstest]
#[case::finer_than_a_millimetre(-4, "a z step of 10^-4 m is outside 10^-3..=10^4 m")]
#[case::coarser_than_10_km(5, "a z step of 10^5 m is outside 10^-3..=10^4 m")]
fn a_step_outside_the_range_is_rejected(#[case] exponent: i8, #[case] expected: &str) {
    assert_eq!(ZStep::new(exponent).unwrap_err().to_string(), expected);
}

#[rstest]
#[case::finest(-3, 0.001)]
#[case::metre(0, 1.0)]
#[case::coarsest(4, 10_000.0)]
fn a_step_spans_its_power_of_ten_in_metres(#[case] exponent: i8, #[case] metres: f64) {
    assert_eq!(
        ZStep::new(exponent).unwrap().metres().to_bits(),
        metres.to_bits()
    );
}

#[test]
fn z_zero_is_terrain_rgb_base() {
    assert_eq!(ZStep::new(0).unwrap().z(-10_000.0), Some(0));
}

#[test]
fn a_decimetre_step_is_terrain_rgb_grid() {
    let terrain_rgb_max = (1 << 24) - 1;
    let decimetres = ZStep::new(-1).unwrap();
    assert_eq!(
        decimetres.elevation(terrain_rgb_max).to_bits(),
        1_667_721.5_f64.to_bits()
    );
    assert_eq!(decimetres.z(1_667_721.5), Some(terrain_rgb_max));
}

#[test]
fn the_finest_step_spans_terrain_rgb() {
    let finest = ZStep::new(ZStep::MIN_EXPONENT).unwrap();
    assert!(finest.elevation(i32::MAX) > 1_667_721.5);
    assert_eq!(finest.z(1_667_721.5), Some(1_677_721_500));
}

#[rstest]
#[case::centimetres_read_back_exactly(-2, 1_001_234, 12.34)]
#[case::decimetres_read_back_exactly(-1, 188_489, 8_848.9)]
#[case::kilometres(3, 19, 9_000.0)]
fn a_grid_value_reads_as_its_decimal_elevation(
    #[case] exponent: i8,
    #[case] z: i32,
    #[case] metres: f64,
) {
    let elevation = ZStep::new(exponent).unwrap().elevation(z);
    assert_eq!(
        elevation.to_bits(),
        metres.to_bits(),
        "{elevation} != {metres}"
    );
}

#[rstest]
#[case::sea_level_in_metres(0, 0.0, Some(10_000))]
#[case::everest_in_centimetres(-2, 8_848.86, Some(1_884_886))]
#[case::below_the_base(0, -10_994.0, Some(-994))]
#[case::past_i32(-3, 1.0e10, None)]
fn an_elevation_lands_on_its_nearest_grid_value(
    #[case] exponent: i8,
    #[case] metres: f64,
    #[case] z: Option<i32>,
) {
    assert_eq!(ZStep::new(exponent).unwrap().z(metres), z);
}
