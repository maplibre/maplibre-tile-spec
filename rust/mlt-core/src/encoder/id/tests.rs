#![cfg(test)]

use geo_types::Point;
use proptest::prelude::*;
use rstest::rstest;

use crate::decoder::{GeometryValues, IdValues, LogicalEncoder, RawIdValue};
use crate::encoder::IdWidth::*;
use crate::encoder::{Encoder, GeometryEncoder, IdEncoder, IdWidth, IntEncoder, StagedLayer01};
use crate::geojson::Geom32;
use crate::test_helpers::{dec, parser};
use crate::{Layer, LazyParsed, MltError, MltResult};

// Test that each config produces the correct variant and optional stream presence
#[rstest]
#[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
#[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
fn test_config_produces_correct_variant(#[case] id_width: IdWidth, #[case] ids: Vec<Option<u64>>) {
    let input = IdValues(ids);
    let raw_id = encode_id_to_raw_layer(&input, IdEncoder::new(LogicalEncoder::None, id_width));

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
    let input = ids.to_vec();
    let config = IdEncoder::new(LogicalEncoder::None, id_width);
    assert_roundtrip(&input, config);
}

#[rstest]
fn test_sequential_ids(
    #[values(LogicalEncoder::None)] logical: LogicalEncoder,
    #[values(Id32, OptId32, Id64, OptId64)] id_width: IdWidth,
) {
    let input: Vec<_> = (1..=100).map(Some).collect();
    let config = IdEncoder::new(logical, id_width);
    assert_roundtrip(&input, config);
}

proptest! {
    #[test]
    fn test_roundtrip_opt_id32(
        ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder::new(logical, OptId32))?;
    }

    #[test]
    fn test_roundtrip_id64(
        ids in prop::collection::vec(any::<u64>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder::new(logical, Id64))?;
    }

    #[test]
    fn test_roundtrip_id32(
        ids in prop::collection::vec(any::<u32>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder::new(logical, Id32))?;
    }

    #[test]
    fn test_roundtrip_opt_id64(
        ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        prop_assert_roundtrip(&ids, IdEncoder::new(logical, OptId64))?;
    }

    #[test]
    fn test_correct_variant_produced_id32(
        ids in prop::collection::vec(1u32..1000u32, 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder::new(logical, Id32))?;
    }

    #[test]
    fn test_correct_variant_produced_id64(
        ids in prop::collection::vec(any::<u64>(), 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder::new(logical, Id64))?;
    }
}

/// Round-trip `IdValues` via full layer bytes (encode → bytes → parse → decode).
/// No encoded→decoded converter; used for testing only.
fn assert_roundtrip(ids: &[Option<u64>], config: IdEncoder) {
    prop_assert_roundtrip(ids, config).expect("roundtrip failed");
}

fn prop_assert_roundtrip(ids: &[Option<u64>], config: IdEncoder) -> Result<(), TestCaseError> {
    let ids = IdValues(ids.to_vec());
    let res = roundtrip_id_values(&ids, config)
        .map_err(|e| TestCaseError::Fail(format!("Roundtrip failed: {e:?}").into()))?;
    prop_assert_eq!(res, ids.clone());
    Ok(())
}

fn roundtrip_id_values(decoded: &IdValues, config: IdEncoder) -> MltResult<IdValues> {
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
    let mut enc = Encoder::default();
    staged.encode_with(
        &mut enc,
        config,
        GeometryEncoder::all(IntEncoder::varint()),
        vec![],
    )?;
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

/// Encode `ids` into a full layer and return the parsed raw ID (leaks the buffer for lifetime).
fn encode_id_to_raw_layer(ids: &IdValues, encoder: IdEncoder) -> crate::decoder::RawId<'static> {
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
    let mut enc = Encoder::default();
    staged
        .encode_with(
            &mut enc,
            encoder,
            GeometryEncoder::all(IntEncoder::varint()),
            vec![],
        )
        .expect("encode failed");
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
    encoder: IdEncoder,
) -> Result<(), TestCaseError> {
    let input = IdValues(ids);
    let raw_id = encode_id_to_raw_layer(&input, encoder);

    if matches!(encoder.id_width, Id32 | OptId32) {
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

    if matches!(encoder.id_width, OptId32 | OptId64) {
        prop_assert!(
            raw_id.presence.0.is_some(),
            "Expected optional stream to be present"
        );
    } else {
        prop_assert!(raw_id.presence.0.is_none(), "Expected no optional stream");
    }
    Ok(())
}
