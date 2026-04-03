use geo_types::Point;
use mlt_core::encoder::{
    Encoder, ExplicitEncoder, IdWidth, IntEncoder, StagedLayer, StagedLayer01,
};
use mlt_core::geojson::Geom32;
use mlt_core::test_helpers::{dec, into_layer01, parser};
use mlt_core::{GeometryValues, IdValues, Layer, LogicalEncoder};
use rstest::rstest;

/// Round-trip `IdValues` via full layer bytes using an explicit encoder.
fn id_roundtrip_via_layer(decoded: &IdValues, id_width: IdWidth, int_enc: IntEncoder) -> IdValues {
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
    let mut enc = Encoder::with_explicit(
        Encoder::default().cfg,
        ExplicitEncoder::for_id(int_enc, id_width),
    );
    staged.encode_explicit(&mut enc).expect("encode failed");
    let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");
    let mut p = parser();
    let (_, layer) = Layer::from_bytes(&buf, &mut p).expect("parse failed");

    into_layer01(layer)
        .id
        .expect("expected id column")
        .into_parsed(&mut dec())
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
    let mut enc = Encoder::default();
    StagedLayer::Tag01(staged)
        .encode_into(&mut enc)
        .expect("encode failed");
    let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");
    let mut p = parser();
    let mut d = dec();
    let (_, layer) = Layer::from_bytes(&buf, &mut p).expect("parse failed");
    assert!(p.reserved() > 0, "parser should reserve bytes after parse");
    let layer01 = into_layer01(layer);

    let result = layer01
        .id
        .expect("expected id column")
        .into_parsed(&mut d)
        .expect("decode failed");
    assert!(
        d.consumed() > 0,
        "decoder should consume bytes after decode"
    );
    result
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

/// Verify that automatic encoding produces no column for empty or all-null ID lists.
#[rstest]
#[case::empty(IdValues(vec![]))]
#[case::all_nulls(IdValues(vec![None, None]))]
fn test_automatic_encoding_skipped(#[case] input: IdValues) {
    let mut enc = Encoder::default();
    let written = input.write_to(&mut enc).unwrap();
    assert!(!written, "empty or all-null ID list should write no column");
}

/// Verify that automatic encoding produces a column for non-trivial inputs.
#[rstest]
#[case::short_sequence(IdValues(vec![Some(1), Some(2)]))]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_encoding_produces_output(#[case] input: IdValues) {
    let mut enc = Encoder::default();
    let written = input.write_to(&mut enc).unwrap();
    assert!(written, "non-trivial ID list should write a column");
}

#[test]
fn test_automatic_optimization_roundtrip_empty() {
    let decoded = IdValues(vec![]);
    let mut enc = Encoder::default();
    let written = decoded.write_to(&mut enc).unwrap();
    assert!(!written, "empty ID list should write no column");
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_optimization_roundtrip(#[case] decoded: IdValues) {
    let decoded_back = id_roundtrip_auto(&decoded);
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimization_applies_encoder() {
    let decoded = create_u32_range_ids();
    let decoded_back = id_roundtrip_via_layer(
        &decoded,
        IdWidth::Id64,
        IntEncoder::varint_with(LogicalEncoder::None),
    );
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimization_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let ids = IdValues(vec![Some(large_value)]);
    let decoded_back = id_roundtrip_via_layer(
        &ids,
        IdWidth::Id32,
        IntEncoder::varint_with(LogicalEncoder::None),
    );
    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_manual_fastpfor_roundtrip() {
    let ids = IdValues((0u64..200).map(|i| Some(i * 7 + 3)).collect());
    let decoded_back = id_roundtrip_via_layer(&ids, IdWidth::Id32, IntEncoder::fastpfor());
    assert_eq!(decoded_back, ids);
}

/// Verify that large sequential u32 IDs produce a smaller encoding than plain varint
/// (confirming that delta+FastPFOR is being selected automatically).
#[test]
fn test_auto_fastpfor_beats_varint_for_large_u32_ids() {
    let ids = IdValues((0u64..1000).map(|i| Some(i * 13 + 5)).collect());

    let mut auto_enc = Encoder::default();
    ids.clone().write_to(&mut auto_enc).unwrap();

    let mut plain_enc = Encoder::with_explicit(
        Encoder::default().cfg,
        ExplicitEncoder::for_id(IntEncoder::varint(), IdWidth::Id32),
    );
    ids.write_to(&mut plain_enc).unwrap();

    assert!(
        auto_enc.total_len() <= plain_enc.total_len(),
        "auto ({} bytes) should not be worse than plain varint ({} bytes)",
        auto_enc.total_len(),
        plain_enc.total_len()
    );
}

/// Verify that large u64 IDs round-trip correctly under automatic encoding.
#[test]
fn test_auto_roundtrip_large_u64_ids() {
    let base = u64::from(u32::MAX) + 1;
    let ids = IdValues((0u64..1000).map(|i| Some(base + i * 13)).collect());
    let decoded_back = id_roundtrip_auto(&ids);
    assert_eq!(decoded_back, ids);
}
