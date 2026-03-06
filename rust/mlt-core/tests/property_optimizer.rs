use insta::assert_debug_snapshot;
use mlt_core::v01::{
    DecodedProperty, MultiPropertyEncoder, OwnedEncodedProperty, PropValue, Property,
    PropertyOptimizer,
};
use mlt_core::{FromDecoded as _, borrowme};

fn str_prop(name: &str, values: &[&str]) -> DecodedProperty {
    DecodedProperty {
        name: name.to_owned(),
        values: PropValue::Str(values.iter().map(|s| Some(s.to_string())).collect()),
    }
}

fn optimize_encode_roundtrip(props: &[DecodedProperty]) -> MultiPropertyEncoder {
    let encoder = PropertyOptimizer::optimize(props);
    let enc = Vec::<OwnedEncodedProperty>::from_decoded(&props.to_vec(), encoder.clone())
        .expect("encoding failed");
    let decoded: Vec<DecodedProperty> = enc
        .iter()
        .map(|enc| {
            Property::from(borrowme::borrow(enc))
                .decode()
                .expect("decode failed")
        })
        .collect();

    // Verify the decoded data is equivalent to the input.
    // When multiple Str columns are encoded as SharedDict, they decode
    // as a single SharedDict property, so we compare values not structure.
    verify_equivalent_data(props, &decoded);
    encoder
}

/// Verify that decoded properties contain equivalent data to the input.
/// Handles the case where multiple Str properties become one `SharedDict`.
fn verify_equivalent_data(input: &[DecodedProperty], decoded: &[DecodedProperty]) {
    // Collect all input values by name
    let mut input_values: std::collections::HashMap<String, Vec<Option<String>>> =
        std::collections::HashMap::new();
    for prop in input {
        if let PropValue::Str(v) = &prop.values {
            input_values.insert(prop.name.clone(), v.clone());
        }
    }

    // Collect all decoded values by name (expanding SharedDict items)
    let mut decoded_values: std::collections::HashMap<String, Vec<Option<String>>> =
        std::collections::HashMap::new();
    for prop in decoded {
        match &prop.values {
            PropValue::Str(v) => {
                decoded_values.insert(prop.name.clone(), v.clone());
            }
            PropValue::SharedDict(items) => {
                for item in items {
                    let full_name = format!("{}{}", prop.name, item.suffix);
                    decoded_values.insert(full_name, item.values.clone());
                }
            }
            _ => {}
        }
    }

    assert_eq!(input_values, decoded_values, "Decoded data mismatch");
}

#[test]
fn two_similar_columns_collapse_to_one_struct() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let props = vec![str_prop("name:en", vocab), str_prop("name:de", vocab)];
    assert_debug_snapshot!(optimize_encode_roundtrip(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "en",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "de",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
        ],
    }
    "#);
}

#[test]
fn three_similar_columns_collapse_to_one_struct() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let props = vec![
        str_prop("name:en", vocab),
        str_prop("name:de", vocab),
        str_prop("name:fr", vocab),
    ];
    assert_debug_snapshot!(optimize_encode_roundtrip(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "en",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "de",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "fr",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
        ],
    }
    "#);
}

#[test]
fn column_ordering_does_not_affect_grouping() {
    // "addr:zip" / "addr:street" / "addr:zipcode" have unequal pairwise prefix
    // lengths. Without min-accumulation in common_prefix_name, a later
    // comparison can produce a longer prefix than an earlier one, causing a
    // name collision between independent groups and dissolving one into scalars.
    let vocab = &["alpha", "beta", "gamma", "delta"];
    let props = vec![
        str_prop("addr:zip", vocab),
        str_prop("addr:street", vocab),
        str_prop("addr:zipcode", vocab),
    ];
    assert_debug_snapshot!(optimize_encode_roundtrip(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "zip",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "street",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    dict_encoder: Plain {
                        string_lengths: IntEncoder {
                            logical: None,
                            physical: VarInt,
                        },
                    },
                    items: [
                        SharedDictItemEncoder {
                            child_name: "zipcode",
                            optional: Absent,
                            offset: IntEncoder {
                                logical: Delta,
                                physical: VarInt,
                            },
                        },
                    ],
                },
            ),
        ],
    }
    "#);
}

#[test]
fn dissimilar_columns_stay_as_separate_scalars() {
    let props = vec![
        str_prop("city:de", &["Munich", "Manheim", "Garching"]),
        str_prop("city:colourado", &["Black", "Red", "Gold"]),
    ];
    assert_debug_snapshot!(optimize_encode_roundtrip(&props), @"
    MultiPropertyEncoder {
        properties: [
            Scalar(
                ScalarEncoder {
                    optional: Absent,
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
                    optional: Absent,
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
        ],
    }
    ");
}

#[test]
fn single_string_column_stays_scalar() {
    let props = vec![str_prop("name", &["Alice", "Bob", "Carol"])];
    assert_debug_snapshot!(optimize_encode_roundtrip(&props), @"
    MultiPropertyEncoder {
        properties: [
            Scalar(
                ScalarEncoder {
                    optional: Absent,
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
        ],
    }
    ");
}

#[test]
fn empty_input_produces_no_encoded_columns() {
    assert_debug_snapshot!(optimize_encode_roundtrip(&[]), @"
    MultiPropertyEncoder {
        properties: [],
    }
    ");
}
