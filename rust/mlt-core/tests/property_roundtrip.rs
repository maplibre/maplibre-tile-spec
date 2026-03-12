use mlt_core::optimizer::ManualOptimisation as _;
use mlt_core::v01::{
    DecodedOptScalar, DecodedProperty, DecodedScalar, DecodedStrings, IntEncoder, LogicalEncoder,
    OwnedProperty, PhysicalEncoder, PresenceStream, PropertyEncoder, ScalarEncoder,
    SharedDictEncoder, SharedDictItemEncoder, StrEncoder, build_decoded_shared_dict,
};
use mlt_core::{MltError, borrowme};
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
    let mut props = vec![OwnedProperty::Decoded(owned)];
    props
        .manual_optimisation(vec![PropertyEncoder::Scalar(encoder)])
        .expect("encoding failed");
    let enc = props.pop().unwrap();
    mlt_core::borrowme::ToOwned::to_owned(
        &borrowme::borrow(&enc).decode().expect("decoding failed"),
    )
}

fn strs(vals: &[&str]) -> Vec<Option<String>> {
    vals.iter().map(|v| Some((*v).to_string())).collect()
}

fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
    vals.iter().map(|v| v.map(ToString::to_string)).collect()
}

fn shared_dict_prop(
    name: &str,
    children: Vec<(String, DecodedStrings<'static>)>,
) -> DecodedProperty<'static> {
    let shared_dict =
        build_decoded_shared_dict(name.to_string(), children).expect("build shared dict");
    DecodedProperty::SharedDict(shared_dict)
}

fn decode_struct(prop: &OwnedProperty) -> DecodedProperty<'static> {
    mlt_core::borrowme::ToOwned::to_owned(&borrowme::borrow(prop).decode().expect("decode failed"))
}

fn decode_scalar(prop: &OwnedProperty) -> DecodedProperty<'static> {
    mlt_core::borrowme::ToOwned::to_owned(&borrowme::borrow(prop).decode().expect("decode failed"))
}

fn struct_encode_and_decode(
    struct_name: &str,
    children: &[(&str, Vec<Option<String>>)],
    presence: PresenceStream,
    offset_encoder: IntEncoder,
    dict_encoder: StrEncoder,
) -> DecodedProperty<'static> {
    // Build a single DecodedProperty::SharedDict
    let items = children
        .iter()
        .map(|(suffix, values)| ((*suffix).to_string(), DecodedStrings::from(values.clone())))
        .collect();
    let decoded = shared_dict_prop(struct_name, items);

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

    let mut properties = vec![OwnedProperty::Decoded(decoded)];
    properties
        .manual_optimisation(vec![shared_enc.into()])
        .expect("encoding failed");
    assert_eq!(properties.len(), 1, "should produce one encoded property");
    decode_struct(&properties[0])
}

// Absent mode has no presence stream on the wire, so only all-Some inputs are
// valid for those variants.
macro_rules! integer_roundtrip_proptests {
    ($present:ident, $absent:ident, $variant:ident, $variant_opt:ident, $ty:ty, $int_encoder:expr) => {
        proptest! {
            #[test]
            fn $present(
                values in prop::collection::vec(prop::option::of(any::<$ty>()), 0..100),
                enc in $int_encoder,
            ) {
                let prop = DecodedProperty::$variant_opt(DecodedOptScalar::new("x".to_string(), values));
                let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
                prop_assert_eq!(roundtrip(&prop, scalar_enc), prop);
            }

            #[test]
            fn $absent(
                values in prop::collection::vec(any::<$ty>(), 0..100),
                enc in $int_encoder,
            ) {
                let prop = DecodedProperty::$variant(DecodedScalar::new("x".to_string(), values));
                let scalar_enc = ScalarEncoder::int(PresenceStream::Absent, enc);
                prop_assert_eq!(roundtrip(&prop, scalar_enc), prop);
            }
        }
    };
}

// i8, u8, i32, u32 — all physical encoders are valid.
integer_roundtrip_proptests!(i8_present, i8_absent, I8, I8Opt, i8, arb_int_encoder());
integer_roundtrip_proptests!(u8_present, u8_absent, U8, U8Opt, u8, arb_int_encoder());
integer_roundtrip_proptests!(i32_present, i32_absent, I32, I32Opt, i32, arb_int_encoder());
integer_roundtrip_proptests!(u32_present, u32_absent, U32, U32Opt, u32, arb_int_encoder());
// FastPFOR does not support 64-bit integers.
integer_roundtrip_proptests!(
    i64_present,
    i64_absent,
    I64,
    I64Opt,
    i64,
    arb_int_encoder_no_fastpfor()
);
integer_roundtrip_proptests!(
    u64_present,
    u64_absent,
    U64,
    U64Opt,
    u64,
    arb_int_encoder_no_fastpfor()
);

#[test]
fn bool_specific_values() {
    let prop = DecodedProperty::bool_opt(
        "active",
        vec![Some(true), None, Some(false), Some(true), None],
    );
    assert_eq!(
        roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
        prop
    );
}

#[test]
fn bool_all_null() {
    let prop = DecodedProperty::bool_opt("active", vec![None, None, None]);
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
        let prop = DecodedProperty::bool_opt("flag", values);
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
        let prop = DecodedProperty::f32_opt("score", values);
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::float(PresenceStream::Present)), prop);
    }

    #[test]
    fn f64_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f64>().prop_filter("no NaN", |f| !f.is_nan())),
            0..100,
        ),
    ) {
        let prop = DecodedProperty::f64_opt("score", values);
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::float(PresenceStream::Present)), prop);
    }
}

#[test]
fn str_scalar_with_nulls() {
    let prop = DecodedProperty::str(
        "city",
        opt_strs(&[Some("Berlin"), None, Some("Hamburg"), None]),
    );
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    assert_eq!(roundtrip(&prop, enc), prop);
}

#[test]
fn str_scalar_all_null() {
    let prop = DecodedProperty::str("city", opt_strs(&[None, None, None]));
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    assert_eq!(roundtrip(&prop, enc), prop);
}

#[test]
fn str_scalar_empty() {
    let prop = DecodedProperty::str("unused", vec![]);
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
        let prop = DecodedProperty::str("name", values);
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
    let prop = DecodedProperty::str(
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
    let DecodedProperty::SharedDict(shared_dict) = &result else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;
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
    let DecodedProperty::SharedDict(shared_dict) = &result else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;
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
    let DecodedProperty::SharedDict(shared_dict) = &prop else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;

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
    let DecodedProperty::SharedDict(shared_dict) = &result else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;
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
    let DecodedProperty::SharedDict(shared_dict) = &result else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;
    assert_eq!(items[0].materialize(shared_dict), de);
    assert_eq!(items[1].materialize(shared_dict), en);
}

#[test]
fn struct_mixed_with_scalars() {
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(enc);
    let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
    let population = DecodedProperty::u32_opt("population", vec![Some(3_748_000), Some(1_787_000)]);
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
    let rank = DecodedProperty::u32_opt("rank", vec![Some(1), Some(2)]);

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
    let mut encoded: Vec<OwnedProperty> = props.into_iter().map(OwnedProperty::Decoded).collect();
    encoded.manual_optimisation(prop_encs).unwrap();

    // Output order: scalar "population", struct "name:", scalar "rank"
    assert_eq!(encoded.len(), 3);
    assert_eq!(decode_scalar(&encoded[0]), population);
    let name = decode_struct(&encoded[1]);
    assert_eq!(name.name(), "name:");
    let DecodedProperty::SharedDict(shared_dict) = &name else {
        panic!("Expected SharedDict");
    };
    let items = &shared_dict.items;
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
    let population = DecodedProperty::u32_opt("population", vec![Some(3_748_000), Some(1_787_000)]);
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
    let mut encoded: Vec<OwnedProperty> = decoded_props
        .into_iter()
        .map(OwnedProperty::Decoded)
        .collect();
    encoded
        .manual_optimisation(vec![
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
        ])
        .unwrap();

    // Output order: struct "name:", scalar "population", struct "label:"
    assert_eq!(encoded.len(), 3);
    let name = decode_struct(&encoded[0]);
    assert_eq!(name.name(), "name:");
    let DecodedProperty::SharedDict(name_shared_dict) = &name else {
        panic!("Expected SharedDict");
    };
    let name_items = &name_shared_dict.items;
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
    let DecodedProperty::SharedDict(label_shared_dict) = &label else {
        panic!("Expected SharedDict");
    };
    let label_items = &label_shared_dict.items;
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
    let mut properties = vec![OwnedProperty::Decoded(DecodedProperty::default())];
    let err = properties.manual_optimisation(vec![]).unwrap_err();
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
        let DecodedProperty::SharedDict(shared_dict) = result else {
            return Err(TestCaseError::Fail("Expected SharedDict".into()));
        };
        let items = &shared_dict.items;
        prop_assert_eq!(items.len(), children.len());
        for (item, (child_name, values)) in items.iter().zip(children.iter()) {
            prop_assert_eq!(&item.suffix, child_name);
            prop_assert_eq!(item.materialize(&shared_dict), values.clone());
        }
    }
}
