//! Vertex sort reorders the vertices inside each feature so it starts near where the previous one ended.

use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation};
use mlt_core::geo_types::{
    Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{Decoder, Parser, TileLayer};
use proptest::prelude::*;

fn line(coords: &[(i32, i32)]) -> LineString<i32> {
    LineString::from(coords.to_vec())
}

fn polygon(exterior: &[(i32, i32)], holes: &[&[(i32, i32)]]) -> Polygon<i32> {
    Polygon::new(line(exterior), holes.iter().map(|h| line(h)).collect())
}

fn multi_point(points: &[(i32, i32)]) -> MultiPoint<i32> {
    MultiPoint(points.iter().map(|&p| Point::from(p)).collect())
}

fn round_trip(geometries: Vec<Geometry<i32>>, cfg: EncoderConfig) -> Vec<Geometry<i32>> {
    let mut builder = TileLayer::builder("l", 4096).unwrap();
    for geometry in geometries {
        builder.feature(geometry).finish().unwrap();
    }
    let bytes = builder.finish().encode(cfg).unwrap();
    let layer = Parser::default().parse_layers(&bytes).unwrap().remove(0);
    let decoded = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    decoded
        .features()
        .iter()
        .map(|f| f.geometry().clone())
        .collect()
}

fn greedy_chain(start: (i32, i32), points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let distance = |a: (i32, i32), b: (i32, i32)| {
        (i64::from(a.0) - i64::from(b.0)).abs() + (i64::from(a.1) - i64::from(b.1)).abs()
    };
    let mut left = points.to_vec();
    let mut at = start;
    let mut chain = Vec::with_capacity(points.len());
    while !left.is_empty() {
        let next = (0..left.len())
            .min_by_key(|&i| distance(at, left[i]))
            .unwrap();
        at = left.remove(next);
        chain.push(at);
    }
    chain
}

#[test]
fn a_ring_starts_at_its_vertex_nearest_the_previous_feature() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(90, 10).into(),
            polygon(&[(0, 0), (100, 0), (100, 100), (0, 100)], &[]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        polygon(&[(100, 0), (100, 100), (0, 100), (0, 0), (100, 0)], &[]).into()
    );
}

#[test]
fn a_clockwise_ring_is_wound_before_it_is_rotated() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(90, 10).into(),
            polygon(&[(0, 0), (0, 100), (100, 100), (100, 0)], &[]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        polygon(&[(100, 0), (100, 100), (0, 100), (0, 0), (100, 0)], &[]).into()
    );
}

#[test]
fn a_hole_starts_near_the_end_of_its_exterior() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(5, 95).into(),
            polygon(
                &[(0, 0), (100, 0), (100, 100), (0, 100)],
                &[&[(10, 10), (10, 20), (20, 20), (20, 10)]],
            )
            .into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        polygon(
            &[(0, 100), (0, 0), (100, 0), (100, 100), (0, 100)],
            &[&[(20, 20), (20, 10), (10, 10), (10, 20), (20, 20)]],
        )
        .into()
    );
}

#[test]
fn the_first_feature_starts_near_the_origin() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![polygon(&[(100, 100), (0, 100), (0, 0), (100, 0)], &[]).into()],
        vertex_sorted,
    );
    assert_eq!(
        decoded[0],
        polygon(&[(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)], &[]).into()
    );
}

#[test]
fn each_polygon_of_a_multi_polygon_starts_near_the_one_before() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(0, 0).into(),
            MultiPolygon(vec![
                polygon(&[(10, 0), (20, 0), (20, 10), (10, 10)], &[]),
                polygon(&[(30, 30), (40, 30), (40, 40), (30, 40)], &[]),
            ])
            .into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        MultiPolygon(vec![
            polygon(&[(10, 0), (20, 0), (20, 10), (10, 10), (10, 0)], &[]),
            polygon(&[(30, 30), (40, 30), (40, 40), (30, 40), (30, 30)], &[]),
        ])
        .into()
    );
}

#[test]
fn a_tessellated_ring_is_rotated_like_an_untessellated_one() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let tessellated = vertex_sorted.with_tessellation(Tessellation::Always);
    let geometries = vec![
        Point::new(90, 10).into(),
        polygon(&[(0, 0), (100, 0), (100, 100), (0, 100)], &[]).into(),
    ];
    assert_eq!(
        round_trip(geometries.clone(), tessellated),
        round_trip(geometries, vertex_sorted)
    );
}

#[test]
fn a_line_is_reversed_when_its_end_is_nearer_the_previous_feature() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(0, 0).into(),
            line(&[(100, 100), (50, 60), (10, 10)]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(decoded[1], line(&[(10, 10), (50, 60), (100, 100)]).into());
}

#[test]
fn a_line_with_both_ends_equally_near_keeps_its_direction() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![Point::new(50, 0).into(), line(&[(0, 0), (100, 0)]).into()],
        vertex_sorted,
    );
    assert_eq!(decoded[1], line(&[(0, 0), (100, 0)]).into());
}

#[test]
fn each_part_of_a_multi_line_string_is_reversed_on_its_own_in_place() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(0, 0).into(),
            MultiLineString(vec![
                line(&[(50, 50), (10, 10)]),
                line(&[(60, 60), (100, 100)]),
                line(&[(500, 500), (110, 110)]),
            ])
            .into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        MultiLineString(vec![
            line(&[(10, 10), (50, 50)]),
            line(&[(60, 60), (100, 100)]),
            line(&[(110, 110), (500, 500)]),
        ])
        .into()
    );
}

#[test]
fn a_multi_point_walks_to_the_nearest_unvisited_point() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(0, 0).into(),
            multi_point(&[(30, 0), (10, 0), (5, 40), (20, 0)]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        multi_point(&[(10, 0), (20, 0), (30, 0), (5, 40)]).into()
    );
}

#[test]
fn equally_near_points_keep_their_order() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(0, 0).into(),
            multi_point(&[(10, 0), (0, 10)]).into(),
            multi_point(&[(0, 10), (10, 0)]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1..],
        [
            multi_point(&[(10, 0), (0, 10)]).into(),
            multi_point(&[(0, 10), (10, 0)]).into(),
        ]
    );
}

#[test]
fn without_vertex_sort_every_vertex_keeps_its_place() {
    let source_order = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(false);
    let geometries: Vec<Geometry<i32>> = vec![
        Point::new(90, 10).into(),
        polygon(&[(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)], &[]).into(),
        line(&[(100, 100), (10, 10)]).into(),
        multi_point(&[(30, 0), (10, 0)]).into(),
    ];
    assert_eq!(round_trip(geometries.clone(), source_order), geometries);
}

proptest! {
    #[test]
    fn a_multi_point_follows_the_greedy_nearest_neighbour_chain(
        start in (-10_000..10_000, -10_000..10_000),
        points in prop::collection::vec((-10_000..10_000, -10_000..10_000), 1..400),
        spread in 0..14_u32,
    ) {
        let points: Vec<(i32, i32)> = points.iter().map(|&(x, y)| (x >> spread, y >> spread)).collect();
        let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
            .with_tessellation(Tessellation::Off)
            .with_vertex_sort(true);
        let decoded = round_trip(
            vec![Point::from(start).into(), multi_point(&points).into()],
            vertex_sorted,
        );
        let expected: Geometry<i32> = multi_point(&greedy_chain(start, &points)).into();
        prop_assert_eq!(&decoded[1], &expected);
    }
}

#[test]
fn a_large_multi_point_follows_the_greedy_nearest_neighbour_chain() {
    let mut seed = 0x2545_f491_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        i32::try_from(seed % 4096).unwrap()
    };
    let points: Vec<(i32, i32)> = (0..5000).map(|_| (next(), next())).collect();
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![Point::new(4000, 0).into(), multi_point(&points).into()],
        vertex_sorted,
    );
    let expected: Geometry<i32> = multi_point(&greedy_chain((4000, 0), &points)).into();
    assert_eq!(decoded[1], expected);
}

#[test]
fn a_multi_point_of_100_000_copies_of_two_points_visits_each_point_once() {
    let points: Vec<(i32, i32)> = (0..100_000)
        .map(|i| if i % 2 == 0 { (9, 9) } else { (0, 0) })
        .collect();
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![Point::new(0, 0).into(), multi_point(&points).into()],
        vertex_sorted,
    );
    let mut expected = vec![(0, 0); 50_000];
    expected.extend(vec![(9, 9); 50_000]);
    assert_eq!(decoded[1], multi_point(&expected).into());
}

#[test]
fn a_two_vertex_ring_is_not_rotated() {
    let vertex_sorted = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_tessellation(Tessellation::Off)
        .with_vertex_sort(true);
    let decoded = round_trip(
        vec![
            Point::new(100, 100).into(),
            polygon(&[(0, 0), (100, 100), (0, 0)], &[]).into(),
        ],
        vertex_sorted,
    );
    assert_eq!(
        decoded[1],
        polygon(&[(0, 0), (100, 100), (0, 0)], &[]).into()
    );
}
