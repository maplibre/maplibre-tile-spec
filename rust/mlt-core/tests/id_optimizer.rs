use mlt_core::v01::{IdEncoder, IdProfile, IdWidth, IntEncoder, LogicalEncoder, ParsedId};
use rstest::rstest;

fn create_u32_range_ids() -> ParsedId {
    ParsedId((1u64..=100).map(Some).collect())
}

fn create_u64_range_ids() -> ParsedId {
    let base = u64::from(u32::MAX) + 1;
    ParsedId((base..base + 50).map(Some).collect())
}

fn create_ids_with_nulls() -> ParsedId {
    ParsedId(vec![Some(10), None, Some(20), None, Some(30)])
}

fn create_constant_ids() -> ParsedId {
    ParsedId(vec![Some(42), Some(42), Some(42), Some(42), Some(42)])
}

#[rstest]
#[case::empty(
    ParsedId(vec![]),
    None
)]
#[case::all_nulls(
    ParsedId(vec![None, None]),
    Some(IdEncoder::new(LogicalEncoder::None, IdWidth::Id32))
)]
#[case::short_sequence(
    ParsedId(vec![Some(1), Some(2)]),
    Some(IdEncoder::new(LogicalEncoder::None, IdWidth::Id32))
)]
#[case::sequential_u32(
    create_u32_range_ids(),
    Some(IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32))
)]
#[case::sequential_u64(
    create_u64_range_ids(),
    Some(IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id64))
)]
#[case::constant(
    create_constant_ids(),
    Some(IdEncoder::new(LogicalEncoder::Rle, IdWidth::Id32))
)]
#[case::with_nulls(
    create_ids_with_nulls(),
    Some(IdEncoder::new(LogicalEncoder::Delta, IdWidth::OptId32))
)]
fn test_automatic_optimisation_selection(
    #[case] input: ParsedId,
    #[case] expected: Option<IdEncoder>,
) {
    let (_, result_enc) = input.encode_automatic().unwrap();
    assert_eq!(result_enc, expected);
}

#[test]
fn test_automatic_optimisation_idempotency() {
    let decoded = create_u32_range_ids();

    let (encoded1_opt, enc1) = decoded.encode_automatic().unwrap();
    let (encoded2_opt, enc2) = decoded.encode_automatic().unwrap();

    assert_eq!(enc1, enc2);
    // Decode both and verify they match.
    let back1 = encoded1_opt.map(|e| ParsedId::try_from(e).unwrap());
    let back2 = encoded2_opt.map(|e| ParsedId::try_from(e).unwrap());
    assert_eq!(back1, back2);
    assert_eq!(back1.unwrap_or_default(), decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_automatic_optimisation_roundtrip(#[case] decoded: ParsedId) {
    let (encoded, _) = decoded.encode_automatic().unwrap();
    let decoded_back = encoded
        .map(|e| ParsedId::try_from(e).expect("decoding failed"))
        .unwrap_or_default();
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_applies_encoder() {
    let decoded = create_u32_range_ids();
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    let encoded = decoded.encode(manual_enc).unwrap();
    let decoded_back = encoded
        .map(|e| ParsedId::try_from(e).unwrap())
        .unwrap_or_default();
    assert_eq!(decoded_back, decoded);
}

#[test]
fn test_manual_optimisation_truncation() {
    let large_value = u64::from(u32::MAX) + 42;
    let input = ParsedId(vec![Some(large_value)]);
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
    let encoded = input.encode(manual_enc).unwrap();
    let decoded_back = encoded
        .map(|e| ParsedId::try_from(e).unwrap())
        .unwrap_or_default();

    // Manual encoding with a too-narrow `IdWidth` silently truncates values.
    // `u32::MAX + 42 == 4_294_967_337`; `4_294_967_337 % 2^32 == 41`
    assert_eq!(decoded_back.0[0], Some(41));
}

#[test]
fn test_reoptimisation_improves_manual_encoding() {
    let decoded = create_u32_range_ids();

    // First encode manually with a suboptimal encoder.
    let manual_enc = IdEncoder::new(LogicalEncoder::None, IdWidth::Id64);
    let _encoded = decoded.encode(manual_enc).unwrap();

    // Then re-encode automatically from the original decoded data.
    let (_, auto_enc) = decoded.encode_automatic().unwrap();
    let expected = IdEncoder::new(LogicalEncoder::DeltaRle, IdWidth::Id32);
    assert_eq!(auto_enc, Some(expected));
}

#[test]
fn test_profile_applies_candidates_and_rederives_width() {
    let u32_sample = create_u32_range_ids();
    let profile = IdProfile::from_sample(&u32_sample);

    let u64_decoded = create_u64_range_ids();
    let (_, enc_opt) = u64_decoded.encode_with_profile(&profile).unwrap();
    let enc = enc_opt.unwrap();

    // Width must reflect the u64 tile data, not the u32 sample.
    assert_eq!(enc.id_width, IdWidth::Id64);
    // Both samples are sequential (>4 values), so the fast path fires: DeltaRle.
    assert_eq!(enc.logical, LogicalEncoder::DeltaRle);
}

#[test]
fn test_profile_already_encoded_roundtrip() {
    let decoded = create_u32_range_ids();
    let profile = IdProfile::from_sample(&decoded);
    let (encoded, _) = decoded.encode_with_profile(&profile).unwrap();
    let decoded_back = encoded
        .map(|e| ParsedId::try_from(e).unwrap())
        .unwrap_or_default();
    assert_eq!(decoded_back, decoded);
}

#[rstest]
#[case::sequential_u32(create_u32_range_ids())]
#[case::sequential_u64(create_u64_range_ids())]
#[case::constant(create_constant_ids())]
#[case::with_nulls(create_ids_with_nulls())]
fn test_profile_roundtrip(#[case] decoded: ParsedId) {
    let profile = IdProfile::from_sample(&decoded);
    let (encoded, _) = decoded.encode_with_profile(&profile).unwrap();
    let decoded_back = encoded
        .map(|e| ParsedId::try_from(e).unwrap())
        .unwrap_or_default();
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
