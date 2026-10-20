//! The geometry each iterated feature borrows equals the one decoded on its own.

use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation};
use mlt_core::geo_types::{
    Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{Decoder, LendingIterator as _, ParsedLayer, Parser, TileLayer};
use rstest::rstest;

fn line(coords: &[(i32, i32)]) -> LineString<i32> {
    LineString::from(coords.to_vec())
}

fn polygon(exterior: &[(i32, i32)], holes: &[&[(i32, i32)]]) -> Polygon<i32> {
    Polygon::new(line(exterior), holes.iter().map(|h| line(h)).collect())
}

fn alternating_geometries() -> Vec<Geometry<i32>> {
    let square = [(0, 0), (100, 0), (100, 100), (0, 100)];
    let hole_a = [(10, 10), (10, 20), (20, 20), (20, 10)];
    let hole_b = [(30, 30), (30, 40), (40, 40), (40, 30)];
    let small = [(200, 200), (210, 200), (210, 210)];
    vec![
        Geometry::Polygon(polygon(&square, &[&hole_a, &hole_b])),
        Geometry::Polygon(polygon(&square, &[&hole_b])),
        Geometry::Polygon(polygon(&small, &[])),
        Geometry::Polygon(polygon(&square, &[&hole_a, &hole_b])),
        Geometry::Point(Point::new(5, 6)),
        Geometry::MultiLineString(MultiLineString(vec![
            line(&[(0, 0), (1, 1), (2, 0)]),
            line(&[(10, 10), (11, 11)]),
            line(&[(20, 20), (21, 21), (22, 22), (23, 23)]),
        ])),
        Geometry::MultiLineString(MultiLineString(vec![line(&[(5, 5), (6, 6)])])),
        Geometry::MultiLineString(MultiLineString(vec![
            line(&[(7, 7), (8, 8), (9, 9)]),
            line(&[(30, 30), (31, 31)]),
        ])),
        Geometry::LineString(line(&[(1, 2), (3, 4), (5, 6), (7, 8)])),
        Geometry::LineString(line(&[(9, 9), (10, 10)])),
        Geometry::MultiPolygon(MultiPolygon(vec![
            polygon(&square, &[&hole_a]),
            polygon(&small, &[]),
        ])),
        Geometry::MultiPolygon(MultiPolygon(vec![polygon(&small, &[])])),
        Geometry::MultiPolygon(MultiPolygon(vec![
            polygon(&small, &[]),
            polygon(&square, &[&hole_a, &hole_b]),
            polygon(&square, &[]),
        ])),
        Geometry::MultiPoint(MultiPoint(vec![
            Point::new(1, 1),
            Point::new(2, 2),
            Point::new(3, 3),
        ])),
        Geometry::MultiPoint(MultiPoint(vec![Point::new(4, 4)])),
        Geometry::Point(Point::new(7, 8)),
    ]
}

#[rstest]
#[case::outlines_only(Tessellation::Off)]
#[case::with_triangles(Tessellation::Always)]
fn iterated_geometries_match_to_geojson_across_alternating_types_and_part_counts(
    #[case] tessellation: Tessellation,
) {
    let input = alternating_geometries();
    let mut builder = TileLayer::builder("mixed", 4096).unwrap();
    for geom in &input {
        builder.feature(geom.clone()).finish().unwrap();
    }
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(tessellation);
    let bytes = builder.finish().encode(cfg).unwrap();

    let mut dec = Decoder::default();
    let mut layers = Parser::default().parse_layers(&bytes).unwrap();
    let parsed = layers.remove(0).decode_all(&mut dec).unwrap();
    let ParsedLayer::Tag01(layer) = parsed else {
        panic!("expected a v1 layer")
    };

    let mut iterated = Vec::new();
    let mut iter = layer.iter_features();
    while let Some(feature) = iter.next() {
        iterated.push(feature.unwrap().geometry().clone());
    }
    let one_by_one: Vec<_> = (0..input.len())
        .map(|i| layer.geometry_values().to_geojson(i).unwrap())
        .collect();

    assert_eq!(iterated, one_by_one);
    assert_eq!(iterated, input);
}
