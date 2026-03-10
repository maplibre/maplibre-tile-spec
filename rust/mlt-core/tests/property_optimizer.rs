use insta::assert_debug_snapshot;
use mlt_core::optimizer::{AutomaticOptimisation as _, ManualOptimisation as _};
use mlt_core::v01::{DecodedProperty, OwnedProperty};

fn str_prop(name: &str, values: &[&str]) -> OwnedProperty {
    let owned: Vec<Option<String>> = values.iter().map(|s| Some((*s).to_string())).collect();
    OwnedProperty::Decoded(DecodedProperty::str(name.to_string(), owned))
}

fn make_prop(prop: DecodedProperty<'static>) -> OwnedProperty {
    OwnedProperty::Decoded(prop)
}

#[test]
fn no_nulls_produces_absent_presence() {
    let mut props = vec![make_prop(DecodedProperty::u32(
        "pop",
        vec![Some(1), Some(2), Some(3)],
    ))];
    assert_debug_snapshot!(props.automatic_encoding_optimisation().unwrap(), @"
    [
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn all_nulls_produces_present_presence() {
    let mut props = vec![make_prop(DecodedProperty::i32("x", vec![None, None, None]))];
    assert_debug_snapshot!(props.automatic_encoding_optimisation().unwrap(), @"
    [
        Scalar(
            ScalarEncoder {
                presence: Present,
                value: Int(
                    IntEncoder {
                        logical: None,
                        physical: None,
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn sequential_u32_picks_delta() {
    let mut props = vec![make_prop(DecodedProperty::u32(
        "id",
        (0u32..1_000).map(Some).collect(),
    ))];
    assert_debug_snapshot!(props.automatic_encoding_optimisation().unwrap(), @"
    [
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: Delta,
                        physical: FastPFOR,
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn constant_u32_picks_rle() {
    let mut props = vec![make_prop(DecodedProperty::u32("val", vec![Some(42); 500]))];
    assert_debug_snapshot!(props.automatic_encoding_optimisation().unwrap(), @"
    [
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: Rle,
                        physical: VarInt,
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn similar_strings_grouped_into_shared_dict() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let mut props = vec![str_prop("name:en", vocab), str_prop("name:de", vocab)];
    let enc = props.automatic_encoding_optimisation().unwrap();

    assert_eq!(props.len(), 1);
    assert_debug_snapshot!(enc, @"
    [
        SharedDict(
            SharedDictEncoder {
                dict_encoder: Plain {
                    string_lengths: IntEncoder {
                        logical: None,
                        physical: VarInt,
                    },
                },
                items: [
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                ],
            },
        ),
    ]
    ");
}

#[test]
fn multiple_similar_string_columns_grouped() {
    let vocab = &["alpha", "beta", "gamma", "delta"];
    let mut props = vec![
        str_prop("addr:zip", vocab),
        str_prop("addr:street", vocab),
        str_prop("addr:zipcode", vocab),
    ];
    let enc = props.automatic_encoding_optimisation().unwrap();

    assert_eq!(props.len(), 1);
    assert_debug_snapshot!(enc, @"
    [
        SharedDict(
            SharedDictEncoder {
                dict_encoder: Plain {
                    string_lengths: IntEncoder {
                        logical: None,
                        physical: VarInt,
                    },
                },
                items: [
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                ],
            },
        ),
    ]
    ");
}

#[test]
fn dissimilar_strings_stay_scalar() {
    let mut props = vec![
        str_prop("city:de", &["Munich", "Manheim", "Garching"]),
        str_prop("city:colourado", &["Black", "Red", "Gold"]),
    ];
    let enc = props.automatic_encoding_optimisation().unwrap();

    assert_eq!(props.len(), 2);
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: String(
                    Plain {
                        string_lengths: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                ),
            },
        ),
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: String(
                    Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn mixed_scalars_and_grouped_strings() {
    let vocab = &["alpha", "beta", "gamma"];
    let mut props = vec![
        make_prop(DecodedProperty::u32("id", vec![Some(1), Some(2), Some(3)])),
        str_prop("name:en", vocab),
        str_prop("name:de", vocab),
        make_prop(DecodedProperty::i32(
            "count",
            vec![Some(10), Some(20), Some(30)],
        )),
    ];
    let enc = props.automatic_encoding_optimisation().unwrap();

    assert_eq!(props.len(), 3);
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                ),
            },
        ),
        SharedDict(
            SharedDictEncoder {
                dict_encoder: Plain {
                    string_lengths: IntEncoder {
                        logical: None,
                        physical: VarInt,
                    },
                },
                items: [
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
                        presence: Absent,
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                ],
            },
        ),
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                ),
            },
        ),
    ]
    ");
}

#[test]
fn manual_optimisation_reuses_derived_encoder() {
    let mut ref_props = vec![make_prop(DecodedProperty::u32(
        "id",
        (0u32..1_000).map(Some).collect(),
    ))];
    let enc = ref_props.automatic_encoding_optimisation().unwrap();

    let mut props = vec![make_prop(DecodedProperty::u32(
        "id",
        (1_000u32..2_000).map(Some).collect(),
    ))];
    props.manual_optimisation(enc).unwrap();

    assert_eq!(props.len(), 1);
}

#[test]
fn manual_optimisation_rejects_mismatched_encoder_count() {
    let mut ref_props = vec![make_prop(DecodedProperty::u32("a", vec![Some(1), Some(2)]))];
    let enc = ref_props.automatic_encoding_optimisation().unwrap();

    let mut props = vec![
        make_prop(DecodedProperty::u32("a", vec![Some(1), Some(2)])),
        make_prop(DecodedProperty::u32("b", vec![Some(3), Some(4)])),
    ];
    assert!(props.manual_optimisation(enc).is_err());
}
