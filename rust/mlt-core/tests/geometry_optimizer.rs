use std::collections::HashSet;

use geo_types::point;
use geo_types::{LineString, Point, Polygon, wkt};
use mlt_core::FromEncoded as _;
use mlt_core::borrowme;
use mlt_core::geojson::{Coord32, Geom32};
use mlt_core::v01::{
    DecodedGeometry, DictionaryType, GeometryOptimizer, LengthType, OffsetType,
    OwnedEncodedGeometry, StreamType,
};
use pretty_assertions::assert_eq;

fn optimize_roundtrip(decoded: &DecodedGeometry) -> DecodedGeometry {
    let encoded =
        GeometryOptimizer::optimize_and_encode(decoded).expect("optimize_and_encode failed");
    DecodedGeometry::from_encoded(borrowme::borrow(&encoded)).expect("from_encoded failed")
}

fn push_geoms(geoms: &[Geom32]) -> DecodedGeometry {
    let mut d = DecodedGeometry::default();
    for g in geoms {
        d.push_geom(g);
    }
    d
}

/// Collect all stream types present in an encoded geometry (meta + items).
fn encoded_stream_types(encoded: &OwnedEncodedGeometry) -> HashSet<StreamType> {
    std::iter::once(encoded.meta.meta.stream_type)
        .chain(encoded.items.iter().map(|s| s.meta.stream_type))
        .collect()
}

#[test]
fn roundtrip_single_point() {
    let geoms = vec![wkt!(POINT(10 20)).into()];
    let original = push_geoms(&geoms);
    let canonical = optimize_roundtrip(&original);
    let output = optimize_roundtrip(&canonical);
    assert_eq!(canonical, output);
}

#[test]
fn roundtrip_linestring() {
    let geoms = vec![wkt!(LINESTRING(10 20, 30 40, 50 60)).into()];
    let original = push_geoms(&geoms);
    let canonical = optimize_roundtrip(&original);
    let output = optimize_roundtrip(&canonical);
    assert_eq!(canonical, output);
}

#[test]
fn roundtrip_polygon() {
    let geoms = vec![wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()];
    let original = push_geoms(&geoms);
    let canonical = optimize_roundtrip(&original);
    let output = optimize_roundtrip(&canonical);
    assert_eq!(canonical, output);
}

#[test]
fn roundtrip_mixed_geometry_types() {
    let point = wkt!(POINT(1 2)).into();
    let ls = wkt!(LINESTRING(0 0, 1 1, 2 2)).into();
    let poly = wkt!(POLYGON((0 0, 10 0, 10 10, 0 0))).into();
    let original = push_geoms(&[point, ls, poly]);
    let canonical = optimize_roundtrip(&original);
    let output = optimize_roundtrip(&canonical);
    assert_eq!(canonical, output);
}

#[test]
fn roundtrip_multi_polygon() {
    let geoms = vec![
        wkt!(MULTIPOLYGON((
          (0 0, 10 0, 10 10, 0 0),
          (5 5, 15 5, 15 15, 5 15)
        )))
        .into(),
    ];
    let original = push_geoms(&geoms);
    let canonical = optimize_roundtrip(&original);
    let output = optimize_roundtrip(&canonical);
    assert_eq!(canonical, output);
}

#[test]
fn vertex_strategy_all_unique_prefers_vec2() {
    // 10 distinct points - uniqueness ratio = 1.0 ≥ 0.5 → Vec2.
    let geoms: Vec<Geom32> = (0i32..10).map(|i| point! { x: i, y: i }.into()).collect();
    let encoded =
        GeometryOptimizer::optimize_and_encode(&push_geoms(&geoms)).expect("encode failed");
    let types = encoded_stream_types(&encoded);
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "all-unique vertices must use Vec2 (Vertex) encoding"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Morton)),
        "all-unique vertices must not use Morton encoding"
    );
}

#[test]
fn vertex_strategy_high_repetition_prefers_morton() {
    // All 20 points share the same coordinate — uniqueness ratio = 1/20 < 0.5 → Morton.
    let geoms: Vec<Geom32> = std::iter::repeat_n(point! { x: 5, y: 5 }.into(), 20).collect();
    let encoded =
        GeometryOptimizer::optimize_and_encode(&push_geoms(&geoms)).expect("encode failed");
    let types = encoded_stream_types(&encoded);
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Morton)),
        "highly repeated vertices must use Morton encoding"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "highly repeated vertices must not use Vec2 encoding"
    );
}

#[test]
fn vertex_strategy_out_of_morton_range_falls_back_to_vec2() {
    // Coordinates exceed 16-bit range (> 65535), so Morton is ruled out entirely.
    let geoms: Vec<Geom32> = std::iter::repeat_n(
        Geom32::Point(Point(Coord32 {
            x: 0x1_0000,
            y: 0x1_0000,
        })),
        50,
    )
    .collect();
    let encoded =
        GeometryOptimizer::optimize_and_encode(&push_geoms(&geoms)).expect("encode failed");
    let types = encoded_stream_types(&encoded);
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "out-of-range coordinates must fall back to Vec2 encoding"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Morton)),
        "out-of-range coordinates must not use Morton encoding"
    );
}

#[test]
fn encoded_output_always_has_meta_stream() {
    let geoms = vec![Geom32::Point(Point(Coord32 { x: 1, y: 1 }))];
    let decoded = push_geoms(&geoms);
    let encoded =
        GeometryOptimizer::optimize_and_encode(&decoded).expect("optimize_and_encode failed");
    assert_eq!(
        encoded.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta (VarBinary) stream must always be present"
    );
}

#[test]
fn encoded_polygon_has_topology_streams() {
    let coords: Vec<Coord32> = vec![(0, 0), (10, 0), (10, 10), (0, 0)]
        .into_iter()
        .map(|(x, y)| Coord32 { x, y })
        .collect();
    let geoms = vec![Geom32::Polygon(Polygon::new(LineString(coords), vec![]))];
    let decoded = push_geoms(&geoms);
    let encoded =
        GeometryOptimizer::optimize_and_encode(&decoded).expect("optimize_and_encode failed");
    let stream_types = encoded_stream_types(&encoded);
    assert!(
        stream_types.contains(&StreamType::Length(LengthType::Rings))
            || stream_types.contains(&StreamType::Length(LengthType::Parts)),
        "polygon must produce at least a Parts or Rings length stream"
    );
}

#[test]
fn encoded_repeated_points_uses_morton_streams() {
    // All vertices identical: uniqueness ratio = 1/3 < 0.5, so optimizer picks Morton.
    let mut decoded = DecodedGeometry::default();
    let pts = wkt!(MULTIPOINT(5 5, 5 5, 5 5));
    decoded.push_geom(&pts.into());
    let encoded =
        GeometryOptimizer::optimize_and_encode(&decoded).expect("optimize_and_encode failed");
    let stream_types = encoded_stream_types(&encoded);
    assert!(
        stream_types.contains(&StreamType::Data(DictionaryType::Morton)),
        "repeated vertices must trigger Morton dictionary encoding"
    );
    assert!(
        stream_types.contains(&StreamType::Offset(OffsetType::Vertex)),
        "Morton encoding must include a vertex offset stream"
    );
    assert_eq!(
        encoded.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta stream must always be present"
    );
}
