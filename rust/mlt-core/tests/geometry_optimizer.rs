use std::collections::HashSet;

use geo_types::{LineString, Point, Polygon, point, wkt};
use mlt_core::Encodable as _;
use mlt_core::geojson::{Coord32, Geom32};
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{
    DecodedGeometry, DictionaryType, GeometryProfile, LengthType, OffsetType, OwnedEncodedGeometry,
    OwnedGeometry, StreamType,
};
use pretty_assertions::assert_eq;
use rstest::rstest;

fn optimize_roundtrip(decoded: &DecodedGeometry) -> DecodedGeometry {
    let mut geom = OwnedGeometry::Decoded(decoded.clone());
    geom.automatic_encoding_optimisation()
        .expect("optimize failed");
    DecodedGeometry::try_from(geom).expect("decode failed")
}

fn profile_roundtrip(sample: &DecodedGeometry, target: &DecodedGeometry) -> DecodedGeometry {
    let profile = GeometryProfile::from_sample(sample).expect("from_sample failed");
    let mut geom = OwnedGeometry::Decoded(target.clone());
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation failed");
    DecodedGeometry::try_from(geom).expect("decode failed")
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

#[rstest]
#[case::single_point(push_geoms(&[wkt!(POINT(10 20)).into()]))]
#[case::linestring(push_geoms(&[wkt!(LINESTRING(10 20, 30 40, 50 60)).into()]))]
#[case::polygon(push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()]))]
#[case::multi_polygon(push_geoms(&[wkt!(MULTIPOLYGON(((0 0, 10 0, 10 10, 0 0),(5 5, 15 5, 15 15, 5 15)))).into()]))]
#[case::mixed(push_geoms(&[wkt!(POINT(1 2)).into(), wkt!(LINESTRING(0 0, 1 1, 2 2)).into(), wkt!(POLYGON((0 0, 10 0, 10 10, 0 0))).into()]))]
fn roundtrip_is_stable(#[case] decoded: DecodedGeometry) {
    let auto_pass1 = optimize_roundtrip(&decoded);
    let auto_pass2 = optimize_roundtrip(&auto_pass1);
    assert_eq!(
        auto_pass1, auto_pass2,
        "automatic: second pass must match first"
    );

    let prof_pass1 = profile_roundtrip(&decoded, &decoded);
    let prof_pass2 = profile_roundtrip(&prof_pass1, &prof_pass1);
    assert_eq!(
        prof_pass1, prof_pass2,
        "profile: second pass must match first"
    );

    assert_eq!(auto_pass1, prof_pass1, "automatic and profile must agree");
}

#[rstest]
#[case::all_unique(
    push_geoms(&(0i32..10).map(|i| point! { x: i, y: i }.into()).collect::<Vec<_>>()),
    DictionaryType::Vertex,
)]
#[case::high_repetition(
    push_geoms(&std::iter::repeat_n(point! { x: 5, y: 5 }.into(), 20).collect::<Vec<_>>()),
    DictionaryType::Morton,
)]
#[case::out_of_morton_range(
    push_geoms(&std::iter::repeat_n(Geom32::Point(Point(Coord32 { x: 0x1_0000, y: 0x1_0000 })), 50).collect::<Vec<_>>()),
    DictionaryType::Vertex,
)]
fn vertex_strategy_selects_correct_encoding(
    #[case] decoded: DecodedGeometry,
    #[case] expected: DictionaryType,
) {
    let unexpected = if expected == DictionaryType::Vertex {
        DictionaryType::Morton
    } else {
        DictionaryType::Vertex
    };

    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");
    let types = encoded_stream_types(encoded);

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
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

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
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

    let stream_types = encoded_stream_types(encoded);
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
    decoded.push_geom(&wkt!(MULTIPOINT(5 5, 5 5, 5 5)).into());
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

    let stream_types = encoded_stream_types(encoded);
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
fn profile_applied_to_different_tile_roundtrips() {
    // Build the profile from a polygon tile, apply it to a linestring tile.
    // The topology is completely different — apply_profile must still produce a
    // valid encoding because it re-runs the probe pass on the actual tile data.
    let sample = push_geoms(&[wkt!(POLYGON((0 0, 50 0, 50 50, 0 50, 0 0))).into()]);
    let target = push_geoms(&[wkt!(LINESTRING(0 0, 10 10, 20 0, 30 10)).into()]);

    let profile = GeometryProfile::from_sample(&sample).expect("from_sample failed");
    let mut geom = OwnedGeometry::Decoded(target.clone());
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation failed");
    let result = DecodedGeometry::try_from(geom).expect("decode failed");
    assert_eq!(target, result);
}

#[test]
fn profile_merge_roundtrips() {
    // Build two profiles from different geometry types, merge them, and verify
    // the merged profile produces valid encodings for both.
    let poly = push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 0))).into()]);
    let ls = push_geoms(&[wkt!(LINESTRING(0 0, 10 10, 20 20)).into()]);

    let merged = GeometryProfile::from_sample(&poly)
        .expect("from_sample poly failed")
        .merge(&GeometryProfile::from_sample(&ls).expect("from_sample ls failed"));

    for (label, target) in [("poly", &poly), ("ls", &ls)] {
        let mut geom = OwnedGeometry::Decoded(target.clone());
        geom.profile_driven_optimisation(&merged)
            .unwrap_or_else(|e| panic!("profile_driven_optimisation failed for {label}: {e}"));
        let result = DecodedGeometry::try_from(geom).expect("decode failed");
        assert_eq!(
            *target, result,
            "merged profile roundtrip failed for {label}"
        );
    }
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
    let mut geom = OwnedGeometry::Decoded(target.clone());
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation failed");

    let encoded = geom.borrow_encoded().expect("must be encoded");
    let types = encoded_stream_types(encoded);
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "apply_profile must re-derive Vec2 for all-unique target vertices"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Morton)),
        "apply_profile must not blindly reuse Morton from the sample profile"
    );

    let result = DecodedGeometry::try_from(geom).expect("decode failed");
    assert_eq!(target, result);
}

#[test]
fn profile_starting_from_encoded_state_roundtrips() {
    // profile_driven_optimisation must also work when OwnedGeometry starts in
    // the Encoded state (it should decode first, then re-encode).
    let decoded = push_geoms(&[wkt!(LINESTRING(0 0, 5 10, 15 20)).into()]);

    let mut geom = OwnedGeometry::Decoded(decoded.clone());
    geom.automatic_encoding_optimisation()
        .expect("automatic optimisation failed");
    assert!(
        geom.borrow_encoded().is_some(),
        "must be encoded after auto"
    );

    let profile = GeometryProfile::from_sample(&decoded).expect("from_sample failed");
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation on Encoded state failed");

    let result = DecodedGeometry::try_from(geom).expect("decode failed");
    assert_eq!(decoded, result);
}

#[test]
fn manual_optimisation_works() {
    use mlt_core::v01::{GeometryEncoder, IntEncoder, VertexBufferType};

    let decoded = push_geoms(&[wkt!(POINT(10 20)).into()]);
    let mut geom = OwnedGeometry::Decoded(decoded.clone());

    let mut encoder = GeometryEncoder::all(IntEncoder::varint());
    encoder.vertex_buffer_type(VertexBufferType::Vec2);
    geom.manual_optimisation(encoder)
        .expect("manual optimization failed");

    let enc = geom.borrow_encoded().expect("must be encoded");
    let types = encoded_stream_types(enc);
    assert!(types.contains(&StreamType::Data(DictionaryType::Vertex)));

    let decoded_back = DecodedGeometry::try_from(enc.clone()).expect("decode failed");
    assert_eq!(decoded, decoded_back);
}
