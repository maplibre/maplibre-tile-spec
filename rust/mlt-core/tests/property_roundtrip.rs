use geo_types::Point;
use mlt_core::encoder::{
    EncodeProperties as _, GeometryEncoder, IntEncoder, PhysicalEncoder, PropertyEncoder,
    ScalarEncoder, SharedDictEncoder, SharedDictItemEncoder, StagedLayer01, StagedLayer01Encoder,
    StagedProperty, StagedSharedDict, StrEncoder,
};
use mlt_core::geojson::Geom32;
use mlt_core::test_helpers::{dec, parser};
use mlt_core::{GeometryValues, Layer01, LogicalEncoder, MltError, PropValue, TileLayer01};
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

fn staged_len(staged: &StagedProperty) -> usize {
    match staged {
        StagedProperty::Bool(s) => s.values.len(),
        StagedProperty::I8(s) => s.values.len(),
        StagedProperty::U8(s) => s.values.len(),
        StagedProperty::I32(s) => s.values.len(),
        StagedProperty::U32(s) => s.values.len(),
        StagedProperty::I64(s) => s.values.len(),
        StagedProperty::U64(s) => s.values.len(),
        StagedProperty::F32(s) => s.values.len(),
        StagedProperty::F64(s) => s.values.len(),
        StagedProperty::Str(s) => s.lengths.len(),
        StagedProperty::SharedDict(s) => s.items.first().map_or(0, |i| i.ranges.len()),
    }
}

fn strs(vals: &[&str]) -> Vec<Option<String>> {
    vals.iter().map(|v| Some((*v).to_string())).collect()
}

fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
    vals.iter().map(|v| v.map(ToString::to_string)).collect()
}

fn shared_dict_prop(name: &str, children: Vec<(String, Vec<Option<String>>)>) -> StagedProperty {
    StagedProperty::SharedDict(StagedSharedDict::new(name, children).expect("build shared dict"))
}

/// Build a `(name, values)` pair for use as a [`shared_dict_prop`] column.
fn col(name: &str, values: Vec<Option<String>>) -> (String, Vec<Option<String>>) {
    (name.to_string(), values)
}

/// Shorthand for a non-null [`PropValue::Str`].
fn ps(s: &str) -> PropValue {
    PropValue::Str(Some(s.into()))
}

/// Create a [`GeometryValues`] with `n` degenerate point features at the origin.
fn n_point_geometry(n: usize) -> GeometryValues {
    let mut g = GeometryValues::default();
    for _ in 0..n {
        g.push_geom(&Geom32::Point(Point::new(0, 0)));
    }
    g
}

/// Encode `props` as a layer with matching point geometry and return the raw bytes.
fn encode_to_bytes(props: Vec<StagedProperty>, encoders: Vec<PropertyEncoder>) -> Vec<u8> {
    let n = props.iter().map(staged_len).max().unwrap_or(0);
    let layer = StagedLayer01 {
        name: "test".into(),
        extent: 4096,
        id: None,
        geometry: n_point_geometry(n),
        properties: props,
    };
    let encoded = layer
        .encode(StagedLayer01Encoder {
            geometry: GeometryEncoder::all(IntEncoder::varint()),
            properties: encoders,
            ..Default::default()
        })
        .expect("encoding failed");
    let mut buf = Vec::new();
    encoded.write_to(&mut buf).expect("write failed");
    buf
}

/// Encode and immediately decode `props` into a [`TileLayer01`].
fn encode_and_tile(props: Vec<StagedProperty>, encoders: Vec<PropertyEncoder>) -> TileLayer01 {
    let bytes = encode_to_bytes(props, encoders);
    let layer = Layer01::from_bytes(&bytes, &mut parser()).expect("layer parse failed");
    let mut d = dec();
    let parsed = layer.decode_all(&mut d).expect("decode failed");
    parsed.into_tile(&mut d).expect("into_tile failed")
}

/// Two-item plain-encoded [`SharedDictEncoder`] — the most common test encoder.
fn plain_enc() -> PropertyEncoder {
    two_item_shared_enc(IntEncoder::plain(), StrEncoder::plain(IntEncoder::plain()))
}

// Absent mode has no presence stream on the wire, so only all-Some inputs are
// valid for those variants.
macro_rules! integer_roundtrip_proptests {
    ($present:ident, $absent:ident, $variant:ident, $staged_fn:ident, $ty:ty, $int_encoder:expr) => {
        proptest! {
            #[test]
            fn $present(
                values in prop::collection::vec(prop::option::of(any::<$ty>()), 1..100),
                enc in $int_encoder,
            ) {
                // All-null columns are skipped in encoding; only test when at
                // least one value is present.
                prop_assume!(values.iter().any(Option::is_some));
                let tile = encode_and_tile(
                    vec![StagedProperty::$staged_fn("x", values.clone())],
                    vec![PropertyEncoder::Scalar(ScalarEncoder::int(enc))],
                );
                prop_assert_eq!(&tile.property_names, &["x"]);
                for (i, ov) in values.into_iter().enumerate() {
                    prop_assert_eq!(&tile.features[i].properties[0], &PropValue::$variant(ov));
                }
            }

            #[test]
            fn $absent(
                values in prop::collection::vec(any::<$ty>(), 1..100),
                enc in $int_encoder,
            ) {
                let tile = encode_and_tile(
                    vec![StagedProperty::$staged_fn("x", values.iter().map(|&v| Some(v)).collect())],
                    vec![PropertyEncoder::Scalar(ScalarEncoder::int(enc))],
                );
                prop_assert_eq!(&tile.property_names, &["x"]);
                for (i, &v) in values.iter().enumerate() {
                    prop_assert_eq!(&tile.features[i].properties[0], &PropValue::$variant(Some(v)));
                }
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
    let tile = encode_and_tile(
        vec![StagedProperty::bool("active", values.clone())],
        vec![PropertyEncoder::Scalar(ScalarEncoder::bool())],
    );
    assert_eq!(tile.property_names, vec!["active"]);
    for (i, ov) in values.into_iter().enumerate() {
        assert_eq!(&tile.features[i].properties[0], &PropValue::Bool(ov));
    }
}

#[test]
fn bool_all_null() {
    // All-null columns are skipped in encoding — no column appears on the wire.
    let tile = encode_and_tile(
        vec![StagedProperty::bool(
            "active",
            vec![None::<bool>, None, None],
        )],
        vec![PropertyEncoder::Scalar(ScalarEncoder::bool())],
    );
    assert!(
        tile.property_names.is_empty(),
        "all-null column must be omitted from the wire"
    );
    assert!(tile.features.iter().all(|f| f.properties.is_empty()));
}

proptest! {
    #[test]
    fn bool_roundtrip(
        values in prop::collection::vec(prop::option::of(any::<bool>()), 1..100),
    ) {
        // All-null columns are skipped; only test when at least one value is present.
        prop_assume!(values.iter().any(Option::is_some));
        let tile = encode_and_tile(
            vec![StagedProperty::bool("flag", values.clone())],
            vec![PropertyEncoder::Scalar(ScalarEncoder::bool())],
        );
        prop_assert_eq!(&tile.property_names, &["flag"]);
        for (i, ov) in values.into_iter().enumerate() {
            prop_assert_eq!(&tile.features[i].properties[0], &PropValue::Bool(ov));
        }
    }
}

// NaN is excluded because NaN != NaN.
proptest! {
    #[test]
    fn f32_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f32>().prop_filter("no NaN", |f| !f.is_nan())),
            1..100,
        ),
    ) {
        // All-null columns are skipped; only test when at least one value is present.
        prop_assume!(values.iter().any(Option::is_some));
        let tile = encode_and_tile(
            vec![StagedProperty::f32("score", values.clone())],
            vec![PropertyEncoder::Scalar(ScalarEncoder::float())],
        );
        prop_assert_eq!(&tile.property_names, &["score"]);
        for (i, ov) in values.into_iter().enumerate() {
            prop_assert_eq!(&tile.features[i].properties[0], &PropValue::F32(ov));
        }
    }

    #[test]
    fn f64_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f64>().prop_filter("no NaN", |f| !f.is_nan())),
            1..100,
        ),
    ) {
        // All-null columns are skipped; only test when at least one value is present.
        prop_assume!(values.iter().any(Option::is_some));
        let tile = encode_and_tile(
            vec![StagedProperty::f64("score", values.clone())],
            vec![PropertyEncoder::Scalar(ScalarEncoder::float())],
        );
        prop_assert_eq!(&tile.property_names, &["score"]);
        for (i, ov) in values.into_iter().enumerate() {
            prop_assert_eq!(&tile.features[i].properties[0], &PropValue::F64(ov));
        }
    }
}

fn plain_str_enc() -> PropertyEncoder {
    PropertyEncoder::Scalar(ScalarEncoder::str(IntEncoder::plain()))
}

#[test]
fn str_scalar_with_nulls() {
    let values = opt_strs(&[Some("Berlin"), None, Some("Hamburg"), None]);
    let tile = encode_and_tile(
        vec![StagedProperty::str("city", values.clone())],
        vec![plain_str_enc()],
    );
    assert_eq!(tile.property_names, vec!["city"]);
    for (i, ov) in values.into_iter().enumerate() {
        assert_eq!(&tile.features[i].properties[0], &PropValue::Str(ov));
    }
}

#[test]
fn str_scalar_all_null() {
    // All-null columns are skipped in encoding.
    let tile = encode_and_tile(
        vec![StagedProperty::str("city", opt_strs(&[None, None, None]))],
        vec![plain_str_enc()],
    );
    assert!(
        tile.property_names.is_empty(),
        "all-null string column must be omitted from the wire"
    );
    assert!(tile.features.iter().all(|f| f.properties.is_empty()));
}

#[test]
fn str_scalar_empty() {
    // Empty columns (zero rows) are skipped in encoding.
    let tile = encode_and_tile(
        vec![StagedProperty::str("unused", vec![])],
        vec![plain_str_enc()],
    );
    assert!(
        tile.property_names.is_empty(),
        "empty column must be omitted from the wire"
    );
    assert!(tile.features.is_empty());
}

proptest! {
    #[test]
    fn str_scalar_roundtrip(
        values in prop::collection::vec(prop::option::of("[a-zA-Z0-9 ]{0,30}"), 1..50),
    ) {
        // All-null columns are skipped; only test when at least one value is present.
        prop_assume!(values.iter().any(Option::is_some));
        let tile = encode_and_tile(
            vec![StagedProperty::str("name", values.clone())],
            vec![plain_str_enc()],
        );
        prop_assert_eq!(&tile.property_names, &["name"]);
        for (i, ov) in values.into_iter().enumerate() {
            prop_assert_eq!(&tile.features[i].properties[0], &PropValue::Str(ov));
        }
    }
}

#[test]
fn fsst_scalar_string_roundtrip() {
    let values = strs(&["Berlin", "Brandenburg", "Bremen", "Braunschweig"]);
    let tile = encode_and_tile(
        vec![StagedProperty::str("name", values.clone())],
        vec![PropertyEncoder::Scalar(ScalarEncoder::str_fsst(
            IntEncoder::plain(),
            IntEncoder::plain(),
        ))],
    );
    assert_eq!(tile.property_names, vec!["name"]);
    for (i, ov) in values.into_iter().enumerate() {
        assert_eq!(&tile.features[i].properties[0], &PropValue::Str(ov));
    }
}

fn two_item_shared_enc(enc: IntEncoder, dict_encoder: StrEncoder) -> PropertyEncoder {
    SharedDictEncoder {
        dict_encoder,
        items: vec![
            SharedDictItemEncoder::new(enc),
            SharedDictItemEncoder::new(enc),
        ],
    }
    .into()
}

/// Round-trip a two-column `SharedDict` with plain encoders and check all feature values.
fn check_two_col_dict(
    name: &str,
    s1: &str,
    vals1: Vec<Option<String>>,
    s2: &str,
    vals2: Vec<Option<String>>,
) {
    let tile = encode_and_tile(
        vec![shared_dict_prop(
            name,
            vec![col(s1, vals1.clone()), col(s2, vals2.clone())],
        )],
        vec![plain_enc()],
    );
    assert_eq!(
        tile.property_names,
        vec![format!("{name}{s1}"), format!("{name}{s2}")]
    );
    for (i, (v1, v2)) in vals1.into_iter().zip(vals2).enumerate() {
        assert_eq!(&tile.features[i].properties[0], &PropValue::Str(v1));
        assert_eq!(&tile.features[i].properties[1], &PropValue::Str(v2));
    }
}

#[test]
fn fsst_struct_shared_dict_roundtrip() {
    check_two_col_dict(
        "name",
        ":de",
        strs(&["Berlin", "München", "Köln"]),
        ":en",
        strs(&["Berlin", "Munich", "Cologne"]),
    );
}

#[test]
fn struct_with_nulls() {
    check_two_col_dict(
        "name",
        ":de",
        opt_strs(&[Some("Berlin"), Some("München"), None]),
        ":en",
        opt_strs(&[Some("Berlin"), None, Some("London")]),
    );
}

#[test]
fn struct_shared_dict_inline_ranges_track_nulls_and_empty_strings() {
    // This test validates internal range bookkeeping in StagedSharedDict —
    // not the byte encoding pipeline — so it inspects the staged form directly.
    let de = opt_strs(&[Some(""), None, Some("Berlin")]);
    let en = opt_strs(&[Some(""), Some("Berlin"), Some("")]);
    let prop = shared_dict_prop("name", vec![col(":de", de.clone()), col(":en", en.clone())]);
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
    check_two_col_dict(
        "name",
        ":de",
        strs(&["Berlin", "München", "Hamburg"]),
        ":en",
        strs(&["Berlin", "Munich", "Hamburg"]),
    );
}

#[test]
fn struct_shared_dict_deduplication() {
    check_two_col_dict(
        "name",
        ":de",
        strs(&["Berlin", "Berlin"]),
        ":en",
        strs(&["Berlin", "London"]),
    );
}

#[test]
fn struct_mixed_with_scalars() {
    let enc = IntEncoder::plain();
    let scalar = || PropertyEncoder::Scalar(ScalarEncoder::int(enc));
    let tile = encode_and_tile(
        vec![
            StagedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]),
            shared_dict_prop(
                "name:",
                vec![
                    col("de", strs(&["Berlin", "Hamburg"])),
                    col("en", strs(&["Berlin", "Hamburg"])),
                ],
            ),
            StagedProperty::u32("rank", vec![Some(1), Some(2)]),
        ],
        vec![
            scalar(),
            two_item_shared_enc(enc, StrEncoder::plain(enc)),
            scalar(),
        ],
    );

    assert_eq!(
        tile.property_names,
        vec!["population", "name:de", "name:en", "rank"]
    );
    assert_eq!(tile.features.len(), 2);
    assert_eq!(
        tile.features[0].properties,
        vec![
            PropValue::U32(Some(3_748_000)),
            ps("Berlin"),
            ps("Berlin"),
            PropValue::U32(Some(1))
        ]
    );
    assert_eq!(
        tile.features[1].properties,
        vec![
            PropValue::U32(Some(1_787_000)),
            ps("Hamburg"),
            ps("Hamburg"),
            PropValue::U32(Some(2))
        ]
    );
}

#[test]
fn two_struct_groups_with_scalar_between() {
    let enc = IntEncoder::plain();
    let str_shared = || two_item_shared_enc(enc, StrEncoder::plain(enc));
    let tile = encode_and_tile(
        vec![
            shared_dict_prop(
                "name:",
                vec![
                    col("de", strs(&["Berlin", "Hamburg"])),
                    col("en", strs(&["Berlin", "Hamburg"])),
                ],
            ),
            StagedProperty::u32("population", vec![Some(3_748_000), Some(1_787_000)]),
            shared_dict_prop(
                "label:",
                vec![
                    col("de", strs(&["BE", "HH"])),
                    col("en", strs(&["BER", "HAM"])),
                ],
            ),
        ],
        vec![
            str_shared(),
            PropertyEncoder::Scalar(ScalarEncoder::int(enc)),
            str_shared(),
        ],
    );

    assert_eq!(
        tile.property_names,
        vec!["name:de", "name:en", "population", "label:de", "label:en"]
    );
    assert_eq!(tile.features.len(), 2);
    assert_eq!(
        tile.features[0].properties,
        vec![
            ps("Berlin"),
            ps("Berlin"),
            PropValue::U32(Some(3_748_000)),
            ps("BE"),
            ps("BER")
        ]
    );
    assert_eq!(
        tile.features[1].properties,
        vec![
            ps("Hamburg"),
            ps("Hamburg"),
            PropValue::U32(Some(1_787_000)),
            ps("HH"),
            ps("HAM")
        ]
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

#[test]
fn lazy_layer01_iterate_prop_names_returns_column_names() {
    // Encode a layer with a scalar column and a two-key SharedDict column.
    let bytes = encode_to_bytes(
        vec![
            StagedProperty::u32("pop", vec![Some(1_000), Some(2_000)]),
            shared_dict_prop(
                "addr:",
                vec![
                    col("city", strs(&["Berlin", "Rome"])),
                    col("zip", strs(&["10115", "00100"])),
                ],
            ),
        ],
        vec![
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::varint())),
            plain_enc(),
        ],
    );

    // Parse as a lazy Layer01 — no column data decoded yet.
    let layer = Layer01::from_bytes(&bytes, &mut parser()).expect("parse failed");

    // iterate_prop_names works on the lazy layer before any decoding.
    let names: Vec<String> = layer.iterate_prop_names().map(|n| n.to_string()).collect();
    assert_eq!(names, ["pop", "addr:city", "addr:zip"]);
}

proptest! {
    #[test]
    fn struct_roundtrip(
        struct_name in "[a-z]{1,8}",
        children in prop::collection::vec(
            (
                "[a-z]{1,6}",
                prop::collection::vec(prop::option::of("[a-zA-Z ]{0,20}"), 1..20),
            ),
            1..5usize,
        ),
        encoder in arb_int_encoder_no_fastpfor(),
        string_enc in arb_str_encoder(),
    ) {
        let n = children[0].1.len();
        // SharedDict requires all items to have the same number of features.
        prop_assume!(children.iter().all(|(_, vals)| vals.len() == n));
        // A SharedDict where every child column is all-null is skipped in encoding.
        prop_assume!(children.iter().any(|(_, vals)| vals.iter().any(Option::is_some)));

        let staged = StagedProperty::SharedDict(
            StagedSharedDict::new(&struct_name, children.clone()).expect("build shared dict"),
        );
        let item_encoders: Vec<SharedDictItemEncoder> = children
            .iter()
            .map(|_| SharedDictItemEncoder::new(encoder))
            .collect();
        let tile = encode_and_tile(
            vec![staged],
            vec![SharedDictEncoder { dict_encoder: string_enc, items: item_encoders }.into()],
        );

        let expected_names: Vec<String> = children
            .iter()
            .map(|(suffix, _)| format!("{struct_name}{suffix}"))
            .collect();
        prop_assert_eq!(&tile.property_names, &expected_names);
        prop_assert_eq!(tile.features.len(), n);

        for (feat_idx, feat) in tile.features.iter().enumerate() {
            for (col_idx, (_, values)) in children.iter().enumerate() {
                prop_assert_eq!(
                    &feat.properties[col_idx],
                    &PropValue::Str(values[feat_idx].clone())
                );
            }
        }
    }
}
