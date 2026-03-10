use std::collections::HashSet;

use geo_types::{LineString, Point, Polygon, point, wkt};
use mlt_core::geojson::{Coord32, Geom32};
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{
    DecodedGeometry, DictionaryType, GeometryProfile, LengthType, OffsetType, OwnedEncodedGeometry,
    OwnedGeometry, StreamType,
};
use mlt_core::{Encodable as _, FromEncoded as _, borrowme};
use pretty_assertions::assert_eq;

fn optimize_roundtrip(decoded: &DecodedGeometry) -> DecodedGeometry {
    let mut geom = OwnedGeometry::Decoded(decoded.clone());
    geom.automatic_encoding_optimisation()
        .expect("optimize failed");
    borrowme::borrow(&geom).decode().expect("decode failed")
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
    let decoded = push_geoms(&geoms);
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

    let types = encoded_stream_types(encoded);
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
    let decoded = push_geoms(&geoms);
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

    let types = encoded_stream_types(encoded);
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
    let decoded = push_geoms(&geoms);
    let mut geom = OwnedGeometry::Decoded(decoded);
    geom.automatic_encoding_optimisation()
        .expect("encode failed");
    let encoded = geom.borrow_encoded().expect("must be encoded");

    let types = encoded_stream_types(encoded);
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
    let coords: Vec<Coord32> = vec![(0, 0), (10, 0), (10, 10), (0, 0)]
        .into_iter()
        .map(|(x, y)| Coord32 { x, y })
        .collect();
    let geoms = vec![Geom32::Polygon(Polygon::new(LineString(coords), vec![]))];
    let decoded = push_geoms(&geoms);
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
    let pts = wkt!(MULTIPOINT(5 5, 5 5, 5 5));
    decoded.push_geom(&pts.into());
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

// ── GeometryProfile / ProfileOptimisation ─────────────────────────────────────

/// Build a profile from `sample`, then apply it to `target` and return the
/// decoded result.
fn profile_roundtrip(sample: &DecodedGeometry, target: &DecodedGeometry) -> DecodedGeometry {
    let profile = GeometryProfile::from_sample(sample).expect("from_sample failed");
    let mut geom = OwnedGeometry::Decoded(target.clone());
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation failed");
    borrowme::borrow(&geom).decode().expect("decode failed")
}

#[test]
fn profile_roundtrip_single_point() {
    let geoms = vec![wkt!(POINT(10 20)).into()];
    let decoded = push_geoms(&geoms);
    let result = profile_roundtrip(&decoded, &decoded);
    assert_eq!(decoded, result);
}

#[test]
fn profile_roundtrip_linestring() {
    let geoms = vec![wkt!(LINESTRING(10 20, 30 40, 50 60)).into()];
    let decoded = push_geoms(&geoms);
    let result = profile_roundtrip(&decoded, &decoded);
    assert_eq!(decoded, result);
}

#[test]
fn profile_roundtrip_polygon() {
    let geoms = vec![wkt!(POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))).into()];
    let decoded = push_geoms(&geoms);
    let result = profile_roundtrip(&decoded, &decoded);
    assert_eq!(decoded, result);
}

#[test]
fn profile_roundtrip_mixed_geometry_types() {
    // Mixed geometry types undergo a normalisation step during encode/decode, so
    // the first pass produces a "canonical" form.  Like the automatic roundtrip
    // test, we verify stability (two passes agree) rather than identity with the
    // raw original.
    let point = wkt!(POINT(1 2)).into();
    let ls = wkt!(LINESTRING(0 0, 1 1, 2 2)).into();
    let poly = wkt!(POLYGON((0 0, 10 0, 10 10, 0 0))).into();
    let original = push_geoms(&[point, ls, poly]);
    let canonical = profile_roundtrip(&original, &original);
    let output = profile_roundtrip(&canonical, &canonical);
    assert_eq!(canonical, output);
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
    let result = borrowme::borrow(&geom).decode().expect("decode failed");
    assert_eq!(target, result);
}

#[test]
fn profile_merge_roundtrips() {
    // Build two profiles from different geometry types and merge them.
    // The merged profile must still produce a valid encoding for both geometries.
    let poly = push_geoms(&[wkt!(POLYGON((0 0, 100 0, 100 100, 0 0))).into()]);
    let ls = push_geoms(&[wkt!(LINESTRING(0 0, 10 10, 20 20)).into()]);

    let profile_a = GeometryProfile::from_sample(&poly).expect("from_sample poly failed");
    let profile_b = GeometryProfile::from_sample(&ls).expect("from_sample ls failed");
    let merged = profile_a.merge(&profile_b);

    // Apply the merged profile to both geometry types.
    for (label, target) in [("poly", &poly), ("ls", &ls)] {
        let mut geom = OwnedGeometry::Decoded(target.clone());
        geom.profile_driven_optimisation(&merged)
            .unwrap_or_else(|e| panic!("profile_driven_optimisation failed for {label}: {e}"));
        let result = borrowme::borrow(&geom).decode().expect("decode failed");
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
    let sample_geoms: Vec<Geom32> = std::iter::repeat_n(point! { x: 5, y: 5 }.into(), 20).collect();
    let sample = push_geoms(&sample_geoms);

    let target_geoms: Vec<Geom32> = (0i32..10).map(|i| point! { x: i, y: i }.into()).collect();
    let target = push_geoms(&target_geoms);

    let profile = GeometryProfile::from_sample(&sample).expect("from_sample failed");
    let mut geom = OwnedGeometry::Decoded(target.clone());
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation failed");

    let encoded = geom.borrow_encoded().expect("must be encoded");
    let types = encoded_stream_types(encoded);

    // All-unique coordinates must have been detected at apply time → Vec2.
    assert!(
        types.contains(&StreamType::Data(DictionaryType::Vertex)),
        "apply_profile must re-derive Vec2 for all-unique target vertices"
    );
    assert!(
        !types.contains(&StreamType::Data(DictionaryType::Morton)),
        "apply_profile must not blindly reuse Morton from the sample profile"
    );

    // And the decoded result must still match the original.
    let result = borrowme::borrow(&geom).decode().expect("decode failed");
    assert_eq!(target, result);
}

#[test]
fn profile_starting_from_encoded_state_roundtrips() {
    // profile_driven_optimisation must also work when OwnedGeometry starts
    // in the Encoded state (it should decode first, then re-encode).
    let geoms = vec![wkt!(LINESTRING(0 0, 5 10, 15 20)).into()];
    let decoded = push_geoms(&geoms);

    // First encode it automatically so we start in Encoded state.
    let mut geom = OwnedGeometry::Decoded(decoded.clone());
    geom.automatic_encoding_optimisation()
        .expect("automatic optimisation failed");
    assert!(
        geom.borrow_encoded().is_some(),
        "must be encoded after auto"
    );

    // Build profile from the decoded geometry, then apply it to the Encoded geom.
    let profile = GeometryProfile::from_sample(&decoded).expect("from_sample failed");
    geom.profile_driven_optimisation(&profile)
        .expect("profile_driven_optimisation on Encoded state failed");

    let result = borrowme::borrow(&geom).decode().expect("decode failed");
    assert_eq!(decoded, result);
}

#[test]
fn profile_and_automatic_both_roundtrip_consistently() {
    // Both paths must produce semantically identical decoded output for the
    // same input — they may choose different encoders, but the data must match.
    let geoms = vec![
        wkt!(POLYGON((0 0, 80 0, 80 80, 0 80, 0 0))).into(),
        wkt!(POLYGON((10 10, 20 10, 20 20, 10 20, 10 10))).into(),
    ];
    let decoded = push_geoms(&geoms);

    let auto_result = optimize_roundtrip(&decoded);

    let profile_result = profile_roundtrip(&decoded, &decoded);

    assert_eq!(
        auto_result, profile_result,
        "automatic and profile-driven paths must decode to the same geometry"
    );
}

#[test]
fn manual_optimisation_works() {
    use mlt_core::v01::{GeometryEncoder, IntEncoder, VertexBufferType};

    let geoms = vec![wkt!(POINT(10 20)).into()];
    let decoded = push_geoms(&geoms);
    let mut geom = OwnedGeometry::Decoded(decoded.clone());

    // Manually use all-varint encoder
    let mut encoder = GeometryEncoder::all(IntEncoder::varint());
    // Ensure we use Vec2 for points
    encoder.vertex_buffer_type(VertexBufferType::Vec2);

    geom.manual_optimisation(encoder)
        .expect("manual optimization failed");

    let enc = geom.borrow_encoded().expect("must be encoded");
    let types = encoded_stream_types(enc);
    assert!(types.contains(&StreamType::Data(DictionaryType::Vertex)));

    let decoded_back = DecodedGeometry::from_encoded(borrowme::borrow(enc)).expect("decode failed");
    assert_eq!(decoded, decoded_back);
}
