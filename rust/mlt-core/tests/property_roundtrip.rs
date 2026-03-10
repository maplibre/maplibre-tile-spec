use mlt_core::v01::{
    DecodedProperty, DecodedStrings, IntEncoder, LogicalEncoder, OwnedEncodedProperty,
    PhysicalEncoder, PresenceStream, PropValue, Property, PropertyEncoder, ScalarEncoder,
    SharedDictEncoder, SharedDictItemEncoder, StrEncoder, build_decoded_shared_dict,
};
use mlt_core::{FromDecoded as _, FromEncoded as _, MltError, borrowme};
use proptest::prelude::*;

// proptest_derive::Arbitrary is only derived for these types inside the crate
// under #[cfg(test)], so we write the strategies by hand here.

fn arb_logical_encoder() -> impl Strategy<Value = LogicalEncoder> {
    prop_oneof![
        Just(LogicalEncoder::None),
        Just(LogicalEncoder::Delta),
        Just(LogicalEncoder::DeltaRle),
        Just(LogicalEncoder::Rle),
    ]
}

fn arb_physical_encoder() -> impl Strategy<Value = PhysicalEncoder> {
    prop_oneof![
        Just(PhysicalEncoder::None),
        Just(PhysicalEncoder::VarInt),
        Just(PhysicalEncoder::FastPFOR),
    ]
}

/// [`PhysicalEncoder`] variants that are valid for 64-bit integers
/// (i.e. everything except `FastPFOR`).
fn arb_physical_no_fastpfor() -> impl Strategy<Value = PhysicalEncoder> {
    prop_oneof![Just(PhysicalEncoder::None), Just(PhysicalEncoder::VarInt),]
}

fn arb_int_encoder() -> impl Strategy<Value = IntEncoder> {
    (arb_logical_encoder(), arb_physical_encoder())
        .prop_map(|(logical, physical)| IntEncoder::new(logical, physical))
}

/// [`IntEncoder`] strategy that excludes `FastPFOR`, which only handles 32-bit integers.
fn arb_int_encoder_no_fastpfor() -> impl Strategy<Value = IntEncoder> {
    (arb_logical_encoder(), arb_physical_no_fastpfor())
        .prop_map(|(logical, physical)| IntEncoder::new(logical, physical))
}

fn arb_str_encoder() -> impl Strategy<Value = StrEncoder> {
    prop_oneof![
        arb_int_encoder().prop_map(StrEncoder::plain),
        (arb_int_encoder(), arb_int_encoder()).prop_map(|(sym, dict)| StrEncoder::fsst(sym, dict)),
    ]
}

fn roundtrip(decoded: &DecodedProperty<'_>, encoder: ScalarEncoder) -> DecodedProperty<'static> {
    let owned = mlt_core::borrowme::ToOwned::to_owned(decoded);
    let enc = OwnedEncodedProperty::from_decoded(&owned, encoder).expect("encoding failed");
    mlt_core::borrowme::ToOwned::to_owned(
        &DecodedProperty::from_encoded(borrowme::borrow(&enc)).expect("decoding failed"),
    )
}

fn strs(vals: &[&str]) -> Vec<Option<String>> {
    vals.iter().map(|v| Some((*v).to_string())).collect()
}

fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
    vals.iter().map(|v| v.map(ToString::to_string)).collect()
}

fn str_prop(name: &str, values: Vec<Option<String>>) -> DecodedProperty<'static> {
    DecodedProperty::from_parts(
        name.to_string(),
        PropValue::Str(DecodedStrings::from(values)),
    )
}

fn shared_dict_prop(
    name: &str,
    children: Vec<(String, DecodedStrings<'static>)>,
) -> DecodedProperty<'static> {
    let (shared_dict, items) = build_decoded_shared_dict(children).expect("build shared dict");
    DecodedProperty::SharedDict(name.to_string(), shared_dict, items)
}

fn make_prop(name: &str, values: PropValue) -> DecodedProperty<'static> {
    DecodedProperty::from_parts(name.to_string(), values)
}

fn decode_struct(prop: &OwnedEncodedProperty) -> DecodedProperty<'static> {
    mlt_core::borrowme::ToOwned::to_owned(
        &Property::from(borrowme::borrow(prop))
            .decode()
            .expect("decode failed"),
    )
}

fn decode_scalar(prop: &OwnedEncodedProperty) -> DecodedProperty<'static> {
    mlt_core::borrowme::ToOwned::to_owned(
        &DecodedProperty::from_encoded(borrowme::borrow(prop)).expect("decode failed"),
    )
}

fn struct_encode_and_decode(
    struct_name: &str,
    children: &[(&str, Vec<Option<String>>)],
    presence: PresenceStream,
    offset_encoder: IntEncoder,
    dict_encoder: StrEncoder,
) -> DecodedProperty<'static> {
    // Build a single DecodedProperty with PropValue::SharedDict
    let items = children
        .iter()
        .map(|(suffix, values)| ((*suffix).to_string(), DecodedStrings::from(values.clone())))
        .collect();
    let decoded = vec![shared_dict_prop(struct_name, items)];

    // Build encoder with matching item encoders
    let item_encoders: Vec<SharedDictItemEncoder> = children
        .iter()
        .map(|_| SharedDictItemEncoder {
            presence,
            offsets: offset_encoder,
        })
        .collect();
    let shared_enc = SharedDictEncoder {
        dict_encoder,
        items: item_encoders,
    };

    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, vec![shared_enc.into()])
        .expect("encoding failed");
    assert_eq!(encoded.len(), 1, "should produce one encoded property");
    decode_struct(&encoded[0])
}

// Absent mode has no presence stream on the wire, so only all-Some inputs are
// valid for those variants.
macro_rules! integer_roundtrip_proptests {
    ($present:ident, $absent:ident, $variant:ident, $ty:ty, $int_encoder:expr) => {
        proptest! {
            #[test]
            fn $present(
                values in prop::collection::vec(prop::option::of(any::<$ty>()), 0..100),
                enc in $int_encoder,
            ) {
                let prop = make_prop("x", PropValue::$variant(values));
                let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
                prop_assert_eq!(roundtrip(&prop, scalar_enc), prop);
            }

            #[test]
            fn $absent(
                values in prop::collection::vec(any::<$ty>(), 0..100),
                enc in $int_encoder,
            ) {
                let opt: Vec<Option<$ty>> = values.into_iter().map(Some).collect();
                let prop = make_prop("x", PropValue::$variant(opt));
                let scalar_enc = ScalarEncoder::int(PresenceStream::Absent, enc);
                prop_assert_eq!(roundtrip(&prop, scalar_enc), prop);
            }
        }
    };
}

// i8, u8, i32, u32 — all physical encoders are valid.
integer_roundtrip_proptests!(i8_present, i8_absent, I8, i8, arb_int_encoder());
integer_roundtrip_proptests!(u8_present, u8_absent, U8, u8, arb_int_encoder());
integer_roundtrip_proptests!(i32_present, i32_absent, I32, i32, arb_int_encoder());
integer_roundtrip_proptests!(u32_present, u32_absent, U32, u32, arb_int_encoder());
// FastPFOR does not support 64-bit integers.
integer_roundtrip_proptests!(
    i64_present,
    i64_absent,
    I64,
    i64,
    arb_int_encoder_no_fastpfor()
);
integer_roundtrip_proptests!(
    u64_present,
    u64_absent,
    U64,
    u64,
    arb_int_encoder_no_fastpfor()
);

#[test]
fn bool_specific_values() {
    let prop = make_prop(
        "active",
        PropValue::Bool(vec![Some(true), None, Some(false), Some(true), None]),
    );
    assert_eq!(
        roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
        prop
    );
}

#[test]
fn bool_all_null() {
    let prop = make_prop("active", PropValue::Bool(vec![None, None, None]));
    assert_eq!(
        roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
        prop
    );
}

proptest! {
    #[test]
    fn bool_roundtrip(
        values in prop::collection::vec(prop::option::of(any::<bool>()), 0..100),
    ) {
        let prop = make_prop("flag", PropValue::Bool(values));
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)), prop);
    }
}

// NaN is excluded because NaN != NaN.
proptest! {
    #[test]
    fn f32_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f32>().prop_filter("no NaN", |f| !f.is_nan())),
            0..100,
        ),
    ) {
        let prop = make_prop("score", PropValue::F32(values));
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::float(PresenceStream::Present)), prop);
    }

    #[test]
    fn f64_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f64>().prop_filter("no NaN", |f| !f.is_nan())),
            0..100,
        ),
    ) {
        let prop = make_prop("score", PropValue::F64(values));
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::float(PresenceStream::Present)), prop);
    }
}

#[test]
fn str_scalar_with_nulls() {
    let prop = str_prop(
        "city",
        opt_strs(&[Some("Berlin"), None, Some("Hamburg"), None]),
    );
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    assert_eq!(roundtrip(&prop, enc), prop);
}

#[test]
fn str_scalar_all_null() {
    let prop = str_prop("city", opt_strs(&[None, None, None]));
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    assert_eq!(roundtrip(&prop, enc), prop);
}

#[test]
fn str_scalar_empty() {
    let prop = str_prop("unused", vec![]);
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    assert_eq!(roundtrip(&prop, enc), prop);
}

proptest! {
    #[test]
    fn str_scalar_roundtrip(
        values in prop::collection::vec(
            prop::option::of("[a-zA-Z0-9 ]{0,30}"),
            0..50,
        ),
    ) {
        let prop = str_prop("name", values);
        let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
        prop_assert_eq!(roundtrip(&prop, enc), prop);
    }
}

#[test]
fn fsst_scalar_string_roundtrip() {
    // Repeated "Br" prefix gives FSST something meaningful to compress.
    let enc = ScalarEncoder::str_fsst(
        PresenceStream::Present,
        IntEncoder::plain(),
        IntEncoder::plain(),
    );
    let prop = str_prop(
        "name",
        strs(&["Berlin", "Brandenburg", "Bremen", "Braunschweig"]),
    );
    assert_eq!(roundtrip(&prop, enc), prop);
}

#[test]
fn fsst_struct_shared_dict_roundtrip() {
    let de = strs(&["Berlin", "München", "Köln"]);
    let en = strs(&["Berlin", "Munich", "Cologne"]);
    let result = struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
    );
    assert_eq!(result.name(), "name");
    let DecodedProperty::SharedDict(_, shared_dict, items) = &result else {
        panic!("Expected SharedDict");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].suffix, ":de");
    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].suffix, ":en");
    assert_eq!(items[1].materialize(shared_dict), en);
}

#[test]
fn struct_with_nulls() {
    let de = opt_strs(&[Some("Berlin"), Some("München"), None]);
    let en = opt_strs(&[Some("Berlin"), None, Some("London")]);
    let result = struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
    );
    assert_eq!(result.name(), "name");
    let DecodedProperty::SharedDict(_, shared_dict, items) = &result else {
        panic!("Expected SharedDict");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].suffix, ":de");
    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].suffix, ":en");
    assert_eq!(items[1].materialize(shared_dict), en);
}

#[test]
fn struct_shared_dict_inline_ranges_track_nulls_and_empty_strings() {
    let de = opt_strs(&[Some(""), None, Some("Berlin")]);
    let en = opt_strs(&[Some(""), Some("Berlin"), Some("")]);
    let prop = shared_dict_prop(
        "name",
        vec![
            (":de".to_string(), DecodedStrings::from(de.clone())),
            (":en".to_string(), DecodedStrings::from(en.clone())),
        ],
    );
    let DecodedProperty::SharedDict(_, shared_dict, items) = &prop else {
        panic!("Expected SharedDict");
    };

    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].materialize(shared_dict), en);

    assert_eq!(items[0].ranges[1], (-1, -1));
    assert_eq!(items[0].get(shared_dict, 1), None);

    let empty_de = items[0].ranges[0];
    let empty_en = items[1].ranges[0];
    assert_ne!(empty_de, (-1, -1));
    assert_ne!(empty_en, (-1, -1));
    assert_eq!(empty_de.0, empty_de.1);
    assert_eq!(empty_en.0, empty_en.1);
}

#[test]
fn struct_no_nulls() {
    let de = strs(&["Berlin", "München", "Hamburg"]);
    let en = strs(&["Berlin", "Munich", "Hamburg"]);
    let result = struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
    );
    assert_eq!(result.name(), "name");
    let DecodedProperty::SharedDict(_, shared_dict, items) = &result else {
        panic!("Expected SharedDict");
    };
    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].materialize(shared_dict), en);
}

#[test]
fn struct_shared_dict_deduplication() {
    let de = strs(&["Berlin", "Berlin"]);
    let en = strs(&["Berlin", "London"]);
    let result = struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
    );
    let DecodedProperty::SharedDict(_, shared_dict, items) = &result else {
        panic!("Expected SharedDict");
    };
    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].materialize(shared_dict), en);
}

#[test]
fn struct_mixed_with_scalars() {
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(enc);
    let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
    let population = make_prop(
        "population",
        PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
    );
    let name_shared = shared_dict_prop(
        "name:",
        vec![
            (
                "de".to_string(),
                DecodedStrings::from(
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                DecodedStrings::from(
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );
    let rank = make_prop("rank", PropValue::U32(vec![Some(1), Some(2)]));

    let props = vec![population.clone(), name_shared.clone(), rank.clone()];
    let prop_encs = vec![
        PropertyEncoder::Scalar(scalar_enc),
        SharedDictEncoder {
            dict_encoder: str_enc,
            items: vec![
                SharedDictItemEncoder {
                    presence: PresenceStream::Present,
                    offsets: enc,
                },
                SharedDictItemEncoder {
                    presence: PresenceStream::Present,
                    offsets: enc,
                },
            ],
        }
        .into(),
        PropertyEncoder::Scalar(scalar_enc),
    ];
    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(&props, prop_encs).unwrap();

    // Output order: scalar "population", struct "name:", scalar "rank"
    assert_eq!(encoded.len(), 3);
    assert_eq!(decode_scalar(&encoded[0]), population);
    let name = decode_struct(&encoded[1]);
    assert_eq!(name.name(), "name:");
    let DecodedProperty::SharedDict(_, shared_dict, items) = &name else {
        panic!("Expected SharedDict");
    };
    assert_eq!(items[0].suffix, "de");
    assert_eq!(
        items[0].materialize(shared_dict),
        strs(&["Berlin", "Hamburg"])
    );
    assert_eq!(items[1].suffix, "en");
    assert_eq!(
        items[1].materialize(shared_dict),
        strs(&["Berlin", "Hamburg"])
    );
    assert_eq!(decode_scalar(&encoded[2]), rank);
}

#[test]
fn two_struct_groups_with_scalar_between() {
    let name_shared = shared_dict_prop(
        "name:",
        vec![
            (
                "de".to_string(),
                DecodedStrings::from(
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                DecodedStrings::from(
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );
    let population = make_prop(
        "population",
        PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
    );
    let label_shared = shared_dict_prop(
        "label:",
        vec![
            (
                "de".to_string(),
                DecodedStrings::from(
                    strs(&["BE", "HH"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                DecodedStrings::from(
                    strs(&["BER", "HAM"])
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );

    let decoded_props = vec![
        name_shared.clone(),
        population.clone(),
        label_shared.clone(),
    ];
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(IntEncoder::plain());
    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(
        &decoded_props,
        vec![
            SharedDictEncoder {
                dict_encoder: str_enc,
                items: vec![
                    SharedDictItemEncoder {
                        presence: PresenceStream::Present,
                        offsets: enc,
                    },
                    SharedDictItemEncoder {
                        presence: PresenceStream::Present,
                        offsets: enc,
                    },
                ],
            }
            .into(),
            ScalarEncoder::int(PresenceStream::Present, enc).into(),
            SharedDictEncoder {
                dict_encoder: str_enc,
                items: vec![
                    SharedDictItemEncoder {
                        presence: PresenceStream::Present,
                        offsets: enc,
                    },
                    SharedDictItemEncoder {
                        presence: PresenceStream::Present,
                        offsets: enc,
                    },
                ],
            }
            .into(),
        ],
    )
    .unwrap();

    // Output order: struct "name:", scalar "population", struct "label:"
    assert_eq!(encoded.len(), 3);
    let name = decode_struct(&encoded[0]);
    assert_eq!(name.name(), "name:");
    let DecodedProperty::SharedDict(_, name_shared_dict, name_items) = &name else {
        panic!("Expected SharedDict");
    };
    assert_eq!(name_items[0].suffix, "de");
    assert_eq!(
        name_items[0].materialize(name_shared_dict),
        strs(&["Berlin", "Hamburg"])
    );
    assert_eq!(name_items[1].suffix, "en");
    assert_eq!(
        name_items[1].materialize(name_shared_dict),
        strs(&["Berlin", "Hamburg"])
    );
    assert_eq!(decode_scalar(&encoded[1]), population);
    let label = decode_struct(&encoded[2]);
    assert_eq!(label.name(), "label:");
    let DecodedProperty::SharedDict(_, label_shared_dict, label_items) = &label else {
        panic!("Expected SharedDict");
    };
    assert_eq!(label_items[0].suffix, "de");
    assert_eq!(
        label_items[0].materialize(label_shared_dict),
        strs(&["BE", "HH"])
    );
    assert_eq!(label_items[1].suffix, "en");
    assert_eq!(
        label_items[1].materialize(label_shared_dict),
        strs(&["BER", "HAM"])
    );
}

#[test]
fn struct_instruction_count_mismatch() {
    let err = Vec::<OwnedEncodedProperty>::from_decoded(&vec![DecodedProperty::default()], vec![])
        .unwrap_err();
    assert!(
        matches!(
            err,
            MltError::EncodingInstructionCountMismatch {
                input_len: 1,
                config_len: 0
            }
        ),
        "unexpected error: {err}"
    );
}

proptest! {
    #[test]
    fn struct_roundtrip(
        struct_name in "[a-z]{1,8}",
        children in prop::collection::vec(
            (
                "[a-z]{1,6}",
                prop::collection::vec(prop::option::of("[a-zA-Z ]{0,20}"), 0..20),
            ),
            1..5usize,
        ),
        encoder in arb_int_encoder_no_fastpfor(),
        string_enc in arb_str_encoder(),
    ) {
        let child_refs: Vec<(&str, Vec<Option<String>>)> = children
            .iter()
            .map(|(name, vals)| (name.as_str(), vals.clone()))
            .collect();
        let result = struct_encode_and_decode(
            &struct_name,
            &child_refs,
            PresenceStream::Present,
            encoder,
            string_enc,
        );
        prop_assert_eq!(result.name(), struct_name.as_str());
        let DecodedProperty::SharedDict(_, shared_dict, items) = result else {
            return Err(TestCaseError::Fail("Expected SharedDict".into()));
        };
        prop_assert_eq!(items.len(), children.len());
        for (item, (child_name, values)) in items.into_iter().zip(children.iter()) {
            prop_assert_eq!(&item.suffix, child_name);
            prop_assert_eq!(item.materialize(&shared_dict), values.clone());
        }
    }
}
