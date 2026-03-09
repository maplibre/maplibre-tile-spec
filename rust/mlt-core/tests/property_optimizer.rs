use insta::assert_debug_snapshot;
use mlt_core::v01::{DecodedProperty, PropValue, PropertyOptimizer};

fn str_prop(name: &str, values: &[&str]) -> DecodedProperty {
    DecodedProperty {
        name: name.to_owned(),
        values: PropValue::Str(values.iter().map(|s| Some((*s).to_string())).collect()),
    }
}

fn make_prop(name: &str, values: PropValue) -> DecodedProperty {
    DecodedProperty {
        name: name.to_owned(),
        values,
    }
}

#[test]
fn similar_strings_grouped_into_shared_dict() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let mut props = vec![str_prop("name:en", vocab), str_prop("name:de", vocab)];
    let enc = PropertyOptimizer::optimize(&mut props);

    // Properties should be transformed into a single SharedDict
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].name, "name:");
    let PropValue::SharedDict(items) = &props[0].values else {
        panic!("expected SharedDict");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].suffix, "en");
    assert_eq!(items[1].suffix, "de");

    // Encoder should be SharedDict
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
    let enc = PropertyOptimizer::optimize(&mut props);

    // All three should be grouped into one SharedDict
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].name, "addr:");
    let PropValue::SharedDict(items) = &props[0].values else {
        panic!("expected SharedDict");
    };
    assert_eq!(items.len(), 3);

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
    let enc = PropertyOptimizer::optimize(&mut props);

    // Should stay as two separate scalar properties (dissimilar values)
    assert_eq!(props.len(), 2);
    assert!(matches!(&props[0].values, PropValue::Str(_)));
    assert!(matches!(&props[1].values, PropValue::Str(_)));

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
fn no_nulls_produces_absent_presence() {
    let mut props = vec![make_prop(
        "pop",
        PropValue::U32(vec![Some(1), Some(2), Some(3)]),
    )];
    assert_debug_snapshot!(PropertyOptimizer::optimize(&mut props), @"
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
    let mut props = vec![make_prop("x", PropValue::I32(vec![None, None, None]))];
    assert_debug_snapshot!(PropertyOptimizer::optimize(&mut props), @"
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
    let data: Vec<Option<u32>> = (0u32..1_000).map(Some).collect();
    let mut props = vec![make_prop("id", PropValue::U32(data))];
    assert_debug_snapshot!(PropertyOptimizer::optimize(&mut props), @"
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
    let data: Vec<Option<u32>> = vec![Some(42); 500];
    let mut props = vec![make_prop("val", PropValue::U32(data))];
    assert_debug_snapshot!(PropertyOptimizer::optimize(&mut props), @"
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
fn mixed_scalars_and_grouped_strings() {
    let vocab = &["alpha", "beta", "gamma"];
    let mut props = vec![
        make_prop("id", PropValue::U32(vec![Some(1), Some(2), Some(3)])),
        str_prop("name:en", vocab),
        str_prop("name:de", vocab),
        make_prop("count", PropValue::I32(vec![Some(10), Some(20), Some(30)])),
    ];
    let enc = PropertyOptimizer::optimize(&mut props);

    // Should have 3 properties: id, name: (SharedDict), count
    assert_eq!(props.len(), 3);
    assert_eq!(props[0].name, "id");
    assert_eq!(props[1].name, "name:");
    assert_eq!(props[2].name, "count");

    assert!(matches!(&props[0].values, PropValue::U32(_)));
    assert!(matches!(&props[1].values, PropValue::SharedDict(_)));
    assert!(matches!(&props[2].values, PropValue::I32(_)));

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
