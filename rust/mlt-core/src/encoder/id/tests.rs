use geo_types::Point;
use proptest::prelude::*;
use rstest::rstest;

use crate::decoder::{GeometryValues, IdValues, LogicalEncoder, RawIdValue};
use crate::encoder::IdWidth::{Id32, Id64, OptId32, OptId64};
use crate::encoder::{
    Encoder, EncoderConfig, ExplicitEncoder, IdWidth, IntEncoder, StagedLayer, StagedLayer01,
};
use crate::geojson::Geom32;
use crate::test_helpers::{dec, into_layer01, parser};
use crate::{Layer, LazyParsed, MltError, MltResult};

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
    staged.encode_into(&mut enc).expect("encode failed");
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
        Id64,
        IntEncoder::varint_with(LogicalEncoder::None),
    );
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimization_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let ids = IdValues(vec![Some(large_value)]);
    let decoded_back =
        id_roundtrip_via_layer(&ids, Id32, IntEncoder::varint_with(LogicalEncoder::None));
    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_manual_fastpfor_roundtrip() {
    let ids = IdValues((0u64..200).map(|i| Some(i * 7 + 3)).collect());
    let decoded_back = id_roundtrip_via_layer(&ids, Id32, IntEncoder::fastpfor());
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
        ExplicitEncoder::for_id(IntEncoder::varint(), Id32),
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

// Test that each config produces the correct variant and optional stream presence
#[rstest]
#[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
#[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
fn test_config_produces_correct_variant(#[case] id_width: IdWidth, #[case] ids: Vec<Option<u64>>) {
    let input = IdValues(ids);
    let int_enc = IntEncoder::varint_with(LogicalEncoder::None);
    let raw_id = encode_id_to_raw_layer(&input, id_width, int_enc);

    match id_width {
        OptId32 | Id32 => assert!(matches!(raw_id.value, RawIdValue::Id32(_))),
        Id64 | OptId64 => assert!(matches!(raw_id.value, RawIdValue::Id64(_))),
    }

    match id_width {
        OptId32 | OptId64 => assert!(raw_id.presence.0.is_some()),
        Id32 | Id64 => assert!(raw_id.presence.0.is_none()),
    }
}

#[rstest]
#[case::id32_basic(Id32, &[Some(1), Some(2), Some(100), Some(1000)])]
#[case::id32_single(Id32, &[Some(42)])]
#[case::id32_boundaries(Id32, &[Some(0), Some(u64::from(u32::MAX))])]
#[case::id64_basic(Id64, &[Some(1), Some(2), Some(100), Some(1000)])]
#[case::id64_single(Id64, &[Some(u64::MAX)])]
#[case::id64_boundaries(Id64, &[Some(0), Some(u64::MAX)])]
#[case::id64_large_values(Id64, &[Some(0), Some(u64::from(u32::MAX)), Some(u64::from(u32::MAX) + 1), Some(u64::MAX)])]
#[case::opt_id32_with_nulls(OptId32, &[Some(1), None, Some(100), None, Some(1000)])]
#[case::opt_id32_no_nulls(OptId32, &[Some(1), Some(2), Some(3)])]
#[case::opt_id32_single_null(OptId32, &[None])]
#[case::opt_id64_with_nulls(OptId64, &[Some(1), None, Some(u64::from(u32::MAX) + 1), None, Some(u64::MAX)])]
#[case::opt_id64_all_nulls(OptId64, &[None, None, None])]
#[case::none(Id32, &[])]
fn test_roundtrip(#[case] id_width: IdWidth, #[case] ids: &[Option<u64>]) {
    let int_enc = IntEncoder::varint_with(LogicalEncoder::None);
    assert_roundtrip(ids, id_width, int_enc);
}

#[rstest]
fn test_sequential_ids(
    #[values(LogicalEncoder::None)] logical: LogicalEncoder,
    #[values(Id32, OptId32, Id64, OptId64)] id_width: IdWidth,
) {
    let input: Vec<_> = (1..=100).map(Some).collect();
    let int_enc = IntEncoder::varint_with(logical);
    assert_roundtrip(&input, id_width, int_enc);
}

proptest! {
    #[test]
    fn test_roundtrip_opt_id32(
        ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
        prop_assert_roundtrip(&ids_u64, OptId32, IntEncoder::varint_with(logical))?;
    }

    #[test]
    fn test_roundtrip_id64(
        ids in prop::collection::vec(any::<u64>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        prop_assert_roundtrip(&ids_u64, Id64, IntEncoder::varint_with(logical))?;
    }

    #[test]
    fn test_roundtrip_id32(
        ids in prop::collection::vec(any::<u32>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        prop_assert_roundtrip(&ids_u64, Id32, IntEncoder::varint_with(logical))?;
    }

    #[test]
    fn test_roundtrip_opt_id64(
        ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        prop_assert_roundtrip(&ids, OptId64, IntEncoder::varint_with(logical))?;
    }

    #[test]
    fn test_correct_variant_produced_id32(
        ids in prop::collection::vec(1u32..1000u32, 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_produces_correct_variant(ids_u64, Id32, IntEncoder::varint_with(logical))?;
    }

    #[test]
    fn test_correct_variant_produced_id64(
        ids in prop::collection::vec(any::<u64>(), 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        assert_produces_correct_variant(ids_u64, Id64, IntEncoder::varint_with(logical))?;
    }
}

/// Round-trip `IdValues` via full layer bytes (encode → bytes → parse → decode).
fn assert_roundtrip(ids: &[Option<u64>], id_width: IdWidth, int_enc: IntEncoder) {
    prop_assert_roundtrip(ids, id_width, int_enc).expect("roundtrip failed");
}

fn prop_assert_roundtrip(
    ids: &[Option<u64>],
    id_width: IdWidth,
    int_enc: IntEncoder,
) -> Result<(), TestCaseError> {
    let ids = IdValues(ids.to_vec());
    let res = roundtrip_id_values(&ids, id_width, int_enc)
        .map_err(|e| TestCaseError::Fail(format!("Roundtrip failed: {e:?}").into()))?;
    prop_assert_eq!(res, ids.clone());
    Ok(())
}

fn roundtrip_id_values(
    decoded: &IdValues,
    id_width: IdWidth,
    int_enc: IntEncoder,
) -> MltResult<IdValues> {
    if decoded.0.is_empty() {
        return Ok(IdValues(vec![]));
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
        EncoderConfig::default(),
        ExplicitEncoder::for_id(int_enc, id_width),
    );
    staged.encode_into(&mut enc)?;
    let buf = enc.into_layer_bytes()?;
    let (_, layer) = Layer::from_bytes(&buf, &mut parser())?;
    let Layer::Tag01(layer01) = layer else {
        return Err(MltError::NotDecoded("expected Tag01 layer"));
    };
    // When all source IDs were null, the encoder skips the ID column entirely.
    // On decode, the absent column is semantically identical to all-null IDs.
    match layer01.id {
        Some(id) => id.into_parsed(&mut dec()),
        None => Ok(IdValues(vec![None; n])),
    }
}

/// Encode `ids` into a full layer and return the parsed raw ID.
fn encode_id_to_raw_layer(
    ids: &IdValues,
    id_width: IdWidth,
    int_enc: IntEncoder,
) -> crate::decoder::RawId<'static> {
    // Use write_id_to to directly encode just the ID and verify the encoded bytes.
    // To test via a full layer, we need to call encode_with on a StagedLayer01.
    let n = ids.0.len();
    let mut geometry = GeometryValues::default();
    for _ in 0..n {
        geometry.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    let staged = StagedLayer01 {
        name: "id_test".to_string(),
        extent: 4096,
        id: Some(ids.clone()),
        geometry,
        properties: vec![],
    };
    let mut enc = Encoder::with_explicit(
        EncoderConfig::default(),
        ExplicitEncoder::for_id(int_enc, id_width),
    );
    staged.encode_into(&mut enc).expect("encode failed");
    let buf = enc.into_layer_bytes().expect("into_layer_bytes failed");
    let buf: &'static [u8] = Box::leak(buf.into_boxed_slice());
    let (_, layer) = Layer::from_bytes(buf, &mut parser()).expect("parse failed");
    let Layer::Tag01(layer01) = layer else {
        panic!("expected Tag01")
    };
    let Some(LazyParsed::Raw(raw_id)) = layer01.id else {
        panic!("expected raw id")
    };
    raw_id
}

fn assert_produces_correct_variant(
    ids: Vec<Option<u64>>,
    id_width: IdWidth,
    int_enc: IntEncoder,
) -> Result<(), TestCaseError> {
    let input = IdValues(ids);
    let raw_id = encode_id_to_raw_layer(&input, id_width, int_enc);

    if matches!(id_width, Id32 | OptId32) {
        prop_assert!(
            matches!(raw_id.value, RawIdValue::Id32(_)),
            "Expected Id32 variant"
        );
    } else {
        prop_assert!(
            matches!(raw_id.value, RawIdValue::Id64(_)),
            "Expected Id64 variant"
        );
    }

    if matches!(id_width, OptId32 | OptId64) {
        prop_assert!(
            raw_id.presence.0.is_some(),
            "Expected optional stream to be present"
        );
    } else {
        prop_assert!(raw_id.presence.0.is_none(), "Expected no optional stream");
    }
    Ok(())
}
