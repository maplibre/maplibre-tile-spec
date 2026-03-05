use std::collections::HashMap;

use mlt_core::borrowme;
use mlt_core::v01::{
    DecodedProperty, IntEncoder, LogicalEncoder, MultiPropertyEncoder, OwnedEncodedProperty,
    PhysicalEncoder, PresenceStream, PropValue, Property, PropertyEncoder, ScalarEncoder,
    StrEncoder,
};
use mlt_core::{FromDecoded as _, FromEncoded as _, MltError};
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

fn roundtrip(decoded: &DecodedProperty, encoder: ScalarEncoder) -> DecodedProperty {
    let enc = OwnedEncodedProperty::from_decoded(decoded, encoder).expect("encoding failed");
    DecodedProperty::from_encoded(borrowme::borrow(&enc)).expect("decoding failed")
}

fn strs(vals: &[&str]) -> Vec<Option<String>> {
    vals.iter().map(|v| Some(v.to_string())).collect()
}

fn opt_strs(vals: &[Option<&str>]) -> Vec<Option<String>> {
    vals.iter().map(|v| v.map(ToString::to_string)).collect()
}

fn str_prop(name: &str, values: Vec<Option<String>>) -> DecodedProperty {
    DecodedProperty {
        name: name.to_string(),
        values: PropValue::Str(values),
    }
}

fn expand_struct(prop: &OwnedEncodedProperty) -> Vec<DecodedProperty> {
    Property::from(borrowme::borrow(prop))
        .decode_expand()
        .expect("decode_expand failed")
        .into_iter()
        .map(|p| p.decode().expect("decode failed"))
        .collect()
}

fn decode_scalar(prop: &OwnedEncodedProperty) -> DecodedProperty {
    DecodedProperty::from_encoded(borrowme::borrow(prop)).expect("decode failed")
}

fn struct_encode_and_expand(
    struct_name: &str,
    children: &[(&str, Vec<Option<String>>)],
    presence: PresenceStream,
    offset_encoder: IntEncoder,
    shared_dicts: impl Into<HashMap<String, StrEncoder>>,
) -> Vec<DecodedProperty> {
    let decoded: Vec<DecodedProperty> = children
        .iter()
        .map(|(child_name, values)| str_prop(child_name, values.clone()))
        .collect();
    let instructions: Vec<PropertyEncoder> = children
        .iter()
        .map(|(child_name, _)| {
            PropertyEncoder::shared_dict(struct_name, *child_name, presence, offset_encoder)
        })
        .collect();
    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(
        &decoded,
        MultiPropertyEncoder::new(instructions, shared_dicts.into()),
    )
    .expect("encoding failed");
    assert_eq!(
        encoded.len(),
        1,
        "struct children must collapse to one column"
    );
    expand_struct(&encoded[0])
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
                let prop = DecodedProperty {
                    name: "x".to_string(),
                    values: PropValue::$variant(values),
                };
                let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
                prop_assert_eq!(roundtrip(&prop, scalar_enc), prop);
            }

            #[test]
            fn $absent(
                values in prop::collection::vec(any::<$ty>(), 0..100),
                enc in $int_encoder,
            ) {
                let opt: Vec<Option<$ty>> = values.into_iter().map(Some).collect();
                let prop = DecodedProperty {
                    name: "x".to_string(),
                    values: PropValue::$variant(opt),
                };
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
    let prop = DecodedProperty {
        name: "active".to_string(),
        values: PropValue::Bool(vec![Some(true), None, Some(false), Some(true), None]),
    };
    assert_eq!(
        roundtrip(&prop, ScalarEncoder::bool(PresenceStream::Present)),
        prop
    );
}

#[test]
fn bool_all_null() {
    let prop = DecodedProperty {
        name: "active".to_string(),
        values: PropValue::Bool(vec![None, None, None]),
    };
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
        let prop = DecodedProperty {
            name: "flag".to_string(),
            values: PropValue::Bool(values),
        };
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
        let prop = DecodedProperty {
            name: "score".to_string(),
            values: PropValue::F32(values),
        };
        prop_assert_eq!(roundtrip(&prop, ScalarEncoder::float(PresenceStream::Present)), prop);
    }

    #[test]
    fn f64_roundtrip(
        values in prop::collection::vec(
            prop::option::of(any::<f64>().prop_filter("no NaN", |f| !f.is_nan())),
            0..100,
        ),
    ) {
        let prop = DecodedProperty {
            name: "score".to_string(),
            values: PropValue::F64(values),
        };
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
    let result = struct_encode_and_expand(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
    );
    assert_eq!(result[0].values, PropValue::Str(de));
    assert_eq!(result[1].values, PropValue::Str(en));
}

#[test]
fn struct_with_nulls() {
    let de = opt_strs(&[Some("Berlin"), Some("München"), None]);
    let en = opt_strs(&[Some("Berlin"), None, Some("London")]);
    let result = struct_encode_and_expand(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
    );
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "name:de");
    assert_eq!(result[0].values, PropValue::Str(de));
    assert_eq!(result[1].name, "name:en");
    assert_eq!(result[1].values, PropValue::Str(en));
}

#[test]
fn struct_no_nulls() {
    let de = strs(&["Berlin", "München", "Hamburg"]);
    let en = strs(&["Berlin", "Munich", "Hamburg"]);
    let result = struct_encode_and_expand(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
    );
    assert_eq!(result[0].values, PropValue::Str(de));
    assert_eq!(result[1].values, PropValue::Str(en));
}

#[test]
fn struct_shared_dict_deduplication() {
    let de = strs(&["Berlin", "Berlin"]);
    let en = strs(&["Berlin", "London"]);
    let children = struct_encode_and_expand(
        "name",
        &[(":de", de.clone()), (":en", en.clone())],
        PresenceStream::Present,
        IntEncoder::plain(),
        [("name".to_string(), StrEncoder::plain(IntEncoder::plain()))],
    );
    assert_eq!(children[0].values, PropValue::Str(de));
    assert_eq!(children[1].values, PropValue::Str(en));
}

#[test]
fn struct_mixed_with_scalars() {
    let enc = IntEncoder::plain();
    let scalar_enc = ScalarEncoder::int(PresenceStream::Present, enc);
    let population = DecodedProperty {
        name: "population".to_string(),
        values: PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
    };
    let name_de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
    let name_en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
    let rank = DecodedProperty {
        name: "rank".to_string(),
        values: PropValue::U32(vec![Some(1), Some(2)]),
    };

    let props = vec![
        population.clone(),
        name_de.clone(),
        name_en.clone(),
        rank.clone(),
    ];
    let prop_encs = vec![
        PropertyEncoder::Scalar(scalar_enc),
        PropertyEncoder::shared_dict("name", ":de", PresenceStream::Present, enc),
        PropertyEncoder::shared_dict("name", ":en", PresenceStream::Present, enc),
        PropertyEncoder::Scalar(scalar_enc),
    ];
    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(
        &props,
        MultiPropertyEncoder::new(
            prop_encs,
            HashMap::from([("name".to_string(), StrEncoder::plain(enc))]),
        ),
    )
    .unwrap();

    // Output order: scalar "population", struct "name", scalar "rank"
    assert_eq!(encoded.len(), 3);
    assert_eq!(decode_scalar(&encoded[0]), population);
    let name = expand_struct(&encoded[1]);
    assert_eq!(name[0].name, "name:de");
    assert_eq!(name[0].values, name_de.values);
    assert_eq!(name[1].name, "name:en");
    assert_eq!(name[1].values, name_en.values);
    assert_eq!(decode_scalar(&encoded[2]), rank);
}

#[test]
fn two_struct_groups_with_scalar_between() {
    let name_de = str_prop(":de", strs(&["Berlin", "Hamburg"]));
    let name_en = str_prop(":en", strs(&["Berlin", "Hamburg"]));
    let population = DecodedProperty {
        name: "population".to_string(),
        values: PropValue::U32(vec![Some(3_748_000), Some(1_787_000)]),
    };
    let label_de = str_prop(":de", strs(&["BE", "HH"]));
    let label_en = str_prop(":en", strs(&["BER", "HAM"]));

    let decoded_props = vec![
        name_de.clone(),
        name_en.clone(),
        population.clone(),
        label_de.clone(),
        label_en.clone(),
    ];
    let enc = IntEncoder::plain();
    let str_enc = StrEncoder::plain(IntEncoder::plain());
    let encoded = Vec::<OwnedEncodedProperty>::from_decoded(
        &decoded_props,
        MultiPropertyEncoder::new(
            vec![
                PropertyEncoder::shared_dict("name:", "de", PresenceStream::Present, enc),
                PropertyEncoder::shared_dict("name:", "en", PresenceStream::Present, enc),
                ScalarEncoder::int(PresenceStream::Present, enc).into(),
                PropertyEncoder::shared_dict("label:", "de", PresenceStream::Present, enc),
                PropertyEncoder::shared_dict("label:", "en", PresenceStream::Present, enc),
            ],
            HashMap::from([
                ("name:".to_string(), str_enc),
                ("label:".to_string(), str_enc),
            ]),
        ),
    )
    .unwrap();

    // Output order: struct "name:", scalar "population", struct "label:"
    assert_eq!(encoded.len(), 3);
    let name = expand_struct(&encoded[0]);
    assert_eq!(name[0].name, "name:de");
    assert_eq!(name[0].values, name_de.values);
    assert_eq!(name[1].name, "name:en");
    assert_eq!(name[1].values, name_en.values);
    assert_eq!(decode_scalar(&encoded[1]), population);
    let label = expand_struct(&encoded[2]);
    assert_eq!(label[0].name, "label:de");
    assert_eq!(label[0].values, label_de.values);
    assert_eq!(label[1].name, "label:en");
    assert_eq!(label[1].values, label_en.values);
}

#[test]
fn struct_instruction_count_mismatch() {
    let err = Vec::<OwnedEncodedProperty>::from_decoded(
        &vec![DecodedProperty::default()],
        MultiPropertyEncoder::new(vec![], HashMap::default()),
    )
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
        let re_children = struct_encode_and_expand(
            &struct_name,
            &child_refs,
            PresenceStream::Present,
            encoder,
            [(struct_name.clone(), string_enc)],
        );
        prop_assert_eq!(re_children.len(), children.len());
        for (re, (child_name, values)) in re_children.into_iter().zip(children.iter()) {
            prop_assert_eq!(re.name, format!("{struct_name}{child_name}"));
            prop_assert_eq!(re.values, PropValue::Str(values.clone()));
        }
    }
}
