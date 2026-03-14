use mlt_core::v01::{IdEncoder, IdProfile, IdValues, IdWidth, IntEncoder, LogicalEncoder};
use rstest::rstest;

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
    let (encoded, _) = decoded.clone().encode_auto().unwrap();
    let encoded = encoded.expect("non-empty IDs should produce Some");
    let decoded_back = IdValues::try_from(encoded).expect("decoding failed");
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_applies_encoder() {
    let decoded = create_u32_range_ids();
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    let encoded = decoded
        .clone()
        .encode(manual_enc)
        .unwrap()
        .expect("should produce encoded");
    let decoded_back = IdValues::try_from(encoded).unwrap();
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let ids = IdValues(vec![Some(large_value)]);
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    let encoded = ids
        .encode(manual_enc)
        .unwrap()
        .expect("should produce encoded");
    let decoded_back = IdValues::try_from(encoded).unwrap();
    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_profile_applies_candidates_and_rederives_width() {
    let u32_sample = create_u32_range_ids();
    let profile = IdProfile::from_sample(&u32_sample);

    let u64_decoded = create_u64_range_ids();
    let (encoded, enc) = u64_decoded.clone().encode_with_profile(&profile).unwrap();
    let enc = enc.unwrap();
    let encoded = encoded.unwrap();

    // Width must reflect the u64 tile data, not the u32 sample.
    assert_eq!(enc.id_width, IdWidth::Id64);
    // Both samples are sequential (>4 values), so the fast path fires: DeltaRle.
    assert_eq!(enc.logical, LogicalEncoder::DeltaRle);
    assert_eq!(IdValues::try_from(encoded).unwrap(), u64_decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_profile_roundtrip(#[case] decoded: IdValues) {
    let profile = IdProfile::from_sample(&decoded);
    let (encoded, _) = decoded.clone().encode_with_profile(&profile).unwrap();
    let encoded = encoded.expect("non-empty IDs should produce Some");
    let decoded_back = IdValues::try_from(encoded).unwrap();
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
