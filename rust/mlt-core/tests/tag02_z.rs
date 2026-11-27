//! Round-trip and wire-level tests for v2 z coordinates, the third word of each vertex.

use insta::assert_snapshot;
use mlt_core::dump::{DumpTree, RenderOpts, annotate_tile, render};
use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{
    Decoder, GeometryValues, HeightMode, Layer, MValue, Parser, PropKind, TileFeature, TileLayer,
    ZAxis, ZStep,
};
use rstest::rstest;

fn annotate(bytes: &[u8]) -> DumpTree {
    let (tree, err) = annotate_tile(bytes);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
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
    builder.set_z_axis(step()).unwrap();
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
    vec![ring(&[(20, 20), (20, 40), (40, 40), (40, 20), (20, 20)])],
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
    let bytes = assert_round_trips(&l, cfg_v2().with_tessellation(Tessellation::Always));
    assert_eq!(
        geometry_layout(&bytes),
        "geometry layout = TessPolygonsWithOutlines"
    );
}

#[test]
fn a_triangles_only_feature_carries_the_z_of_each_corner_in_decoded_order() {
    let l = z_layer(vec![Geometry::Polygon(square(0, 0))]);
    let bytes = l
        .encode(
            cfg_v2()
                .with_tessellation(Tessellation::Always)
                .with_triangles_only(true),
        )
        .unwrap();
    assert_eq!(geometry_layout(&bytes), "geometry layout = TessPolygons");
    let tile = decode(&bytes);
    assert_eq!(tile.z_axis(), Some(step().into()));
    assert_snapshot!(format!("{:?}", tile.features()[0].z()), @"[1, 8, 15, 1, 15, 22]");
    assert_round_trips(&tile, cfg_v2());
}

#[test]
fn repeated_triples_stay_plain() {
    let l = {
        let mut builder = TileLayer::builder("z", 4096).unwrap();
        builder.set_z_axis(step()).unwrap();
        for i in 0..200 {
            let mut feature = builder.feature(pt((i % 7) * 10, (i % 5) * 10));
            feature.z(vec![i % 3]).unwrap();
            feature.finish().unwrap();
        }
        builder.finish()
    };
    let bytes = assert_round_trips(&l, cfg_v2());
    assert_eq!(geometry_layout(&bytes), "geometry layout = Points");
}

#[test]
fn z_and_m_values_round_trip_together() {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_axis(step()).unwrap();
    let key = builder.add_m_value("m", PropKind::U32).unwrap();
    let mut feature = builder.feature(line(&[(0, 0), (10, 10), (20, 0)]));
    feature.z(vec![3, 2, 1]).unwrap();
    feature
        .m_value(key, MValue::U32(Some(vec![7, 8, 9])))
        .unwrap();
    feature.finish().unwrap();
    assert_round_trips(&builder.finish(), cfg_v2());
}

/// The vertex stream's header fields after its encoding byte, one region per line.
fn vertex_header_fields(bytes: &[u8]) -> String {
    let tree = annotate(bytes);
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
    fields.join("\n")
}

#[test]
fn the_vertex_stream_header_names_its_axis() {
    let bytes = z_layer(vec![pt(1, 2)]).encode(cfg_v2()).unwrap();
    assert_snapshot!(vertex_header_fields(&bytes), @"
    encoding = 0x09 logical=CwDelta physical=VarInt
      has_explicit_count = false -> 1 values from context
      logical = CwDelta, numbered for vertex stream
      physical = VarInt
      xyz = true -> (x, y, z) triples, a z axis byte follows
    byte_length = 3
    z_axis = 1 dm, absolute
    ");
}

#[test]
fn a_relative_to_ground_layer_sets_bit_3_of_the_z_axis_byte() {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    let axis = ZAxis {
        step: step(),
        mode: HeightMode::RelativeToGround,
    };
    builder.set_z_axis(axis).unwrap();
    let mut feature = builder.feature(pt(1, 2));
    feature.z(vec![120]).unwrap();
    feature.finish().unwrap();
    let layer = builder.finish();

    let bytes = assert_round_trips(&layer, cfg_v2());
    assert_eq!(decode(&bytes).z_axis(), Some(axis));
    assert_snapshot!(vertex_header_fields(&bytes), @"
    encoding = 0x09 logical=CwDelta physical=VarInt
      has_explicit_count = false -> 1 values from context
      logical = CwDelta, numbered for vertex stream
      physical = VarInt
      xyz = true -> (x, y, z) triples, a z axis byte follows
    byte_length = 4
    z_axis = 1 dm, relative to ground
    ");
}

/// A layer on `step` of `features`, each a geometry and the z of its stored vertices.
fn z_features(features: Vec<(Geometry<i32>, Vec<i32>)>) -> TileLayer {
    let mut builder = TileLayer::builder("z", 4096).unwrap();
    builder.set_z_axis(step()).unwrap();
    for (geom, z) in features {
        let mut feature = builder.feature(geom);
        feature.z(z).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

/// Lines of 20 vertices, each at one height 25 m above the one before.
fn contours() -> Vec<(Geometry<i32>, Vec<i32>)> {
    (0..30)
        .map(|i| {
            let pts: Vec<(i32, i32)> = (0..20)
                .map(|j| (i * 40 + j * 17 + (j * j) % 7, i * 30 + j * 11 - (j * 3) % 5))
                .collect();
            (line(&pts), vec![2500 + i * 250; 20])
        })
        .collect()
}

/// Squares whose corners rise towards the east and the south.
fn slopes() -> Vec<(Geometry<i32>, Vec<i32>)> {
    (0..40)
        .map(|i| {
            let (x, y) = ((i % 8) * 300, (i / 8) * 300);
            let z = [0, 120, 170, 50].map(|dz| 4000 + i * 13 + dz);
            (Geometry::Polygon(square(x, y)), z.to_vec())
        })
        .collect()
}

/// The vertex stream's encoding byte as the dump names it.
fn vertex_encoding(bytes: &[u8]) -> String {
    annotate(bytes)
        .regions
        .iter()
        .skip_while(|r| r.label != "vertices")
        .find(|r| r.label == "encoding")
        .and_then(|r| r.value.clone())
        .expect("a vertex stream encoding")
}

#[rstest]
#[case::contours(z_features(contours()))]
#[case::slopes(z_features(slopes()))]
#[case::mixed(z_layer((0..60).map(|i| if i % 2 == 0 {
    line(&[(i * 9, i * 4), (i * 9 + 30, i * 4 + 7), (i * 9 + 61, i * 4 + 15)])
} else {
    Geometry::Polygon(square(i * 20, 900))
}).collect()))]
fn predicted_vertices_shrink_a_z_layer_and_decode_unchanged(#[case] layer: TileLayer) {
    let plain = assert_round_trips(&layer, cfg_v2());
    let predicted = assert_round_trips(&layer, cfg_v2().with_predicted_vertices(true));
    assert_snapshot!(vertex_encoding(&predicted), @"0xC1 logical=Predicted physical=implied");
    assert!(
        predicted.len() < plain.len(),
        "{} vs {}",
        predicted.len(),
        plain.len()
    );
}

#[test]
fn a_tessellated_z_layer_races_predicted_vertices() {
    let layer = z_features(slopes());
    let tessellated = cfg_v2().with_tessellation(Tessellation::Always);
    let predicted = assert_round_trips(&layer, tessellated.with_predicted_vertices(true));
    assert_snapshot!(vertex_encoding(&predicted), @"0xC1 logical=Predicted physical=implied");
}

#[test]
fn a_predicted_vertex_stream_header_names_its_axis() {
    let bytes = z_features(contours())
        .encode(cfg_v2().with_predicted_vertices(true))
        .unwrap();
    assert_snapshot!(vertex_header_fields(&bytes), @"
    encoding = 0xC1 logical=Predicted physical=implied
      has_explicit_count = true -> a num_values varint follows
      logical = Predicted, numbered for vertex stream
      physical = implied
      xyz = true -> (x, y, z) triples, a z axis byte follows
    num_values = 600
    byte_length = 754
    z_axis = 1 dm, absolute
    ");
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
    assert_eq!(geometry.z_axis(), None);
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
    builder.set_z_axis(step()).unwrap();
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
    builder.set_z_axis(step()).unwrap();
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
    let err = layer.set_z_axis(step()).unwrap_err();
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

#[rstest]
#[case::metres(0)]
#[case::finest(ZStep::MIN_EXPONENT)]
#[case::coarsest(ZStep::MAX_EXPONENT)]
fn z_zero_is_mean_sea_level(#[case] exponent: i8) {
    let step = ZStep::new(exponent).unwrap();
    assert_eq!(step.z(0.0), Some(0));
    assert_eq!(step.elevation(0).to_bits(), 0.0_f64.to_bits());
}

#[test]
fn the_finest_step_spans_the_deepest_trench_to_the_highest_peak() {
    let finest = ZStep::new(ZStep::MIN_EXPONENT).unwrap();
    assert_eq!(finest.z(-10_994.0), Some(-10_994_000));
    assert_eq!(finest.z(8_848.86), Some(8_848_860));
}

#[rstest]
#[case::centimetres_read_back_exactly(-2, 1_234, 12.34)]
#[case::decimetres_read_back_exactly(-1, 88_489, 8_848.9)]
#[case::below_sea_level(-2, -43_050, -430.5)]
#[case::kilometres(3, 9, 9_000.0)]
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
#[case::sea_level_in_metres(0, 0.0, Some(0))]
#[case::everest_in_centimetres(-2, 8_848.86, Some(884_886))]
#[case::challenger_deep_in_metres(0, -10_994.0, Some(-10_994))]
#[case::past_i32(-3, 1.0e10, None)]
fn an_elevation_lands_on_its_nearest_grid_value(
    #[case] exponent: i8,
    #[case] metres: f64,
    #[case] z: Option<i32>,
) {
    assert_eq!(ZStep::new(exponent).unwrap().z(metres), z);
}

#[test]
fn a_dictionary_layout_over_componentwise_delta_triples_is_rejected() {
    let bytes =
        hex::decode("2302066c617965723110050189040c021a54640314140000147828140805000100020300")
            .unwrap();
    let err = Parser::default().parse_layers(&bytes).unwrap_err();
    assert_snapshot!(err, @"the vertex stream of a PointsDict layout must be Morton coded");
}
