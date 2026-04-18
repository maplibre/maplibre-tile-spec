use std::collections::HashSet;

use geo_types::{LineString, Point, Polygon, point, wkt};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::encoder::Encoder;
use crate::test_helpers::{assert_empty, dec, parser};
use crate::{
    Coord32, Decode as _, DictionaryType, Geom32, GeometryValues, LengthType, OffsetType,
    RawGeometry, StreamType,
};

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
#[case::polygon(push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()]))]
#[case::multi_polygon(push_geoms(&[wkt!(MULTIPOLYGON(((0 0, 10 0, 10 10, 0 0),(5 5, 15 5, 15 15, 5 15)))).into()]))]
fn automatic_optimization_roundtrip(#[case] decoded: GeometryValues) {
    let mut enc = Encoder::default();
    decoded.clone().write_to(&mut enc).expect("optimize failed");
    assert_geometry_roundtrip(&enc.data, &decoded);
}

#[rstest]
#[case::single_point_vec2(
    push_geoms(&[(0i32..10).map(|i| point!{ x: i, y: i }.into()).collect::<Vec<_>>()].concat()),
    DictionaryType::Vertex
)]
#[case::repeated_points_morton(
    push_geoms(&std::iter::repeat_n(point!{ x: 5, y: 5 }.into(), 20).collect::<Vec<_>>()),
    DictionaryType::Morton
)]
fn automatic_optimization_picks_correct_vertex_strategy(
    #[case] decoded: GeometryValues,
    #[case] expected: DictionaryType,
) {
    let unexpected = if expected == DictionaryType::Vertex {
        DictionaryType::Morton
    } else {
        DictionaryType::Vertex
    };

    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");
    let types = encoded_stream_types(&enc.data);

    assert!(
        types.contains(&StreamType::Data(expected)),
        "expected {expected:?} stream to be present"
    );
    assert!(
        !types.contains(&StreamType::Data(unexpected)),
        "expected {unexpected:?} stream to be absent"
    );
}

#[test]
fn encoded_output_always_has_meta_stream() {
    let decoded = push_geoms(&[Geom32::Point(Point(Coord32 { x: 1, y: 1 }))]);
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");
    let raw = assert_empty(RawGeometry::from_bytes(&enc.data, &mut parser()));

    assert_eq!(
        raw.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta (VarBinary) stream must always be present"
    );
}

#[test]
fn encoded_polygon_has_topology_streams() {
    let coords: Vec<Coord32> = [(0, 0), (10, 0), (10, 10), (0, 0)]
        .into_iter()
        .map(|(x, y)| Coord32 { x, y })
        .collect();
    let decoded = push_geoms(&[Geom32::Polygon(Polygon::new(LineString(coords), vec![]))]);
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    assert!(
        stream_types.contains(&StreamType::Length(LengthType::Rings))
            || stream_types.contains(&StreamType::Length(LengthType::Parts)),
        "polygon must produce at least a Parts or Rings length stream"
    );
}

#[test]
fn encoded_repeated_points_uses_morton_streams() {
    // All vertices identical: uniqueness ratio = 1/3 < 0.5, so optimizer picks Morton.
    let mut decoded = GeometryValues::default();
    decoded.push_geom(&wkt!(MULTIPOINT(5 5, 5 5, 5 5)).into());
    let mut enc = Encoder::default();
    decoded.write_to(&mut enc).expect("encode failed");

    let stream_types = encoded_stream_types(&enc.data);
    assert!(
        stream_types.contains(&StreamType::Data(DictionaryType::Morton)),
        "repeated vertices must trigger Morton dictionary encoding"
    );
    assert!(
        stream_types.contains(&StreamType::Offset(OffsetType::Vertex)),
        "Morton encoding must include a vertex offset stream"
    );

    let raw = assert_empty(RawGeometry::from_bytes(&enc.data, &mut parser()));
    assert_eq!(
        raw.meta.meta.stream_type,
        StreamType::Length(LengthType::VarBinary),
        "meta stream must always be present"
    );
}

#[test]
fn manual_encode_works() {
    let decoded = push_geoms(&[wkt!(POINT(10 20)).into()]);

    let mut enc = Encoder::default();
    decoded.clone().write_to(&mut enc).expect("encode failed");
    let types = encoded_stream_types(&enc.data);
    assert!(types.contains(&StreamType::Data(DictionaryType::Vertex)));

    assert_geometry_roundtrip(&enc.data, &decoded);
}

/// Round-trip geometry bytes: parse then decode and compare.
fn assert_geometry_roundtrip(data: &[u8], expected: &GeometryValues) {
    let mut p = parser();
    let mut d = dec();
    let raw = assert_empty(RawGeometry::from_bytes(data, &mut p));
    let result = raw.decode(&mut d).unwrap();
    assert!(
        d.consumed() > 0,
        "decoder should consume bytes after decode"
    );
    assert_eq!(expected, &result);
}

fn push_geoms(geoms: &[Geom32]) -> GeometryValues {
    let mut d = GeometryValues::default();
    for g in geoms {
        d.push_geom(g);
    }
    d
}

/// Collect all stream types present in the encoded geometry bytes (meta + items).
fn encoded_stream_types(data: &[u8]) -> HashSet<StreamType> {
    let raw = assert_empty(RawGeometry::from_bytes(data, &mut parser()));
    std::iter::once(raw.meta.meta.stream_type)
        .chain(raw.items.iter().map(|s| s.meta.stream_type))
        .collect()
}
