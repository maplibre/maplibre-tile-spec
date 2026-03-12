use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{
    DecodedId, IdEncoder, IdProfile, IdWidth, IntEncoder, LogicalEncoder, OwnedId,
};
use rstest::rstest;

fn create_u32_range_ids() -> DecodedId {
    DecodedId((1u64..=100).map(Some).collect())
}

fn create_u64_range_ids() -> DecodedId {
    let base = u64::from(u32::MAX) + 1;
    DecodedId((base..base + 50).map(Some).collect())
}

fn create_ids_with_nulls() -> DecodedId {
    DecodedId(vec![Some(10), None, Some(20), None, Some(30)])
}

fn create_constant_ids() -> DecodedId {
    DecodedId(vec![Some(42), Some(42), Some(42), Some(42), Some(42)])
}

#[rstest]
#[case::empty(
    DecodedId(vec![]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::all_nulls(
    DecodedId(vec![None, None]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::short_sequence(
    DecodedId(vec![Some(1), Some(2)]),
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
fn test_automatic_optimisation_selection(#[case] input: DecodedId, #[case] expected: IdEncoder) {
    let mut owned = OwnedId::Decoded(input);
    let result = owned.automatic_encoding_optimisation().unwrap();
    assert_eq!(result, Some(expected));
    assert!(matches!(owned, OwnedId::Encoded(_)));
}

#[test]
fn test_automatic_optimisation_idempotency() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(decoded.clone());

    let enc1 = owned.automatic_encoding_optimisation().unwrap();
    let enc2 = owned.automatic_encoding_optimisation().unwrap();

    assert_eq!(enc1, enc2);
    assert_eq!(DecodedId::try_from(owned).unwrap(), decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_optimisation_roundtrip(#[case] decoded: DecodedId) {
    let mut owned = OwnedId::Decoded(decoded.clone());
    owned.automatic_encoding_optimisation().unwrap();

    let decoded_back = DecodedId::try_from(owned).expect("decoding failed");
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_applies_encoder() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(decoded.clone());

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    owned.manual_optimisation(manual_enc).unwrap();

    assert!(matches!(owned, OwnedId::Encoded(_)));
    assert_eq!(DecodedId::try_from(owned).unwrap(), decoded);
}

#[test]
fn test_manual_optimisation_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let mut owned = OwnedId::Decoded(DecodedId(vec![Some(large_value)]));

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    owned.manual_optimisation(manual_enc).unwrap();

    let decoded_back = DecodedId::try_from(owned).unwrap();

    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_reoptimisation_improves_manual_encoding() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(decoded);

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    owned.manual_optimisation(manual_enc).unwrap();

    let auto_enc = owned.automatic_encoding_optimisation().unwrap();
    let expected = IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32);
    assert_eq!(auto_enc, Some(expected));
}

#[test]
fn test_profile_applies_candidates_and_rederives_width() {
    let u32_sample = create_u32_range_ids();
    let profile = IdProfile::from_sample(&u32_sample);

    let u64_decoded = create_u64_range_ids();
    let mut owned = OwnedId::Decoded(u64_decoded.clone());
    let enc = owned
        .profile_driven_optimisation(&profile)
        .unwrap()
        .unwrap();

    // Width must reflect the u64 tile data, not the u32 sample.
    assert_eq!(enc.id_width, IdWidth::Id64);
    // Both samples are sequential (>4 values), so the fast path fires: DeltaRle.
    assert_eq!(enc.logical, LogicalEncoder::DeltaRle);
    assert_eq!(DecodedId::try_from(owned).unwrap(), u64_decoded);
}

#[test]
fn test_profile_already_encoded_roundtrip() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(decoded.clone());
    owned.automatic_encoding_optimisation().unwrap();

    let profile = IdProfile::from_sample(&decoded);
    owned.profile_driven_optimisation(&profile).unwrap();

    let decoded_back = DecodedId::try_from(owned).unwrap();
    assert_eq!(decoded_back, decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_profile_roundtrip(#[case] decoded: DecodedId) {
    let profile = IdProfile::from_sample(&decoded);
    let mut owned = OwnedId::Decoded(decoded.clone());
    owned.profile_driven_optimisation(&profile).unwrap();

    let decoded_back = DecodedId::try_from(owned).expect("decode failed");
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
