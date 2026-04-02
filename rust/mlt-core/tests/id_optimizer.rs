use geo_types::Point;
use mlt_core::encoder::{
    EncodedLayer, EncoderConfig, GeometryEncoder, IdEncoder, IdWidth, IntEncoder, StagedLayer01,
    StagedLayer01Encoder,
};
use mlt_core::geojson::Geom32;
use mlt_core::test_helpers::{dec, into_layer01, parser};
use mlt_core::{GeometryValues, IdValues, Layer, LogicalEncoder};
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
        id: id_encoder,
        geometry: GeometryEncoder::all(IntEncoder::varint()),
        properties: vec![],
    };
    let layer_enc = staged.encode(stream_encoder).expect("encode failed");
    let mut buf = Vec::new();
    EncodedLayer::Tag01(layer_enc)
        .write_to(&mut buf)
        .expect("write_to failed");
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
    let (encoded, _) = staged
        .encode_auto(EncoderConfig::default())
        .expect("encode_auto failed");
    let mut buf = Vec::new();
    EncodedLayer::Tag01(encoded)
        .write_to(&mut buf)
        .expect("write_to failed");
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
fn test_automatic_optimization_selection(#[case] input: IdValues, #[case] expected: IdEncoder) {
    let is_skipped = input.0.is_empty() || input.0.iter().all(Option::is_none);
    let result = input.encode_auto(EncoderConfig::default()).unwrap();
    let enc = result.map(|(_, enc)| enc);
    assert_eq!(enc, if is_skipped { None } else { Some(expected) });
}

#[test]
fn test_automatic_optimization_roundtrip_empty() {
    let decoded = IdValues(vec![]);
    let result = decoded
        .clone()
        .encode_auto(EncoderConfig::default())
        .unwrap();
    assert!(result.is_none(), "empty ID list should produce None");
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
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    let decoded_back = id_roundtrip_via_layer(&decoded, manual_enc);
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimization_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let ids = IdValues(vec![Some(large_value)]);
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    let decoded_back = id_roundtrip_via_layer(&ids, manual_enc);
    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_manual_fastpfor_roundtrip() {
    let ids = IdValues((0u64..200).map(|i| Some(i * 7 + 3)).collect());
    let enc = IdEncoder::with_int_encoder(IntEncoder::fastpfor(), IdWidth::Id32);
    let decoded_back = id_roundtrip_via_layer(&ids, enc);
    assert_eq!(decoded_back, ids);
}

#[test]
fn test_auto_selects_fastpfor_for_large_u32_ids() {
    let ids = IdValues((0u64..1000).map(|i| Some(i * 13 + 5)).collect());
    let (_, enc) = ids.encode_auto(EncoderConfig::default()).unwrap().unwrap();
    assert_eq!(enc.int_encoder, IntEncoder::delta_fastpfor());
}

#[test]
fn test_auto_keeps_varint_for_u64_ids() {
    let base = u64::from(u32::MAX) + 1;
    let ids = IdValues((0u64..1000).map(|i| Some(base + i * 13)).collect());
    let (_, enc) = ids.encode_auto(EncoderConfig::default()).unwrap().unwrap();
    assert_eq!(enc.int_encoder, IntEncoder::delta_varint());
}
