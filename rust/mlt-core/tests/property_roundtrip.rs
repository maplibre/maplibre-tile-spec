use mlt_core::v01::{
    EncodeProperties as _, EncodedProperty, EncodedScalar, EncodedSharedDictEncoding,
    EncodedStringsEncoding, IntEncoder, LogicalEncoder, ParsedProperty, ParsedScalar,
    ParsedStrings, PhysicalEncoder, PresenceStream, PropertyEncoder, RawFsstData, RawPlainData,
    RawPresence, RawProperty, RawScalar, RawSharedDict, RawSharedDictChild, RawSharedDictEncoding,
    RawStrings, RawStringsEncoding, ScalarEncoder, SharedDictEncoder, SharedDictItemEncoder,
    StagedProperty, StrEncoder, build_staged_shared_dict,
};
use mlt_core::{Decode, MltError};
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

fn roundtrip(staged: StagedProperty, expected: &ParsedProperty<'_>, encoder: ScalarEncoder) {
    let mut enc_props = vec![staged]
        .encode(vec![PropertyEncoder::Scalar(encoder)])
        .expect("encoding failed");
    let enc_prop = enc_props.pop().expect("one encoded prop");
    let result = decode_encoded_for_test(&enc_prop).expect("decoding failed");
    assert_eq!(&result, expected);
}

/// Decode an [`EncodedProperty`] by temporarily borrowing its data.
/// FIXME: This function should NOT exist, round-trip should go through public api
///   all the way to bytes and back.
pub fn decode_encoded_for_test(encoded: &EncodedProperty) -> Result<ParsedProperty<'_>, MltError> {
    fn borrow_scalar(s: &EncodedScalar) -> RawScalar<'_> {
        RawScalar {
            name: &s.name.0,
            presence: RawPresence(s.presence.0.as_ref().map(|s| s.as_borrowed())),
            data: s.data.as_borrowed(),
        }
    }

    let borrowed: RawProperty<'_> = match encoded {
        EncodedProperty::Bool(s) => RawProperty::Bool(borrow_scalar(s)),
        EncodedProperty::I8(s) => RawProperty::I8(borrow_scalar(s)),
        EncodedProperty::U8(s) => RawProperty::U8(borrow_scalar(s)),
        EncodedProperty::I32(s) => RawProperty::I32(borrow_scalar(s)),
        EncodedProperty::U32(s) => RawProperty::U32(borrow_scalar(s)),
        EncodedProperty::I64(s) => RawProperty::I64(borrow_scalar(s)),
        EncodedProperty::U64(s) => RawProperty::U64(borrow_scalar(s)),
        EncodedProperty::F32(s) => RawProperty::F32(borrow_scalar(s)),
        EncodedProperty::F64(s) => RawProperty::F64(borrow_scalar(s)),
        EncodedProperty::Str(s) => RawProperty::Str(RawStrings {
            name: &s.name.0,
            presence: RawPresence(s.presence.0.as_ref().map(|p| p.as_borrowed())),
            encoding: match &s.encoding {
                EncodedStringsEncoding::Plain(d) => RawStringsEncoding::Plain(RawPlainData {
                    lengths: d.lengths.as_borrowed(),
                    data: d.data.as_borrowed(),
                }),
                EncodedStringsEncoding::Dictionary {
                    plain_data,
                    offsets,
                } => RawStringsEncoding::Dictionary {
                    plain_data: RawPlainData {
                        lengths: plain_data.lengths.as_borrowed(),
                        data: plain_data.data.as_borrowed(),
                    },
                    offsets: offsets.as_borrowed(),
                },
                EncodedStringsEncoding::FsstPlain(d) => {
                    RawStringsEncoding::FsstPlain(RawFsstData {
                        symbol_lengths: d.symbol_lengths.as_borrowed(),
                        symbol_table: d.symbol_table.as_borrowed(),
                        lengths: d.lengths.as_borrowed(),
                        corpus: d.corpus.as_borrowed(),
                    })
                }
                EncodedStringsEncoding::FsstDictionary { fsst_data, offsets } => {
                    RawStringsEncoding::FsstDictionary {
                        fsst_data: RawFsstData {
                            symbol_lengths: fsst_data.symbol_lengths.as_borrowed(),
                            symbol_table: fsst_data.symbol_table.as_borrowed(),
                            lengths: fsst_data.lengths.as_borrowed(),
                            corpus: fsst_data.corpus.as_borrowed(),
                        },
                        offsets: offsets.as_borrowed(),
                    }
                }
            },
        }),
        EncodedProperty::SharedDict(s) => RawProperty::SharedDict(RawSharedDict {
            name: &s.name.0,
            encoding: match &s.encoding {
                EncodedSharedDictEncoding::Plain(d) => RawSharedDictEncoding::Plain(RawPlainData {
                    lengths: d.lengths.as_borrowed(),
                    data: d.data.as_borrowed(),
                }),
                EncodedSharedDictEncoding::FsstPlain(d) => {
                    RawSharedDictEncoding::FsstPlain(RawFsstData {
                        symbol_lengths: d.symbol_lengths.as_borrowed(),
                        symbol_table: d.symbol_table.as_borrowed(),
                        lengths: d.lengths.as_borrowed(),
                        corpus: d.corpus.as_borrowed(),
                    })
                }
            },
            children: s
                .children
                .iter()
                .map(|c| RawSharedDictChild {
                    name: &c.name.0,
                    presence: RawPresence(c.presence.0.as_ref().map(|s| s.as_borrowed())),
                    data: c.data.as_borrowed(),
                })
                .collect(),
        }),
    };
    <ParsedProperty<'_> as Decode<RawProperty<'_>>>::decode(borrowed)
}

fn strs(vals: &[&str]) -> Vec<Option<String>> {
    vals.iter().map(|v| Some((*v).to_string())).collect()
}

fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
    vals.iter().map(|v| v.map(ToString::to_string)).collect()
}

fn shared_dict_prop(name: &str, children: Vec<(String, ParsedStrings<'static>)>) -> StagedProperty {
    use mlt_core::v01::StagedStrings;
    let staged_children: Vec<(String, StagedStrings)> = children
        .into_iter()
        .map(|(suffix, ps)| (suffix, StagedStrings::from(ps.materialize())))
        .collect();
    let shared_dict =
        build_staged_shared_dict(name.to_string(), staged_children).expect("build shared dict");
    StagedProperty::SharedDict(shared_dict)
}

fn struct_encode_and_decode<F>(
    struct_name: &str,
    children: &[(&str, Vec<Option<String>>)],
    presence: PresenceStream,
    offset_encoder: IntEncoder,
    dict_encoder: StrEncoder,
    check: F,
) where
    F: FnOnce(&ParsedProperty<'_>),
{
    use mlt_core::v01::StagedStrings;
    // Build a single StagedProperty::SharedDict
    let items: Vec<(String, StagedStrings)> = children
        .iter()
        .map(|(suffix, values)| ((*suffix).to_string(), StagedStrings::from(values.clone())))
        .collect();
    let decoded = StagedProperty::SharedDict(
        build_staged_shared_dict(struct_name.to_string(), items).expect("build shared dict"),
    );

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

    let mut encoded = vec![decoded]
        .encode(vec![shared_enc.into()])
        .expect("encoding failed");
    assert_eq!(encoded.len(), 1, "should produce one encoded property");
    let encoded_prop = encoded.pop().unwrap();
    let result = decode_encoded_for_test(&encoded_prop).unwrap();
    check(&result);
}

// Absent mode has no presence stream on the wire, so only all-Some inputs are
// valid for those variants.
macro_rules! integer_roundtrip_proptests {
    ($present:ident, $absent:ident, $variant:ident, $staged_fn:ident, $ty:ty, $int_encoder:expr) => {
        proptest! {
            #[test]
            fn $present(
                values in prop::collection::vec(prop::option::of(any::<$ty>()), 0..100),
                enc in $int_encoder,
            ) {
                let expected = ParsedProperty::$variant(ParsedScalar::new("x", values.clone()));
                let staged = StagedProperty::$staged_fn("x", values);
                let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
                roundtrip(staged, &expected, scalar_enc);
            }

            #[test]
            fn $absent(
                values in prop::collection::vec(any::<$ty>(), 0..100),
                enc in $int_encoder,
            ) {
                let opt: Vec<Option<$ty>> = values.into_iter().map(Some).collect();
                let expected = ParsedProperty::$variant(ParsedScalar::new("x", opt.clone()));
                let staged = StagedProperty::$staged_fn("x", opt);
                let scalar_enc = ScalarEncoder::int(PresenceStream::Absent, enc);
                roundtrip(staged, &expected, scalar_enc);
            }
        }
    };
}

// i8, u8, i32, u32 — all physical encoders are valid.
integer_roundtrip_proptests!(i8_present, i8_absent, I8, i8, i8, arb_int_encoder());
integer_roundtrip_proptests!(u8_present, u8_absent, U8, u8, u8, arb_int_encoder());
integer_roundtrip_proptests!(i32_present, i32_absent, I32, i32, i32, arb_int_encoder());
integer_roundtrip_proptests!(u32_present, u32_absent, U32, u32, u32, arb_int_encoder());
// FastPFOR does not support 64-bit integers.
integer_roundtrip_proptests!(
    i64_present,
    i64_absent,
    I64,
    i64,
    i64,
    arb_int_encoder_no_fastpfor()
);
integer_roundtrip_proptests!(
    u64_present,
    u64_absent,
    U64,
    u64,
    u64,
    arb_int_encoder_no_fastpfor()
);

#[test]
fn bool_specific_values() {
    let values = vec![Some(true), None, Some(false), Some(true), None];
    let expected = ParsedProperty::bool("active", values.clone());
    let staged = StagedProperty::bool("active", values);
    roundtrip(
        staged,
        &expected,
        ScalarEncoder::bool(PresenceStream::Present),
    );
}

#[test]
fn bool_all_null() {
    let values = vec![None, None, None];
    let expected = ParsedProperty::bool("active", values.clone());
    let staged = StagedProperty::bool("active", values);
    roundtrip(
        staged,
        &expected,
        ScalarEncoder::bool(PresenceStream::Present),
    );
}

proptest! {
    #[test]
    fn bool_roundtrip(
        values in prop::collection::vec(prop::option::of(any::<bool>()), 0..100),
    ) {
        let expected = ParsedProperty::bool("flag", values.clone());
        let staged = StagedProperty::bool("flag", values);
        roundtrip(staged, &expected, ScalarEncoder::bool(PresenceStream::Present));
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
        let expected = ParsedProperty::f32("score", values.clone());
        let staged = StagedProperty::f32("score", values);
        roundtrip(staged, &expected, ScalarEncoder::float(PresenceStream::Present));
    }

    #[test]
    fn f64_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f64>().prop_filter("no NaN", |f| !f.is_nan())),
            0..100,
        ),
    ) {
        let expected = ParsedProperty::f64("score", values.clone());
        let staged = StagedProperty::f64("score", values);
        roundtrip(staged, &expected, ScalarEncoder::float(PresenceStream::Present));
    }
}

#[test]
fn str_scalar_with_nulls() {
    let values = opt_strs(&[Some("Berlin"), None, Some("Hamburg"), None]);
    let expected = ParsedProperty::str("city", values.clone());
    let staged = StagedProperty::str("city", values);
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    roundtrip(staged, &expected, enc);
}

#[test]
fn str_scalar_all_null() {
    let values = opt_strs(&[None, None, None]);
    let expected = ParsedProperty::str("city", values.clone());
    let staged = StagedProperty::str("city", values);
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    roundtrip(staged, &expected, enc);
}

#[test]
fn str_scalar_empty() {
    let expected = ParsedProperty::str("unused", vec![]);
    let staged = StagedProperty::str("unused", vec![]);
    let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
    roundtrip(staged, &expected, enc);
}

proptest! {
    #[test]
    fn str_scalar_roundtrip(
        values in prop::collection::vec(
            prop::option::of("[a-zA-Z0-9 ]{0,30}"),
            0..50,
        ),
    ) {
        let expected = ParsedProperty::str("name", values.clone());
        let staged = StagedProperty::str("name", values);
        let enc = ScalarEncoder::str(PresenceStream::Present, IntEncoder::plain());
        roundtrip(staged, &expected, enc);
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
    let values = strs(&["Berlin", "Brandenburg", "Bremen", "Braunschweig"]);
    let expected = ParsedProperty::str("name", values.clone());
    let staged = StagedProperty::str("name", values);
    roundtrip(staged, &expected, enc);
}

#[test]
fn fsst_struct_shared_dict_roundtrip() {
    let de = strs(&["Berlin", "München", "Köln"]);
    let en = strs(&["Berlin", "Munich", "Cologne"]);
    struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
        |result| {
            assert_eq!(result.name(), "name");
            let ParsedProperty::SharedDict(shared_dict) = result else {
                panic!("Expected SharedDict");
            };
            let items = &shared_dict.items;
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].suffix, ":de");
            assert_eq!(items[0].materialize(shared_dict), de);
            assert_eq!(items[1].suffix, ":en");
            assert_eq!(items[1].materialize(shared_dict), en);
        },
    );
}

#[test]
fn struct_with_nulls() {
    let de = opt_strs(&[Some("Berlin"), Some("München"), None]);
    let en = opt_strs(&[Some("Berlin"), None, Some("London")]);
    struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
        |result| {
            assert_eq!(result.name(), "name");
            let ParsedProperty::SharedDict(shared_dict) = result else {
                panic!("Expected SharedDict");
            };
            let items = &shared_dict.items;
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].suffix, ":de");
            assert_eq!(items[0].materialize(shared_dict), de);
            assert_eq!(items[1].suffix, ":en");
            assert_eq!(items[1].materialize(shared_dict), en);
        },
    );
}

#[test]
fn struct_shared_dict_inline_ranges_track_nulls_and_empty_strings() {
    let de = opt_strs(&[Some(""), None, Some("Berlin")]);
    let en = opt_strs(&[Some(""), Some("Berlin"), Some("")]);
    let prop = shared_dict_prop(
        "name",
        vec![
            (
                ":de".to_string(),
                ParsedStrings::from_optional_strings("", de.clone()),
            ),
            (
                ":en".to_string(),
                ParsedStrings::from_optional_strings("", en.clone()),
            ),
        ],
    );
    let StagedProperty::SharedDict(shared_dict) = &prop else {
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
    struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
        |result| {
            assert_eq!(result.name(), "name");
            let ParsedProperty::SharedDict(shared_dict) = result else {
                panic!("Expected SharedDict");
            };
            let items = &shared_dict.items;
            assert_eq!(items[0].materialize(shared_dict), de);
            assert_eq!(items[1].materialize(shared_dict), en);
        },
    );
}

#[test]
fn struct_shared_dict_deduplication() {
    let de = strs(&["Berlin", "Berlin"]);
    let en = strs(&["Berlin", "London"]);
    struct_encode_and_decode(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        StrEncoder::plain(IntEncoder::plain()),
        |result| {
            let ParsedProperty::SharedDict(shared_dict) = result else {
                panic!("Expected SharedDict");
            };
            let items = &shared_dict.items;
            assert_eq!(items[0].materialize(shared_dict), de);
            assert_eq!(items[1].materialize(shared_dict), en);
        },
    );
}

#[test]
fn struct_mixed_with_scalars() {
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(enc);
    let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
    let population = StagedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]);
    let name_shared = shared_dict_prop(
        "name:",
        vec![
            (
                "de".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );
    let rank = StagedProperty::u32("rank", vec![Some(1), Some(2)]);
    let population_parsed =
        ParsedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]);
    let rank_parsed = ParsedProperty::u32("rank", vec![Some(1), Some(2)]);

    let props = vec![population, name_shared, rank];
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
    let encoded = props.encode(prop_encs).unwrap();

    // Output order: scalar "population", struct "name:", scalar "rank"
    assert_eq!(encoded.len(), 3);
    assert_eq!(
        decode_encoded_for_test(&encoded[0]).unwrap(),
        population_parsed
    );
    let name = decode_encoded_for_test(&encoded[1]).unwrap();
    assert_eq!(name.name(), "name:");
    let ParsedProperty::SharedDict(shared_dict) = &name else {
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
    assert_eq!(decode_encoded_for_test(&encoded[2]).unwrap(), rank_parsed);
}

#[test]
fn two_struct_groups_with_scalar_between() {
    let name_shared = shared_dict_prop(
        "name:",
        vec![
            (
                "de".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["Berlin", "Hamburg"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );
    let population = StagedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]);
    let population_parsed =
        ParsedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]);
    let label_shared = shared_dict_prop(
        "label:",
        vec![
            (
                "de".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["BE", "HH"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "en".to_string(),
                ParsedStrings::from_optional_strings(
                    "",
                    strs(&["BER", "HAM"])
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .collect::<Vec<_>>(),
                ),
            ),
        ],
    );

    let decoded_props = vec![name_shared, population, label_shared];
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(IntEncoder::plain());
    let encoded = decoded_props
        .encode(vec![
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
    let name = decode_encoded_for_test(&encoded[0]).unwrap();
    assert_eq!(name.name(), "name:");
    let ParsedProperty::SharedDict(name_shared_dict) = &name else {
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
    assert_eq!(
        decode_encoded_for_test(&encoded[1]).unwrap(),
        population_parsed
    );
    let label = decode_encoded_for_test(&encoded[2]).unwrap();
    assert_eq!(label.name(), "label:");
    let ParsedProperty::SharedDict(label_shared_dict) = &label else {
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
    let err = vec![StagedProperty::bool("", vec![])]
        .encode(vec![])
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
        let mut test_result: Result<(), TestCaseError> = Ok(());
        struct_encode_and_decode(
            &struct_name,
            &child_refs,
            PresenceStream::Present,
            encoder,
            string_enc,
            |result| {
                test_result = (|| {
                    prop_assert_eq!(result.name(), struct_name.as_str());
                    let ParsedProperty::SharedDict(shared_dict) = result else {
                        return Err(TestCaseError::Fail("Expected SharedDict".into()));
                    };
                    let items = &shared_dict.items;
                    prop_assert_eq!(items.len(), children.len());
                    for (item, (child_name, values)) in items.iter().zip(children.iter()) {
                        prop_assert_eq!(&item.suffix, child_name);
                        prop_assert_eq!(item.materialize(shared_dict), values.clone());
                    }
                    Ok(())
                })();
            },
        );
        test_result?;
    }
}
