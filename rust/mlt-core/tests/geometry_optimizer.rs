use std::collections::HashSet;

use geo_types::{LineString, Point, Polygon, point, wkt};
use mlt_core::geojson::{Coord32, Geom32};
use mlt_core::test_helpers::{dec, parser};
use mlt_core::v01::{
    DictionaryType, EncodedGeometry, GeometryProfile, GeometryValues, LengthType, OffsetType,
    RawGeometry, StreamType,
};
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
#[case::polygon(push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()]))]
#[case::multi_polygon(push_geoms(&[wkt!(MULTIPOLYGON(((0 0, 10 0, 10 10, 0 0),(5 5, 15 5, 15 15, 5 15)))).into()]))]
fn automatic_optimisation_roundtrip(#[case] decoded: GeometryValues) {
    let (encoded, _) = decoded.clone().encode_auto().expect("optimize failed");
    assert_geometry_roundtrip(&encoded, &decoded);
}

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
fn profile_optimisation_roundtrip(#[case] decoded: GeometryValues) {
    let profile = GeometryProfile::from_sample(&decoded).expect("from_sample failed");
    let (encoded, _) = decoded
        .encode_with_profile(&profile)
        .expect("profile_driven_optimisation failed");
    assert_geometry_roundtrip(&encoded, &decoded);
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
fn automatic_optimisation_picks_correct_vertex_strategy(
    #[case] decoded: GeometryValues,
    #[case] expected: DictionaryType,
) {
    let unexpected = if expected == DictionaryType::Vertex {
        DictionaryType::Morton
    } else {
        DictionaryType::Vertex
    };

    let (encoded, _) = decoded.encode_auto().expect("encode failed");
    let types = encoded_stream_types(&encoded);

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
    let (encoded, _) = decoded.encode_auto().expect("encode failed");

    assert_eq!(
        encoded.meta.meta.stream_type,
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
    let (encoded, _) = decoded.encode_auto().expect("encode failed");

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
    let mut decoded = GeometryValues::default();
    decoded.push_geom(&wkt!(MULTIPOINT(5 5, 5 5, 5 5)).into());
    let (encoded, _) = decoded.encode_auto().expect("encode failed");

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

#[test]
fn profile_applied_to_different_tile_roundtrip() {
    // Build the profile from a polygon tile, apply it to a linestring tile.
    // The topology is completely different — apply_profile must still produce a
    // valid encoding because it re-runs the probe pass on the actual tile data.
    let sample = push_geoms(&[wkt!(POLYGON((0 0, 50 0, 50 50, 0 50, 0 0))).into()]);
    let target = push_geoms(&[wkt!(LINESTRING(0 0, 10 10, 20 0, 30 10)).into()]);

    let profile = GeometryProfile::from_sample(&sample).expect("from_sample failed");
    let (encoded, _) = target
        .encode_with_profile(&profile)
        .expect("profile_driven_optimisation failed");
    assert_geometry_roundtrip(&encoded, &target);
}

#[test]
fn profile_merge_roundtrip() {
    // Build two profiles from different geometry types, merge them, and verify
    // the merged profile produces valid encodings for both.
    let poly = push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 0))).into()]);
    let ls = push_geoms(&[wkt!(LINESTRING(0 0, 10 10, 20 20)).into()]);

    let merged = GeometryProfile::from_sample(&poly)
        .expect("from_sample poly failed")
        .merge(&GeometryProfile::from_sample(&ls).expect("from_sample ls failed"));

    let (encoded, _) = poly
        .encode_with_profile(&merged)
        .unwrap_or_else(|e| panic!("profile_driven_optimisation failed for poly: {e}"));
    assert_geometry_roundtrip(&encoded, &poly);

    let (encoded, _) = ls
        .encode_with_profile(&merged)
        .unwrap_or_else(|e| panic!("profile_driven_optimisation failed for ls: {e}"));
    assert_geometry_roundtrip(&encoded, &ls);
}

#[test]
fn profile_rederives_vertex_strategy_from_actual_data() {
    // Sample is built from high-repetition points (→ Morton in profile).
    // Target has all-unique coordinates in Vec2-only range.
    // apply_profile must re-derive the vertex strategy from the target data
    // and select Vec2, not blindly reuse Morton from the sample.
    let sample =
        push_geoms(&std::iter::repeat_n(point! { x: 5, y: 5 }.into(), 20).collect::<Vec<_>>());
    let target = push_geoms(
        &(0i32..10)
            .map(|i| point! { x: i, y: i }.into())
            .collect::<Vec<_>>(),
    );

    let profile = GeometryProfile::from_sample(&sample).expect("from_sample failed");
    let (encoded, _) = target
        .encode_with_profile(&profile)
        .expect("profile_driven_optimisation failed");
    let types = encoded_stream_types(&encoded);
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "apply_profile must re-derive Vec2 for all-unique target vertices"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Morton)),
        "apply_profile must not blindly reuse Morton from the sample profile"
    );

    assert_geometry_roundtrip(&encoded, &target);
}

#[test]
fn manual_encode_works() {
    use mlt_core::v01::{GeometryEncoder, IntEncoder, VertexBufferType};

    let decoded = push_geoms(&[wkt!(POINT(10 20)).into()]);

    let mut geom_enc = GeometryEncoder::all(IntEncoder::varint());
    geom_enc.vertex_buffer_type(VertexBufferType::Vec2);
    let result = decoded
        .clone()
        .encode(geom_enc)
        .expect("manual encode failed");
    let types = encoded_stream_types(&result);
    assert!(types.contains(&StreamType::Data(DictionaryType::Vertex)));

    assert_geometry_roundtrip(&result, &decoded);
}

/// Round-trip `GeometryValues` via bytes (no encoded→decoded converter).
fn assert_geometry_roundtrip(encoded: &EncodedGeometry, expected: &GeometryValues) {
    let mut buf = Vec::new();
    encoded.write_to(&mut buf).expect("write_to failed");
    let (inp, raw) = RawGeometry::from_bytes(&buf, &mut parser()).expect("parse failed");
    assert_eq!(inp.len(), 0, "expected all bytes to be consumed in parse");
    let result = raw.decode(&mut dec()).unwrap();
    assert_eq!(expected, &result);
}

fn push_geoms(geoms: &[Geom32]) -> GeometryValues {
    let mut d = GeometryValues::default();
    for g in geoms {
        d.push_geom(g);
    }
    d
}

/// Collect all stream types present in an encoded geometry (meta + items).
fn encoded_stream_types(encoded: &EncodedGeometry) -> HashSet<StreamType> {
    std::iter::once(encoded.meta.meta.stream_type)
        .chain(encoded.items.iter().map(|s| s.meta.stream_type))
        .collect()
}
