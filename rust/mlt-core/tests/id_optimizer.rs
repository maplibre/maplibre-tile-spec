use mlt_core::MltError;
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{DecodedId, IdEncoder, IdWidth, LogicalEncoder, OwnedId};
use mlt_core::{borrowme};
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
fn test_profile_optimisation_unimplemented() {
    let mut owned = OwnedId::Decoded(Some(create_u32_range_ids()));
    let result = owned.profile_driven_optimisation(&());
    assert!(matches!(result, Err(MltError::NotImplemented(_))));
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
