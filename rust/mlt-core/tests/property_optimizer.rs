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

fn optimize_encode_expand(props: &[DecodedProperty]) -> MultiPropertyEncoder {
    let encoder = PropertyOptimizer::optimize(props);
    let enc = Vec::<OwnedEncodedProperty>::from_decoded(&props.to_vec(), encoder.clone())
        .expect("encoding failed");
    let decoded = enc
        .iter()
        .flat_map(|enc| {
            Property::from(borrowme::borrow(enc))
                .decode_expand()
                .expect("decode_expand failed")
                .into_iter()
                .map(|p| p.decode().expect("decode failed"))
        })
        .collect::<Vec<_>>();
    assert_eq!(&decoded, props);
    encoder
}

#[test]
fn two_similar_columns_collapse_to_one_struct() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let props = vec![str_prop("name:en", vocab), str_prop("name:de", vocab)];
    assert_debug_snapshot!(optimize_encode_expand(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    child_name: "en",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    child_name: "de",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
        ],
        shared_dicts: {
            "name:": Plain {
                string_lengths: IntEncoder {
                    logical: None,
                    physical: VarInt,
                },
            },
        },
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
    assert_debug_snapshot!(optimize_encode_expand(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    child_name: "en",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    child_name: "de",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "name:",
                    child_name: "fr",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
        ],
        shared_dicts: {
            "name:": Plain {
                string_lengths: IntEncoder {
                    logical: None,
                    physical: VarInt,
                },
            },
        },
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
    assert_debug_snapshot!(optimize_encode_expand(&props), @r#"
    MultiPropertyEncoder {
        properties: [
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    child_name: "zip",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    child_name: "street",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
            SharedDict(
                SharedDictEncoder {
                    struct_name: "addr:",
                    child_name: "zipcode",
                    offset: IntEncoder {
                        logical: Delta,
                        physical: VarInt,
                    },
                    optional: Absent,
                },
            ),
        ],
        shared_dicts: {
            "addr:": Plain {
                string_lengths: IntEncoder {
                    logical: None,
                    physical: VarInt,
                },
            },
        },
    }
    "#);
}

#[test]
fn dissimilar_columns_stay_as_separate_scalars() {
    let props = vec![
        str_prop("city:de", &["Munich", "Manheim", "Garching"]),
        str_prop("city:colourado", &["Black", "Red", "Gold"]),
    ];
    assert_debug_snapshot!(optimize_encode_expand(&props), @"
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
        shared_dicts: {},
    }
    ");
}

#[test]
fn single_string_column_stays_scalar() {
    let props = vec![str_prop("name", &["Alice", "Bob", "Carol"])];
    assert_debug_snapshot!(optimize_encode_expand(&props), @"
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
        shared_dicts: {},
    }
    ");
}

#[test]
fn empty_input_produces_no_encoded_columns() {
    assert_debug_snapshot!(optimize_encode_expand(&[]), @"
    MultiPropertyEncoder {
        properties: [],
        shared_dicts: {},
    }
    ");
}
