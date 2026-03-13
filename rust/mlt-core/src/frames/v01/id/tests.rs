#![cfg(test)]

use IdWidth::*;
use proptest::prelude::*;
use rstest::rstest;

use crate::encode::FromDecoded as _;
use crate::frames::v01::id::encode::IdEncoder;
use crate::frames::v01::id::model::{EncodedId, EncodedIdValue, IdWidth, ParsedId};
use crate::v01::LogicalEncoder;

// Helper function to encode and decode for roundtrip testing
fn roundtrip(decoded: &ParsedId, config: IdEncoder) -> ParsedId {
    match decoded.clone().encode(config).expect("Failed to encode") {
        Some(encoded) => ParsedId::try_from(encoded).expect("Failed to decode"),
        None => ParsedId(vec![]), // empty list encodes to nothing and decodes to empty
    }
}

// Test that each config produces the correct variant and optional stream presence
#[rstest]
#[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
#[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
#[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
fn test_config_produces_correct_variant(#[case] id_width: IdWidth, #[case] ids: Vec<Option<u64>>) {
    let input = ParsedId(ids);
    let config = IdEncoder {
        logical: LogicalEncoder::None,
        id_width,
    };
    let encoded = EncodedId::from_decoded(&input, config).unwrap();

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
    let input = ParsedId(ids.to_vec());
    let config = IdEncoder {
        logical: LogicalEncoder::None,
        id_width,
    };
    let output = roundtrip(&input, config);
    assert_eq!(output, input);
}

#[rstest]
fn test_sequential_ids(
    #[values(LogicalEncoder::None)] logical: LogicalEncoder,
    #[values(Id32, OptId32, Id64, OptId64)] id_width: IdWidth,
) {
    let input = ParsedId((1..=100).map(Some).collect());
    let config = IdEncoder { logical, id_width };
    let output = roundtrip(&input, config);
    assert_eq!(output, input);
}

proptest! {
    #[test]
    fn test_roundtrip_id32(
        ids in prop::collection::vec(any::<u32>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: Id32,logical})?;
    }

    #[test]
    fn test_roundtrip_opt_id32(
        ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
        assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: OptId32,logical})?;
    }

    #[test]
    fn test_roundtrip_id64(
        ids in prop::collection::vec(any::<u64>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: Id64, logical})?;
    }

    #[test]
    fn test_roundtrip_opt_id64(
        ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        assert_roundtrip_succeeds(ids, IdEncoder{ id_width: OptId64, logical})?;
    }

    #[test]
    fn test_encodable_trait_api_id32(
        ids in prop::collection::vec(any::<u32>(), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_encodable_api_works(ids_u64, IdEncoder{ id_width: Id32,logical})?;
    }

    #[test]
    fn test_encodable_trait_api_opt_id64(
        ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
        logical in any::<LogicalEncoder>()
    ) {
        assert_encodable_api_works(ids, IdEncoder{ id_width: OptId64,logical})?;
    }

    #[test]
    fn test_correct_variant_produced_id32(
        ids in prop::collection::vec(1u32..1000u32, 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder{ id_width: Id32,logical})?;
    }

    #[test]
    fn test_correct_variant_produced_id64(
        ids in prop::collection::vec(any::<u64>(), 1..50),
        logical in any::<LogicalEncoder>()
    ) {
        let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
        assert_produces_correct_variant(ids_u64, IdEncoder{ id_width: Id64,logical})?;
    }
}

fn assert_roundtrip_succeeds(
    ids: Vec<Option<u64>>,
    config: IdEncoder,
) -> Result<(), TestCaseError> {
    let input = ParsedId(ids.clone());
    let output = roundtrip(&input, config);
    prop_assert_eq!(output, ParsedId(ids));
    Ok(())
}

fn assert_encodable_api_works(
    ids: Vec<Option<u64>>,
    config: IdEncoder,
) -> Result<(), TestCaseError> {
    let decoded = ParsedId(ids.clone());

    let encoded = decoded
        .encode(config)
        .expect("Failed to encode")
        .expect("non-empty IDs should produce Some(EncodedId)");

    let decoded_back = ParsedId::try_from(encoded).expect("Failed to decode");
    prop_assert_eq!(decoded_back, ParsedId(ids));
    Ok(())
}

fn assert_produces_correct_variant(
    ids: Vec<Option<u64>>,
    encoder: IdEncoder,
) -> Result<(), TestCaseError> {
    let input = ParsedId(ids);
    let enc_id = EncodedId::from_decoded(&input, encoder).expect("Failed to encode");

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
