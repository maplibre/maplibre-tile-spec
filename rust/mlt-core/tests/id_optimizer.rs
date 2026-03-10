use mlt_core::borrowme;
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{
    DecodedId, IdEncoder, IdProfile, IdWidth, IntEncoder, LogicalEncoder, OwnedId, PhysicalEncoder,
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

#[rstest]
#[case::empty(
    DecodedId(vec![]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::all_nulls(
    DecodedId(vec![None, None]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::sequential_u32(
    create_u32_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::large_values(
    create_u64_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id64)
)]
#[case::with_nulls(
    create_ids_with_nulls(),
    IdEncoder::new(LogicalEncoder::Delta, IdWidth::OptId32)
)]
fn test_automatic_optimisation_selection(#[case] input: DecodedId, #[case] expected: IdEncoder) {
    let mut owned = OwnedId::Decoded(Some(input));
    let result = owned.automatic_encoding_optimisation().unwrap();
    assert_eq!(result, Some(expected));
    assert!(matches!(owned, OwnedId::Encoded(Some(_))));
}

#[test]
fn test_automatic_optimisation_idempotency() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(Some(decoded.clone()));

    let enc1 = owned.automatic_encoding_optimisation().unwrap();
    let enc2 = owned.automatic_encoding_optimisation().unwrap();

    assert_eq!(enc1, enc2);
    assert_eq!(borrowme::borrow(&owned).decode().unwrap(), decoded);
}

#[test]
fn test_automatic_optimisation_none_variant() {
    let mut owned = OwnedId::Decoded(None);
    let result = owned.automatic_encoding_optimisation().unwrap();

    assert_eq!(result, None);
    assert!(matches!(owned, OwnedId::Encoded(None)));
}

#[test]
fn test_manual_optimisation_applies_encoder() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(Some(decoded.clone()));

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    owned.manual_optimisation(manual_enc).unwrap();

    assert!(!matches!(owned, OwnedId::Decoded(_)));
    assert_eq!(borrowme::borrow(&owned).decode().unwrap(), decoded);
}

#[test]
fn test_manual_optimisation_override() {
    let mut owned = OwnedId::Decoded(Some(create_u32_range_ids()));

    owned.automatic_encoding_optimisation().unwrap();

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    owned.manual_optimisation(manual_enc).unwrap();

    assert!(!matches!(owned, OwnedId::Decoded(_)));
}

#[test]
fn test_manual_optimisation_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let mut owned = OwnedId::Decoded(Some(DecodedId(vec![Some(large_value)])));

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    owned.manual_optimisation(manual_enc).unwrap();

    let decoded_back = borrowme::borrow(&owned).decode().unwrap();
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_reoptimisation_improves_manual_encoding() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(Some(decoded));

    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    owned.manual_optimisation(manual_enc).unwrap();

    let auto_enc = owned.automatic_encoding_optimisation().unwrap();
    let expected = IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32);
    assert_eq!(auto_enc, Some(expected));
}

#[test]
fn test_profile_applies_candidates_and_rederives_width() {
    // Profile built from u32 data; applied to u64 data.
    // Logical encoding comes from the profile; IdWidth is re-derived from the tile.
    let u32_sample = create_u32_range_ids();
    let profile = IdProfile::from_sample(&u32_sample);

    let u64_decoded = create_u64_range_ids();
    let mut owned = OwnedId::Decoded(Some(u64_decoded.clone()));
    let result = owned.profile_driven_optimisation(&profile).unwrap();

    // Width must reflect the actual u64 data, not the u32 sample.
    let enc = result.unwrap();
    assert_eq!(enc.id_width, IdWidth::Id64);
    assert_eq!(borrowme::borrow(&owned).decode().unwrap(), u64_decoded);
}

#[test]
fn test_profile_none_variant() {
    let mut owned = OwnedId::Decoded(None);
    let profile = IdProfile::from_sample(&create_u32_range_ids());

    let result = owned.profile_driven_optimisation(&profile).unwrap();

    assert_eq!(result, None);
    assert!(matches!(owned, OwnedId::Encoded(None)));
}

#[test]
fn test_profile_roundtrip() {
    for decoded in [
        create_u32_range_ids(),
        create_u64_range_ids(),
        create_ids_with_nulls(),
    ] {
        let profile = IdProfile::from_sample(&decoded);
        let mut owned = OwnedId::Decoded(Some(decoded.clone()));
        owned.profile_driven_optimisation(&profile).unwrap();

        let decoded_back = borrowme::borrow(&owned).decode().expect("decode failed");
        assert_eq!(decoded_back, decoded);
    }
}

#[test]
fn test_profile_from_sample_is_nonempty() {
    let profile = IdProfile::from_sample(&create_u32_range_ids());
    assert!(!profile.candidates.is_empty());
}

#[test]
fn test_profile_merge_is_union() {
    let p1 = IdProfile {
        candidates: vec![IntEncoder::varint()],
    };
    let p2 = IdProfile {
        candidates: vec![
            IntEncoder::varint(),
            IntEncoder::new(LogicalEncoder::Rle, PhysicalEncoder::VarInt),
        ],
    };
    let merged = p1.merge(&p2);
    assert_eq!(merged.candidates.len(), 2);
    assert!(merged.candidates.contains(&IntEncoder::varint()));
    assert!(merged.candidates.contains(&IntEncoder::new(
        LogicalEncoder::Rle,
        PhysicalEncoder::VarInt
    )));
}

#[test]
fn test_profile_already_encoded_roundtrip() {
    let decoded = create_u32_range_ids();
    let mut owned = OwnedId::Decoded(Some(decoded.clone()));
    owned.automatic_encoding_optimisation().unwrap();

    // Re-optimise from encoded state using a profile.
    let profile = IdProfile::from_sample(&decoded);
    owned.profile_driven_optimisation(&profile).unwrap();

    let decoded_back = borrowme::borrow(&owned).decode().unwrap();
    assert_eq!(decoded_back, decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_optimisation_roundtrip(#[case] decoded: DecodedId) {
    let mut owned = OwnedId::Decoded(Some(decoded.clone()));
    owned.automatic_encoding_optimisation().unwrap();

    let decoded_back = borrowme::borrow(&owned).decode().expect("decoding failed");
    assert_eq!(decoded_back, decoded);
}
