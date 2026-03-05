use mlt_core::v01::{
    DecodedId, Id, IdEncoder, IdOptimizer, IdWidth, LogicalEncoder, OwnedEncodedId,
};
use mlt_core::{FromDecoded as _, borrowme};
use rstest::rstest;

fn make_ids(values: &[Option<u64>]) -> DecodedId {
    DecodedId(Some(values.to_vec()))
}

fn u32_range_ids() -> DecodedId {
    DecodedId(Some((1u64..=100).map(Some).collect()))
}

fn u64_range_ids() -> DecodedId {
    let base = u64::from(u32::MAX) + 1;
    DecodedId(Some((base..base + 50).map(Some).collect()))
}

fn u64_range_ids_with_nulls() -> DecodedId {
    let base = u64::from(u32::MAX) + 1;
    make_ids(&[
        Some(base),
        None,
        Some(base + 1),
        Some(base + 2),
        None,
        Some(base + 3),
    ])
}

fn sequential_ids_large() -> DecodedId {
    DecodedId(Some((1u64..=1_000).map(Some).collect()))
}

fn sequential_ids_from_zero() -> DecodedId {
    DecodedId(Some((0u64..500).map(Some).collect()))
}

fn constant_ids() -> DecodedId {
    make_ids(&vec![Some(42u64); 500])
}

fn non_sequential_u64_ids() -> DecodedId {
    let base = u64::from(u32::MAX) + 1;
    make_ids(&[
        Some(base + 100),
        Some(base + 50),
        Some(base + 200),
        Some(base + 10),
        Some(base + 150),
    ])
}

#[rstest]
#[case::none_input(DecodedId(None))]
#[case::empty_vec(DecodedId(Some(vec![])))]
#[case::all_nulls(make_ids(&[None, None, None]))]
fn returns_default_encoder(#[case] decoded: DecodedId) {
    let enc = IdOptimizer::optimize(&decoded);
    assert_eq!(enc, IdEncoder::new(LogicalEncoder::None, IdWidth::Id32));
}

#[rstest]
#[case::no_nulls_u32_range(
    u32_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::with_nulls_u32_range(
    make_ids(&[Some(1), None, Some(2), Some(3), None, Some(4)]),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::OptId32)
)]
#[case::no_nulls_u64_range(
    u64_range_ids(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id64)
)]
#[case::with_nulls_u64_range(
    u64_range_ids_with_nulls(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::OptId64)
)]
#[case::max_u32_value(
    make_ids(&[Some(u64::from(u32::MAX))]),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::sequential_large(
    sequential_ids_large(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::sequential_from_zero(
    sequential_ids_from_zero(),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::constant(constant_ids(), IdEncoder::new(LogicalEncoder::Rle, IdWidth::Id32))]
#[case::constant_with_nulls(
    make_ids(&[Some(7), None, Some(7), Some(7), None]),
    IdEncoder::new(LogicalEncoder::Rle, IdWidth::OptId32)
)]
#[case::single_non_null(
    make_ids(&[Some(99)]),
    IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32)
)]
#[case::non_sequential_u32(
    make_ids(&[Some(100), Some(50), Some(200), Some(10), Some(150), Some(75)]),
    IdEncoder::new(LogicalEncoder::None, IdWidth::Id32)
)]
#[case::non_sequential_u64(
    non_sequential_u64_ids(),
    IdEncoder::new(LogicalEncoder::Delta, IdWidth::Id64)
)]
fn produces_expected_encoder(#[case] decoded: DecodedId, #[case] expected: IdEncoder) {
    let encoder = IdOptimizer::optimize(&decoded);
    assert_eq!(encoder, expected);
    let owned = OwnedEncodedId::from_decoded(&decoded, encoder).expect("encoding failed");
    let decoded_back = Id::Encoded(borrowme::borrow(&owned))
        .decode()
        .expect("decoding failed");
    assert_eq!(decoded_back, decoded);
}
