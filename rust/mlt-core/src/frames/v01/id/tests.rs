#![cfg(test)]

use IdWidth::*;
use geo_types::Point;
use proptest::prelude::*;
use rstest::rstest;

use crate::frames::v01::id::encode::IdEncoder;
use crate::frames::v01::id::model::{EncodedId, EncodedIdValue, IdValues, IdWidth};
use crate::geojson::Geom32;
use crate::v01::{
    GeometryEncoder, GeometryValues, IntEncoder, LogicalEncoder, StagedLayer01,
    StagedLayer01Encoder,
};
use crate::{Decoder, EncodedLayer, Layer, MemBudget, MltError};

// Test that each config produces the correct variant and optional stream presence
#[rstest]
#[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
#[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
fn test_config_produces_correct_variant(#[case] id_width: IdWidth, #[case] ids: Vec<Option<u64>>) {
    let input = IdValues(ids);
    let config = IdEncoder {
        logical: LogicalEncoder::None,
        id_width,
    };
    let encoded = EncodedId::encode(&input, config).unwrap();

    match id_width {
        OptId32 | Id32 => assert!(matches!(encoded.value, EncodedIdValue::Id32(_))),
        Id64 | OptId64 => assert!(matches!(encoded.value, EncodedIdValue::Id64(_))),
    }

    match id_width {
        OptId32 | OptId64 => assert!(encoded.presence.is_some()),
        Id32 | Id64 => assert!(encoded.presence.is_none()),
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
    let config = IdEncoder {
        logical: LogicalEncoder::None,
        id_width,
    };
    assert_roundtrip(&input, config);
}

#[rstest]
fn test_sequential_ids(
    #[values(LogicalEncoder::None)] logical: LogicalEncoder,
    #[values(Id32, OptId32, Id64, OptId64)] id_width: IdWidth,
) {
    let input: Vec<_> = (1..=100).map(Some).collect();
    let config = IdEncoder { logical, id_width };
    assert_roundtrip(&input, config);
}

proptest! {
    #[test]
    fn test_roundtrip_opt_id32(
        ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder { id_width: OptId32, logical })?;
    }

    #[test]
    fn test_roundtrip_id64(
        ids in prop::collection::vec(any::<u64>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder { id_width: Id64, logical })?;
    }

    #[test]
    fn test_roundtrip_id32(
        ids in prop::collection::vec(any::<u32>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        prop_assert_roundtrip(&ids_u64, IdEncoder { id_width: Id32, logical })?;
    }

    #[test]
    fn test_roundtrip_opt_id64(
        ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        prop_assert_roundtrip(&ids, IdEncoder { id_width: OptId64, logical })?;
    }

    #[test]
    fn test_correct_variant_produced_id32(
        ids in prop::collection::vec(1u32..1000u32, 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder { id_width: Id32, logical })?;
    }

    #[test]
    fn test_correct_variant_produced_id64(
        ids in prop::collection::vec(any::<u64>(), 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder { id_width: Id64, logical })?;
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

fn roundtrip_id_values(decoded: &IdValues, config: IdEncoder) -> Result<IdValues, MltError> {
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
    let stream_encoder = StagedLayer01Encoder {
        id: Some(config),
        geometry: GeometryEncoder::all(IntEncoder::varint()),
        properties: vec![],
    };
    let layer_enc = staged.encode(stream_encoder)?;
    let mut buf = Vec::new();
    EncodedLayer::Tag01(layer_enc)
        .write_to(&mut buf)
        .map_err(MltError::from)?;
    let (_, layer) = Layer::from_bytes(&buf, &mut MemBudget::default())?;
    let Layer::Tag01(layer01) = layer else {
        return Err(MltError::NotDecoded("expected Tag01 layer"));
    };
    let id = layer01
        .id
        .ok_or(MltError::NotDecoded("expected id column"))?;
    id.into_parsed(&mut Decoder::default())
}

fn assert_produces_correct_variant(
    ids: Vec<Option<u64>>,
    encoder: IdEncoder,
) -> Result<(), TestCaseError> {
    let input = IdValues(ids);
    let enc_id = EncodedId::encode(&input, encoder).expect("Failed to encode");

    if matches!(encoder.id_width, Id32 | OptId32) {
        prop_assert!(
            matches!(enc_id.value, EncodedIdValue::Id32(_)),
            "Expected Id32 variant"
        );
    } else {
        prop_assert!(
            matches!(enc_id.value, EncodedIdValue::Id64(_)),
            "Expected Id64 variant"
        );
    }

    if matches!(encoder.id_width, OptId32 | OptId64) {
        prop_assert!(
            enc_id.presence.is_some(),
            "Expected optional stream to be present"
        );
    } else {
        prop_assert!(enc_id.presence.is_none(), "Expected no optional stream");
    }
    Ok(())
}
