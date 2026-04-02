use insta::assert_debug_snapshot;
use mlt_core::encoder::{
    EncodeProperties as _, StagedLayer01, StagedProperty, group_string_properties,
};
use mlt_core::v01::{PropValue, TileFeature, TileLayer01};

fn str_prop(name: &str, values: &[&str]) -> StagedProperty {
    let owned: Vec<Option<String>> = values.iter().map(|s| Some((*s).to_string())).collect();
    StagedProperty::str(name, owned)
}

fn make_prop(prop: StagedProperty) -> StagedProperty {
    prop
}

/// Build a [`TileLayer01`] from heterogeneous column data (one `Vec<PropValue>` per column).
fn tile_from_cols(cols: &[(&str, Vec<PropValue>)]) -> TileLayer01 {
    let n = cols.first().map_or(0, |(_, v)| v.len());
    let property_names = cols.iter().map(|(name, _)| (*name).to_string()).collect();
    let geom = geo_types::Geometry::<i32>::Point(geo_types::Point::new(0, 0));
    let features = (0..n)
        .map(|i| TileFeature {
            id: None,
            geometry: geom.clone(),
            properties: cols.iter().map(|(_, vals)| vals[i].clone()).collect(),
        })
        .collect();
    TileLayer01 {
        name: "test".to_string(),
        extent: 4096,
        property_names,
        features,
    }
}

/// Convert a `&[&str]` slice into a column of `PropValue::Str` values.
fn str_vals(values: &[&str]) -> Vec<PropValue> {
    values
        .iter()
        .map(|s| PropValue::Str(Some((*s).to_string())))
        .collect()
}

/// Stage a [`TileLayer01`] with `MinHash` grouping and return its properties.
fn stage_props(tile: TileLayer01) -> Vec<StagedProperty> {
    let groups = group_string_properties(&tile);
    StagedLayer01::from_tile(tile, &groups).properties
}

#[test]
fn no_nulls_produces_absent_presence() {
    let props = vec![make_prop(StagedProperty::u32(
        "pop",
        vec![Some(1), Some(2), Some(3)],
    ))];
    let (_, enc) = props.encode_auto().unwrap();
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
    let props = vec![make_prop(StagedProperty::i32("x", vec![None, None, None]))];
    let (_, enc) = props.encode_auto().unwrap();
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
    let props = vec![make_prop(StagedProperty::u32(
        "id",
        (0u32..1_000).map(Some).collect(),
    ))];
    let (_, enc) = props.encode_auto().unwrap();
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
    let props = vec![make_prop(StagedProperty::u32("val", vec![Some(42); 500]))];
    let (_, enc) = props.encode_auto().unwrap();
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
    let tile = tile_from_cols(&[("name:en", str_vals(vocab)), ("name:de", str_vals(vocab))]);
    let (encoded, enc) = stage_props(tile).encode_auto().unwrap();

    assert_eq!(
        encoded.len(),
        1,
        "two similar string columns should be merged into one SharedDict"
    );
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
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
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
    let tile = tile_from_cols(&[
        ("addr:zip", str_vals(vocab)),
        ("addr:street", str_vals(vocab)),
        ("addr:zipcode", str_vals(vocab)),
    ]);
    let (encoded, enc) = stage_props(tile).encode_auto().unwrap();

    assert_eq!(
        encoded.len(),
        1,
        "three similar string columns should be merged"
    );
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
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
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
    let props = vec![
        str_prop("city:de", &["Munich", "Manheim", "Garching"]),
        str_prop("city:colourado", &["Black", "Red", "Gold"]),
    ];
    let (encoded, enc) = props.encode_auto().unwrap();

    assert_eq!(encoded.len(), 2, "dissimilar strings should not be merged");
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
    let tile = tile_from_cols(&[
        ("id", (1u32..=3).map(|v| PropValue::U32(Some(v))).collect()),
        ("name:en", str_vals(vocab)),
        ("name:de", str_vals(vocab)),
        (
            "count",
            [10i32, 20, 30]
                .iter()
                .map(|&v| PropValue::I32(Some(v)))
                .collect(),
        ),
    ]);
    let (encoded, enc) = stage_props(tile).encode_auto().unwrap();

    assert_eq!(encoded.len(), 3, "two scalar + one merged dict");
    assert_debug_snapshot!(enc, @"
    [
        Scalar(
            ScalarEncoder {
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
                        offsets: IntEncoder {
                            logical: Delta,
                            physical: VarInt,
                        },
                    },
                    SharedDictItemEncoder {
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
fn manual_encode_applies_given_encoder() {
    let ref_props = vec![make_prop(StagedProperty::u32(
        "id",
        (0u32..1_000).map(Some).collect(),
    ))];
    let (_, enc) = ref_props.encode_auto().unwrap();

    let props = vec![make_prop(StagedProperty::u32(
        "id",
        (1_000u32..2_000).map(Some).collect(),
    ))];
    let encoded = props.encode(enc).unwrap();
    assert_eq!(encoded.len(), 1);
}

#[test]
fn manual_encode_rejects_mismatched_encoder_count() {
    let ref_props = vec![make_prop(StagedProperty::u32("a", vec![Some(1), Some(2)]))];
    let (_, enc) = ref_props.encode_auto().unwrap();

    let props = vec![
        make_prop(StagedProperty::u32("a", vec![Some(1), Some(2)])),
        make_prop(StagedProperty::u32("b", vec![Some(3), Some(4)])),
    ];
    assert!(props.encode(enc).is_err());
}
