use geo_types::Point;
use mlt_core::geojson::Geom32;
use mlt_core::v01::{
    GeometryEncoder, GeometryProfile, GeometryValues, IdEncoder, IdProfile, IdValues, IdWidth,
    IntEncoder, LogicalEncoder, PropertyProfile, StagedLayer01, StagedLayer01Encoder, Tag01Profile,
};
use mlt_core::{Decoder, EncodedLayer, Layer, MemBudget};
use rstest::rstest;

/// Round-trip `IdValues` via full layer bytes (no encoded→decoded converter).
fn id_roundtrip_via_layer(decoded: &IdValues, id_encoder: IdEncoder) -> IdValues {
    if decoded.0.is_empty() {
        return IdValues(vec![]);
    }
    let n = decoded.0.len();
    let mut geometry = GeometryValues::default();
    for _ in 0..n {
        geometry.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    let staged = StagedLayer01 {
        name: "id_roundtrip".to_string(),
        extent: 4096,
        id: Some(decoded.clone()),
        geometry,
        properties: vec![],
    };
    let stream_encoder = StagedLayer01Encoder {
        id: Some(id_encoder),
        geometry: GeometryEncoder::all(IntEncoder::varint()),
        properties: vec![],
    };
    let layer_enc = staged.encode(stream_encoder).expect("encode failed");
    let mut buf = Vec::new();
    EncodedLayer::Tag01(layer_enc)
        .write_to(&mut buf)
        .expect("write_to failed");
    let (_, layer) = Layer::from_bytes(&buf, &mut MemBudget::default()).expect("parse failed");
    let Layer::Tag01(layer01) = layer else {
        panic!("expected Tag01 layer");
    };
    layer01
        .id
        .expect("expected id column")
        .into_parsed(&mut Decoder::default())
        .expect("decode failed")
}

fn id_roundtrip_auto(decoded: &IdValues) -> IdValues {
    if decoded.0.is_empty() {
        return IdValues(vec![]);
    }
    let n = decoded.0.len();
    let mut geometry = GeometryValues::default();
    for _ in 0..n {
        geometry.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    let staged = StagedLayer01 {
        name: "id_roundtrip".to_string(),
        extent: 4096,
        id: Some(decoded.clone()),
        geometry,
        properties: vec![],
    };
    let (encoded, _) = staged.encode_auto().expect("encode_auto failed");
    let mut buf = Vec::new();
    EncodedLayer::Tag01(encoded)
        .write_to(&mut buf)
        .expect("write_to failed");
    let (_, layer) = Layer::from_bytes(&buf, &mut MemBudget::default()).expect("parse failed");
    let Layer::Tag01(layer01) = layer else {
        panic!("expected Tag01 layer");
    };
    layer01
        .id
        .expect("expected id column")
        .into_parsed(&mut Decoder::default())
        .expect("decode failed")
}

fn id_roundtrip_with_profile(decoded: &IdValues, profile: &IdProfile) -> IdValues {
    if decoded.0.is_empty() {
        return IdValues(vec![]);
    }
    let n = decoded.0.len();
    let mut geometry = GeometryValues::default();
    for _ in 0..n {
        geometry.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    let staged = StagedLayer01 {
        name: "id_roundtrip".to_string(),
        extent: 4096,
        id: Some(decoded.clone()),
        geometry: geometry.clone(),
        properties: vec![],
    };
    let geom_profile = GeometryProfile::from_sample(&geometry).expect("geometry profile");
    let tag01 = Tag01Profile::new(
        None,
        profile.clone(),
        PropertyProfile::from_sample(&[]),
        geom_profile,
    );
    let (encoded, _) = staged
        .encode_with_profile(&tag01)
        .expect("encode_with_profile failed");
    let mut buf = Vec::new();
    EncodedLayer::Tag01(encoded)
        .write_to(&mut buf)
        .expect("write_to failed");
    let (_, layer) = Layer::from_bytes(&buf, &mut MemBudget::default()).expect("parse failed");
    let Layer::Tag01(layer01) = layer else {
        panic!("expected Tag01 layer");
    };
    layer01
        .id
        .expect("expected id column")
        .into_parsed(&mut Decoder::default())
        .expect("decode failed")
}

fn create_u32_range_ids() -> IdValues {
    IdValues((1u64..=100).map(Some).collect())
}

fn create_u64_range_ids() -> IdValues {
    let base = u64::from(u32::MAX) + 1;
    IdValues((base..base + 50).map(Some).collect())
}

fn create_ids_with_nulls() -> IdValues {
    IdValues(vec![Some(10), None, Some(20), None, Some(30)])
}

fn create_constant_ids() -> IdValues {
    IdValues(vec![Some(42), Some(42), Some(42), Some(42), Some(42)])
}

#[rstest]
#[case::empty(
    IdValues(vec![]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::all_nulls(
    IdValues(vec![None, None]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::short_sequence(
    IdValues(vec![Some(1), Some(2)]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::sequential_u32(
    create_u32_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::sequential_u64(
    create_u64_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id64)
)]
#[case::constant(
    create_constant_ids(),
    IdEncoder::new(LogicalEncoder::Rle, IdWidth::Id32)
)]
#[case::with_nulls(
    create_ids_with_nulls(),
    IdEncoder::new(LogicalEncoder::Delta, IdWidth::OptId32)
)]
fn test_automatic_optimisation_selection(#[case] input: IdValues, #[case] expected: IdEncoder) {
    let is_empty = input.0.is_empty();
    let (_encoded, enc) = input.encode_auto().unwrap();
    assert_eq!(enc, if is_empty { None } else { Some(expected) });
}

#[test]
fn test_automatic_optimisation_roundtrip_empty() {
    let decoded = IdValues(vec![]);
    let (encoded, _enc) = decoded.clone().encode_auto().unwrap();
    assert!(encoded.is_none(), "empty ID list should produce None");
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_optimisation_roundtrip(#[case] decoded: IdValues) {
    let decoded_back = id_roundtrip_auto(&decoded);
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_applies_encoder() {
    let decoded = create_u32_range_ids();
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    let decoded_back = id_roundtrip_via_layer(&decoded, manual_enc);
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let ids = IdValues(vec![Some(large_value)]);
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    let decoded_back = id_roundtrip_via_layer(&ids, manual_enc);
    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_profile_applies_candidates_and_rederives_width() {
    let u32_sample = create_u32_range_ids();
    let profile = IdProfile::from_sample(&u32_sample);

    let u64_decoded = create_u64_range_ids();
    let decoded_back = id_roundtrip_with_profile(&u64_decoded, &profile);
    assert_eq!(decoded_back, u64_decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_profile_roundtrip(#[case] decoded: IdValues) {
    let profile = IdProfile::from_sample(&decoded);
    let decoded_back = id_roundtrip_with_profile(&decoded, &profile);
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_profile_from_sample_is_nonempty() {
    let profile = IdProfile::from_sample(&create_u32_range_ids());
    insta::assert_debug_snapshot!(profile, @"
    IdProfile {
        candidates: [
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ],
    }
    ");
}

#[test]
fn test_profile_merge_is_union() {
    let p1 = IdProfile::new(vec![IntEncoder::varint()]);
    let p2 = IdProfile::new(vec![IntEncoder::varint(), IntEncoder::fastpfor()]);
    let merged = p1.merge(&p2);
    insta::assert_debug_snapshot!(merged, @"
    IdProfile {
        candidates: [
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: FastPFOR,
            },
        ],
    }
    ");
}

#[test]
fn test_profile_merge_empty() {
    let p1 = IdProfile::new(vec![]);
    let p2 = IdProfile::new(vec![]);
    let merged = p1.merge(&p2);
    insta::assert_debug_snapshot!(merged, @"
    IdProfile {
        candidates: [],
    }
    ");
}
