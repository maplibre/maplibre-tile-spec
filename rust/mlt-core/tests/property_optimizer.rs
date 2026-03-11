use insta::assert_debug_snapshot;
use mlt_core::optimizer::{
    AutomaticOptimisation as _, ManualOptimisation as _, ProfileOptimisation as _,
};
use mlt_core::v01::{DecodedProperty, OwnedProperty, PropertyProfile};
use rstest::rstest;

fn str_prop(name: &str, values: &[&str]) -> OwnedProperty {
    let owned: Vec<Option<String>> = values.iter().map(|s| Some((*s).to_string())).collect();
    OwnedProperty::Decoded(DecodedProperty::str(name.to_string(), owned))
}

fn make_prop(prop: DecodedProperty<'static>) -> OwnedProperty {
    OwnedProperty::Decoded(prop)
}

/// Like `str_prop` but returns a `DecodedProperty` directly (for `from_sample` calls).
fn decoded_str(name: &str, values: &[&str]) -> DecodedProperty<'static> {
    let owned: Vec<Option<String>> = values.iter().map(|s| Some((*s).to_string())).collect();
    DecodedProperty::str(name.to_string(), owned)
}

fn to_owned_props(decoded: Vec<DecodedProperty<'static>>) -> Vec<OwnedProperty> {
    decoded.into_iter().map(OwnedProperty::Decoded).collect()
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
fn from_sample_similar_strings_groups_them() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let props = vec![decoded_str("name:en", vocab), decoded_str("name:de", vocab)];
    let profile = PropertyProfile::from_sample(&props);
    assert_debug_snapshot!(profile, @"
    PropertyProfile {
        string_groups: [
            [
                \"name:en\",
                \"name:de\",
            ],
        ],
    }
    ");
}

#[test]
fn from_sample_dissimilar_strings_no_groups() {
    let props = vec![
        decoded_str("city:de", &["Munich", "Mannheim", "Garching"]),
        decoded_str("city:us", &["Chicago", "Seattle", "Austin"]),
    ];
    let profile = PropertyProfile::from_sample(&props);
    assert_debug_snapshot!(profile, @"
    PropertyProfile {
        string_groups: [],
    }
    ");
}

#[test]
fn from_sample_single_string_column_no_groups() {
    // Single-element groups are always filtered out.
    let props = vec![decoded_str("name", &["Alice", "Bob"])];
    let profile = PropertyProfile::from_sample(&props);
    assert_debug_snapshot!(profile, @"
    PropertyProfile {
        string_groups: [],
    }
    ");
}

#[test]
fn from_sample_no_string_columns_empty_profile() {
    let props = vec![
        DecodedProperty::u32("pop", vec![Some(1), Some(2), Some(3)]),
        DecodedProperty::i32("delta", vec![Some(-1), Some(0), Some(1)]),
    ];
    let profile = PropertyProfile::from_sample(&props);
    assert_debug_snapshot!(profile, @"
    PropertyProfile {
        string_groups: [],
    }
    ");
}

#[test]
fn from_sample_multiple_similar_groups() {
    let alpha_vocab = &["alpha", "beta", "gamma"];
    let name_vocab = &["Alice", "Bob", "Carol"];
    let props = vec![
        decoded_str("addr:zip", alpha_vocab),
        decoded_str("addr:city", alpha_vocab),
        decoded_str("name:en", name_vocab),
        decoded_str("name:de", name_vocab),
    ];
    let profile = PropertyProfile::from_sample(&props);
    // Two independent groups, order determined by first column index.
    assert_debug_snapshot!(profile, @"
    PropertyProfile {
        string_groups: [
            [
                \"addr:zip\",
                \"addr:city\",
            ],
            [
                \"name:en\",
                \"name:de\",
            ],
        ],
    }
    ");
}

#[test]
fn merge_disjoint_groups_both_kept() {
    let p1 = PropertyProfile::new(vec![vec!["a:en".to_owned(), "a:de".to_owned()]]);
    let p2 = PropertyProfile::new(vec![vec!["b:en".to_owned(), "b:de".to_owned()]]);
    let merged = p1.merge(&p2);
    assert_debug_snapshot!(merged, @"
    PropertyProfile {
        string_groups: [
            [
                \"a:en\",
                \"a:de\",
            ],
            [
                \"b:en\",
                \"b:de\",
            ],
        ],
    }
    ");
}

#[test]
fn merge_overlapping_groups_are_unioned() {
    // p1 groups en+de; p2 groups en+fr – they share "name:en" so they merge.
    let p1 = PropertyProfile::new(vec![vec!["name:en".to_owned(), "name:de".to_owned()]]);
    let p2 = PropertyProfile::new(vec![vec!["name:en".to_owned(), "name:fr".to_owned()]]);
    let merged = p1.merge(&p2);
    assert_debug_snapshot!(merged, @"
    PropertyProfile {
        string_groups: [
            [
                \"name:en\",
                \"name:de\",
                \"name:fr\",
            ],
        ],
    }
    ");
}

#[test]
fn merge_with_empty_is_identity() {
    let p1 = PropertyProfile::new(vec![vec!["name:en".to_owned(), "name:de".to_owned()]]);
    let empty = PropertyProfile::new(vec![]);
    let merged = p1.clone().merge(&empty);
    assert_eq!(merged, p1);
    let merged_other_way = empty.merge(&p1);
    assert_eq!(merged_other_way, p1);
}

#[test]
fn merge_duplicate_group_not_added_twice() {
    let group = vec!["name:en".to_owned(), "name:de".to_owned()];
    let p1 = PropertyProfile::new(vec![group.clone()]);
    let p2 = PropertyProfile::new(vec![group]);
    let merged = p1.merge(&p2);
    assert_debug_snapshot!(merged, @"
    PropertyProfile {
        string_groups: [
            [
                \"name:en\",
                \"name:de\",
            ],
        ],
    }
    ");
}

#[rstest]
#[case::sequential_u32(vec![
    DecodedProperty::u32("id", (0u32..100).map(Some).collect()),
])]
#[case::constant_u32(vec![
    DecodedProperty::u32("val", vec![Some(42u32); 200]),
])]
#[case::signed_i32(vec![
    DecodedProperty::i32("delta", (-50i32..50).map(Some).collect()),
])]
#[case::multiple_int_columns(vec![
    DecodedProperty::u32("pop", (0u32..50).map(Some).collect()),
    DecodedProperty::i32("rank", (0i32..50).map(Some).collect()),
])]
#[case::similar_strings(vec![
    decoded_str("name:en", &["Alice", "Bob", "Carol", "Dave"]),
    decoded_str("name:de", &["Alice", "Bob", "Carol", "Dave"]),
])]
#[case::dissimilar_strings(vec![
    decoded_str("city:de", &["Munich", "Mannheim", "Garching"]),
    decoded_str("city:us", &["Chicago", "Seattle", "Austin"]),
])]
#[case::mixed_int_and_similar_strings(vec![
    DecodedProperty::u32("id", (0u32..10).map(Some).collect()),
    decoded_str("name:en", &["alpha", "beta", "gamma"]),
    decoded_str("name:de", &["alpha", "beta", "gamma"]),
])]
fn profile_driven_matches_automatic(#[case] decoded: Vec<DecodedProperty<'static>>) {
    let profile = PropertyProfile::from_sample(&decoded);

    let mut auto_props = to_owned_props(decoded.clone());
    let auto_enc = auto_props.automatic_encoding_optimisation().unwrap();

    let mut profile_props = to_owned_props(decoded);
    let profile_enc = profile_props.profile_driven_optimisation(&profile).unwrap();

    assert_eq!(auto_enc, profile_enc);
}

#[test]
fn profile_applied_to_partial_tile_skips_missing_columns() {
    // Profile says "name:en" and "name:de" should be grouped.
    let vocab = &["Alice", "Bob", "Carol"];
    let sample = vec![decoded_str("name:en", vocab), decoded_str("name:de", vocab)];
    let profile = PropertyProfile::from_sample(&sample);

    // Tile only has "name:en" – group resolves to 1 column, so it is skipped.
    let mut props = vec![str_prop("name:en", vocab)];
    let enc = props.profile_driven_optimisation(&profile).unwrap();

    assert_eq!(
        props.len(),
        1,
        "column must not be merged with a missing partner"
    );
    assert_debug_snapshot!(enc, @"
    [
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
fn profile_applies_grouping_to_different_tile_data() {
    // Profile is built from one tile, applied to a different tile with the
    // same column names but different string values.
    let sample_vocab = &["Alice", "Bob", "Carol", "Dave"];
    let sample = vec![
        decoded_str("name:en", sample_vocab),
        decoded_str("name:de", sample_vocab),
    ];
    let profile = PropertyProfile::from_sample(&sample);

    let tile_vocab = &["Eve", "Frank", "Grace", "Heidi"];
    let mut props = vec![
        str_prop("name:en", tile_vocab),
        str_prop("name:de", tile_vocab),
    ];
    let enc = props.profile_driven_optimisation(&profile).unwrap();

    // Grouping from the profile must have been applied.
    assert_eq!(props.len(), 1, "columns must be merged by the profile");
    assert_debug_snapshot!(enc, @"
    [
        SharedDict(
            SharedDictEncoder {
                dict_encoder: Plain {
                    string_lengths: IntEncoder {
                        logical: Delta,
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
fn profile_ignores_unrecognised_columns() {
    // Profile has no groups for columns that weren't in the sample tile.
    // The tile must still encode correctly with automatic per-column selection.
    let profile = PropertyProfile::new(vec![]);

    let mut props = vec![
        str_prop("highway", &["motorway", "trunk", "primary"]),
        make_prop(DecodedProperty::u32(
            "lanes",
            vec![Some(2), Some(4), Some(3)],
        )),
    ];
    let enc = props.profile_driven_optimisation(&profile).unwrap();

    assert_eq!(props.len(), 2);
    assert_debug_snapshot!(enc, @"
    [
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
        Scalar(
            ScalarEncoder {
                presence: Absent,
                value: Int(
                    IntEncoder {
                        logical: None,
                        physical: VarInt,
                    },
                ),
            },
        ),
    ]
    ");
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
