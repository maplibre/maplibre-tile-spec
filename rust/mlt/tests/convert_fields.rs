//! Checks how `mlt convert --fields` parses string properties into typed columns, and what `--verify` accepts.
#![cfg(all(feature = "unstable-v2", not(feature = "hotpath")))]

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use mlt::convert::fields::FieldConfig;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{Geometry, LineString, Point};
use mlt_core::mvt::tile_layers_to_mvt;
use mlt_core::{
    Decoder, MValue, NestedKind, NestedValue, Parser, PropKind, PropValue, TileLayer, ZStep,
};

#[test]
fn comma_separated_u64s_become_a_list() {
    let report = parsed(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        &[(2, "5,6"), (2, "18446744073709551615,0")],
    );
    insta::assert_snapshot!(report, @r#"
    "5,6" -> u64 [5, 6]
    "18446744073709551615,0" -> u64 [18446744073709551615, 0]
    "#);
}

#[test]
fn strings_split_at_a_character_keep_empty_and_padded_items() {
    let report = parsed(
        "[layers.l]\nf = { split = ';', kind = 'str' }",
        &[
            (2, "cafe;wifi"),
            (2, ""),
            (2, "cafe;;wifi"),
            (2, " cafe ; wifi"),
        ],
    );
    insta::assert_snapshot!(report, @r#"
    "cafe;wifi" -> str ["cafe", "wifi"]
    "" -> str [""]
    "cafe;;wifi" -> str ["cafe", "", "wifi"]
    " cafe ; wifi" -> str [" cafe ", " wifi"]
    "#);
}

#[test]
fn a_sign_split_starts_a_value_at_each_sign_after_the_first() {
    let report = parsed(
        "[layers.l]\nf = { split = 'sign', kind = 'i32' }",
        &[(2, "10+1-2+0"), (2, "-5-5"), (2, "-2147483648+2147483647")],
    );
    insta::assert_snapshot!(report, @r#"
    "10+1-2+0" -> i32 [10, 1, -2, 0]
    "-5-5" -> i32 [-5, -5]
    "-2147483648+2147483647" -> i32 [-2147483648, 2147483647]
    "#);
}

#[test]
fn a_running_sum_adds_each_value_to_the_one_before() {
    let report = parsed(
        "[layers.l]\nf = { split = 'sign', kind = 'i64', running-sum = true }",
        &[(2, "10+1-2+0"), (2, "-100+0-1")],
    );
    insta::assert_snapshot!(report, @r#"
    "10+1-2+0" -> i64 [10, 11, 9, 9]
    "-100+0-1" -> i64 [-100, -100, -101]
    "#);
}

#[test]
fn a_running_sum_of_unsigned_values_takes_negative_differences() {
    let report = parsed(
        "[layers.l]\nf = { split = ',', kind = 'u32', running-sum = true }",
        &[(2, "3,1,0,2"), (2, "4,-1")],
    );
    insta::assert_snapshot!(report, @r#"
    "3,1,0,2" -> u32 [3, 4, 4, 6]
    "4,-1" -> u32 [4, 3]
    "#);
}

#[test]
fn a_running_sum_becomes_one_m_value_per_vertex() {
    let report = parsed(
        "[layers.l]\nf = { split = 'sign', kind = 'i32', running-sum = true, into = 'm-value' }",
        &[(4, "10+1-2+0")],
    );
    insta::assert_snapshot!(report, @r#""10+1-2+0" -> m-value i32 [10, 11, 9, 9]"#);
}

#[test]
fn u32s_become_one_m_value_per_vertex() {
    let report = parsed(
        "[layers.l]\nf = { split = ',', kind = 'u32', into = 'm-value' }",
        &[(3, "0,7,4294967295")],
    );
    insta::assert_snapshot!(report, @r#""0,7,4294967295" -> m-value u32 [0, 7, 4294967295]"#);
}

#[test]
fn i64s_become_one_m_value_per_vertex() {
    let report = parsed(
        "[layers.l]\nf = { split = 'sign', kind = 'i64', into = 'm-value' }",
        &[(2, "-9223372036854775808+9223372036854775807")],
    );
    insta::assert_snapshot!(report, @r#""-9223372036854775808+9223372036854775807" -> m-value i64 [-9223372036854775808, 9223372036854775807]"#);
}

#[test]
fn strings_become_one_m_value_per_vertex() {
    let report = parsed(
        "[layers.l]\nf = { split = '|', kind = 'str', into = 'm-value' }",
        &[(3, "gravel|gravel|asphalt"), (3, "gravel||")],
    );
    insta::assert_snapshot!(report, @r#"
    "gravel|gravel|asphalt" -> m-value str ["gravel", "gravel", "asphalt"]
    "gravel||" -> m-value str ["gravel", "", ""]
    "#);
}

#[test]
fn z_and_unparsed_properties_are_kept() {
    let out = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        strings(&[(2, "5,6")]),
    )
    .unwrap();
    let feature = &out.features()[0];
    assert_eq!(out.z_step(), Some(ZStep::new(0).unwrap()));
    assert_eq!(feature.z(), [100, 101]);
    assert_eq!(out.property_names(), ["name"]);
    assert_eq!(
        feature.properties(),
        [PropValue::Str(Some("unparsed".into()))]
    );
}

#[test]
fn a_leading_zero_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        strings(&[(2, "007,1")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: "007,1" formats back as "7,1"
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn a_space_after_the_split_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        strings(&[(2, "5, 6")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: " 6" is not a number: invalid digit found in string
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn a_negative_u64_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        strings(&[(2, "-1")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: "-1" does not fit u64
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn a_plus_on_the_first_value_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i32' }",
        strings(&[(2, "+7")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: "+7" formats back as "7"
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn two_signs_in_a_row_are_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i32' }",
        strings(&[(2, "10++1")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: "+" is not a number: invalid digit found in string
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn a_running_sum_past_the_kind_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i64', running-sum = true }",
        strings(&[(2, "9223372036854775807+1")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: 9223372036854775807 plus +1 overflows
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn a_running_sum_below_zero_for_unsigned_values_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u32', running-sum = true }",
        strings(&[(2, "4,-5")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: 4 plus -5 overflows
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn a_plus_on_a_comma_separated_difference_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u32', running-sum = true }",
        strings(&[(2, "4,+1")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: "4,+1" formats back as "4,1"
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn more_m_values_than_vertices_are_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i32', into = 'm-value' }",
        strings(&[(2, "10+1-2")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: 3 values for 2 vertices
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn fewer_m_values_than_vertices_are_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i32', into = 'm-value' }",
        strings(&[(4, "10+1-2")]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f of feature 1: 3 values for 4 vertices
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn a_configured_field_that_is_already_a_nested_column_is_rejected() {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    let f = layer
        .add_nested("f", NestedKind::list(NestedKind::Leaf(PropKind::U64)))
        .unwrap();
    let mut row = layer.feature(Geometry::from(LineString::from(vec![(0, 0), (1, 0)])));
    row.nested(
        f,
        NestedValue::list([NestedValue::Leaf(PropValue::U64(Some(5)))]),
    )
    .unwrap();
    row.finish().unwrap();

    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        layer.finish(),
    );

    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: f is already a typed column
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn a_running_sum_on_strings_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ';', kind = 'str', running-sum = true }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 5
          |
        2 | f = { split = ';', kind = 'str', running-sum = true }
          |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        a running sum needs a number kind
    ");
}

#[test]
fn a_sign_split_on_strings_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'str' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 5
          |
        2 | f = { split = 'sign', kind = 'str' }
          |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        splitting at signs needs a number kind
    ");
}

#[test]
fn a_minus_as_the_split_for_numbers_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = '-', kind = 'i32' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 5
          |
        2 | f = { split = '-', kind = 'i32' }
          |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        a number kind cannot split at a digit or sign
    ");
}

#[test]
fn a_digit_as_the_split_for_numbers_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = '0', kind = 'u32' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 5
          |
        2 | f = { split = '0', kind = 'u32' }
          |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        a number kind cannot split at a digit or sign
    ");
}

#[test]
fn a_split_of_two_characters_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ', ', kind = 'u64' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @r#"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 15
          |
        2 | f = { split = ', ', kind = 'u64' }
          |               ^^^^
        split is "sign" or one character, not ", "
    "#);
}

#[test]
fn an_unknown_key_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64', delta = true }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 34
          |
        2 | f = { split = ',', kind = 'u64', delta = true }
          |                                  ^^^^^
        unknown field `delta`, expected one of `split`, `kind`, `running-sum`, `into`
    ");
}

#[test]
fn an_unknown_column_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = 'sign', kind = 'i32', into = 'z' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 44
          |
        2 | f = { split = 'sign', kind = 'i32', into = 'z' }
          |                                            ^^^
        unknown variant `z`, expected `list` or `m-value`
    ");
}

#[test]
fn a_misspelled_layers_table_is_rejected() {
    let err = convert("[layer.l]\nf = { split = ',', kind = 'u64' }", strings(&[]));
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 1, column 2
          |
        1 | [layer.l]
          |  ^^^^^
        unknown field `layer`, expected `layers`
    ");
}

#[test]
fn an_unknown_key_in_a_form_table_is_rejected() {
    let err = convert(
        "[layers.l.f]\nsplit = ','\nkind = 'u64'\ndelta = true",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 4, column 1
          |
        4 | delta = true
          | ^^^^^
        unknown field `delta`, expected one of `split`, `kind`, `running-sum`, `into`
    ");
}

#[test]
fn a_snake_case_column_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64', into = 'm_value' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 41
          |
        2 | f = { split = ',', kind = 'u64', into = 'm_value' }
          |                                         ^^^^^^^^^
        unknown variant `m_value`, expected `list` or `m-value`
    ");
}

#[test]
fn a_snake_case_running_sum_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64', running_sum = true }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 34
          |
        2 | f = { split = ',', kind = 'u64', running_sum = true }
          |                                  ^^^^^^^^^^^
        unknown field `running_sum`, expected one of `split`, `kind`, `running-sum`, `into`
    ");
}

#[test]
fn an_upper_case_kind_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'U64' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 27
          |
        2 | f = { split = ',', kind = 'U64' }
          |                           ^^^^^
        unknown variant `U64`, expected one of `i32`, `u32`, `i64`, `u64`, `str`
    ");
}

#[test]
fn layers_and_fields_no_applied_layer_held_are_unused() {
    let fields: FieldConfig = "[layers.l]\nf = { split = ',', kind = 'u64' }\ntypo = { split = ',', kind = 'u64' }\n[layers.other]\nf = { split = ',', kind = 'u64' }"
        .parse()
        .unwrap();
    fields.apply(strings(&[(2, "5,6")])).unwrap();
    assert_eq!(fields.unused(), ["field typo of layer l", "layer other"]);
}

#[test]
fn an_unknown_kind_is_rejected() {
    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'f32' }",
        strings(&[]),
    );
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 27
          |
        2 | f = { split = ',', kind = 'f32' }
          |                           ^^^^^
        unknown variant `f32`, expected one of `i32`, `u32`, `i64`, `u64`, `str`
    ");
}

#[test]
fn a_missing_kind_is_rejected() {
    let err = convert("[layers.l]\nf = { split = ',' }", strings(&[]));
    insta::assert_snapshot!(err.unwrap_err(), @"
    Error: parsing fields.toml

    Caused by:
        TOML parse error at line 2, column 5
          |
        2 | f = { split = ',' }
          |     ^^^^^^^^^^^^^^^
        missing field `kind`
    ");
}

#[test]
fn a_null_field_stays_null_in_a_list() {
    let out = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        strings_or_null(&[(2, Some("5,6")), (2, None)]),
    )
    .unwrap();
    let null = out.features().iter().find(|f| f.id() == Some(2)).unwrap();
    assert_eq!(null.nested(), [NestedValue::List(None)]);
}

#[test]
fn a_null_field_stays_null_in_an_m_value_column() {
    let out = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64', into = 'm-value' }",
        strings_or_null(&[(2, Some("5,6")), (2, None)]),
    )
    .unwrap();
    let null = out.features().iter().find(|f| f.id() == Some(2)).unwrap();
    assert_eq!(null.m_values(), [MValue::U64(None)]);
}

#[test]
fn existing_m_values_and_nested_columns_are_kept() {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    let f = layer.add_property("f", PropKind::Str).unwrap();
    let m = layer.add_m_value("m", PropKind::I32).unwrap();
    let n = layer
        .add_nested(
            "n",
            NestedKind::map([("k", NestedKind::Leaf(PropKind::Str))]),
        )
        .unwrap();
    let mut row = layer.feature(Geometry::from(LineString::from(vec![(0, 0), (1, 0)])));
    row.property(f, PropValue::Str(Some("5,6".into()))).unwrap();
    row.m_value(m, MValue::I32(Some(vec![-1, 1]))).unwrap();
    row.nested(
        n,
        NestedValue::map([("k", NestedValue::Leaf(PropValue::Str(Some("v".into()))))]),
    )
    .unwrap();
    row.finish().unwrap();

    let out = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        layer.finish(),
    )
    .unwrap();

    let feature = &out.features()[0];
    assert_eq!(out.m_value_names(), ["m"]);
    assert_eq!(feature.m_values(), [MValue::I32(Some(vec![-1, 1]))]);
    assert_eq!(out.nested_names(), ["f", "n"]);
    assert_eq!(
        feature.nested(),
        [
            NestedValue::list([
                NestedValue::Leaf(PropValue::U64(Some(5))),
                NestedValue::Leaf(PropValue::U64(Some(6))),
            ]),
            NestedValue::map([("k", NestedValue::Leaf(PropValue::Str(Some("v".into()))))]),
        ]
    );
}

#[test]
fn a_configured_property_that_is_not_a_string_is_rejected() {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    let f = layer.add_property("f", PropKind::U32).unwrap();
    let mut row = layer.feature(Geometry::from(LineString::from(vec![(0, 0), (1, 0)])));
    row.property(f, PropValue::U32(Some(5))).unwrap();
    row.finish().unwrap();

    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        layer.finish(),
    );

    insta::assert_snapshot!(err.unwrap_err(), @"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: f holds u32 values, but only strings are parsed
    Error: 1 file(s) failed to convert
    ");
}

#[test]
fn a_rejected_feature_without_an_id_is_named_by_its_field_alone() {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    let f = layer.add_property("f", PropKind::Str).unwrap();
    let mut row = layer.feature(Geometry::from(LineString::from(vec![(0, 0), (1, 0)])));
    row.property(f, PropValue::Str(Some("007".into()))).unwrap();
    row.finish().unwrap();

    let err = convert(
        "[layers.l]\nf = { split = ',', kind = 'u64' }",
        layer.finish(),
    );

    insta::assert_snapshot!(err.unwrap_err(), @r#"
    error: in/tile.mlt: converting MLT in/tile.mlt: layer l: field f: "007" formats back as "7"
    Error: 1 file(s) failed to convert
    "#);
}

#[test]
fn every_column_kind_survives_verify() {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    layer.set_z_step(ZStep::new(0).unwrap()).unwrap();
    let props = [
        (PropKind::Bool, PropValue::Bool(Some(true))),
        (PropKind::I8, PropValue::I8(Some(-8))),
        (PropKind::U8, PropValue::U8(Some(8))),
        (PropKind::I32, PropValue::I32(Some(-32))),
        (PropKind::U32, PropValue::U32(Some(32))),
        (PropKind::I64, PropValue::I64(Some(-64))),
        (PropKind::U64, PropValue::U64(Some(u64::MAX))),
        (PropKind::F32, PropValue::F32(Some(0.5))),
        (PropKind::F64, PropValue::F64(Some(0.1))),
        (PropKind::Str, PropValue::Str(Some("s".into()))),
    ];
    let prop_keys: Vec<_> = props
        .iter()
        .map(|(kind, _)| layer.add_property(<&str>::from(*kind), *kind).unwrap())
        .collect();
    let m_values = [
        (PropKind::Bool, MValue::Bool(Some(vec![true, false]))),
        (PropKind::I8, MValue::I8(Some(vec![-8, 8]))),
        (PropKind::U8, MValue::U8(Some(vec![0, 8]))),
        (PropKind::I32, MValue::I32(Some(vec![-32, 32]))),
        (PropKind::U32, MValue::U32(Some(vec![0, 32]))),
        (PropKind::I64, MValue::I64(Some(vec![-64, 64]))),
        (PropKind::U64, MValue::U64(Some(vec![0, u64::MAX]))),
        (PropKind::F32, MValue::F32(Some(vec![0.5, -0.5]))),
        (PropKind::F64, MValue::F64(Some(vec![0.1, -0.1]))),
        (
            PropKind::Str,
            MValue::Str(Some(vec!["a".into(), "b".into()])),
        ),
    ];
    let m_keys: Vec<_> = m_values
        .iter()
        .map(|(kind, _)| {
            layer
                .add_m_value(format!("m_{}", <&str>::from(*kind)), *kind)
                .unwrap()
        })
        .collect();
    let list = layer
        .add_nested("list", NestedKind::list(NestedKind::Leaf(PropKind::U64)))
        .unwrap();
    let map = layer
        .add_nested(
            "map",
            NestedKind::map([
                ("a", NestedKind::Leaf(PropKind::Str)),
                ("b", NestedKind::Leaf(PropKind::F64)),
            ]),
        )
        .unwrap();
    for (id, y) in [(1, 4000), (2, 0)] {
        let mut row = layer.feature(Geometry::from(LineString::from(vec![(0, y), (1, y)])));
        row.id(Some(id));
        row.z(vec![-1, 1]).unwrap();
        for (key, (_, value)) in prop_keys.iter().zip(&props) {
            row.property(*key, value.clone()).unwrap();
        }
        for (key, (_, value)) in m_keys.iter().zip(&m_values) {
            row.m_value(*key, value.clone()).unwrap();
        }
        row.nested(
            list,
            NestedValue::list([
                NestedValue::Leaf(PropValue::U64(Some(1))),
                NestedValue::Leaf(PropValue::U64(None)),
                NestedValue::Leaf(PropValue::U64(Some(3))),
            ]),
        )
        .unwrap();
        row.nested(
            map,
            NestedValue::map([
                ("a", NestedValue::Leaf(PropValue::Str(Some("v".into())))),
                ("b", NestedValue::Leaf(PropValue::F64(None))),
            ]),
        )
        .unwrap();
        row.finish().unwrap();
    }

    run(None, layer.finish()).unwrap();
}

#[test]
fn an_empty_mvt_layer_survives_verify() {
    let mvt = tile_layers_to_mvt(vec![TileLayer::new("l", 4096).unwrap()]).unwrap();
    let mlt = run_tile(None, "tile.mvt", &mvt).unwrap();
    assert_eq!(mlt, Vec::<u8>::new());
}

#[test]
fn an_empty_string_list_is_not_restored() {
    let err = restored("[layers.l]\nf = { split = ',', kind = 'str' }", &[]).unwrap_err();
    insta::assert_snapshot!(format!("{err:#}"), @r#"layer l: field f of feature 1: Strings([]) formats as "", which parses to other values"#);
}

#[test]
fn a_string_holding_the_split_is_not_restored() {
    let err = restored("[layers.l]\nf = { split = ',', kind = 'str' }", &["a,b"]).unwrap_err();
    insta::assert_snapshot!(format!("{err:#}"), @r#"layer l: field f of feature 1: Strings(["a,b"]) formats as "a,b", which parses to other values"#);
}

fn restored(config: &str, items: &[&str]) -> anyhow::Result<TileLayer> {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    let f = layer
        .add_nested("f", NestedKind::list(NestedKind::Leaf(PropKind::Str)))
        .unwrap();
    let mut row = layer.feature(Geometry::from(Point::new(1, 2)));
    row.id(Some(1));
    let leaves = items
        .iter()
        .map(|&item| NestedValue::Leaf(PropValue::Str(Some(item.to_owned()))));
    row.nested(f, NestedValue::list(leaves)).unwrap();
    row.finish().unwrap();
    config
        .parse::<FieldConfig>()
        .unwrap()
        .restore(layer.finish())
}

fn convert(config: &str, layer: TileLayer) -> Result<TileLayer, String> {
    run(Some(config), layer)
}

/// `layer` encoded as v2, converted with `--verify` and the `--fields` file `config`, and decoded again.
fn run(config: Option<&str>, layer: TileLayer) -> Result<TileLayer, String> {
    let v2 = EncoderConfig::default().with_wire_version(WireVersion::V02);
    let bytes = run_tile(config, "tile.mlt", &layer.encode(v2).unwrap())?;
    let [layer] = <[_; 1]>::try_from(Parser::default().parse_layers(&bytes).unwrap()).unwrap();
    Ok(layer.into_tile(&mut Decoder::default()).unwrap().unwrap())
}

/// The bytes `mlt convert --verify` writes for the tile `file` holding `bytes`, or its stderr if it fails.
fn run_tile(config: Option<&str>, file: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let name = thread::current()
        .name()
        .expect("libtest names each test's thread")
        .to_owned();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("convert_fields")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("in")).unwrap();
    fs::write(dir.join("in").join(file), bytes).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_mlt"));
    command.arg("convert").arg("--verify");
    if let Some(config) = config {
        fs::write(dir.join("fields.toml"), config).unwrap();
        command.arg("--fields").arg(dir.join("fields.toml"));
    }
    let out = command
        .arg(dir.join("in"))
        .arg(dir.join("out"))
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("mlt convert");
    if !out.status.success() {
        let stderr = String::from_utf8(out.stderr).unwrap();
        return Err(stderr.replace(&format!("{}/", dir.display()), ""));
    }
    Ok(fs::read(dir.join("out/tile.mlt")).unwrap())
}

fn parsed(config: &str, cases: &[(i32, &str)]) -> String {
    let out = convert(config, strings(cases)).unwrap();
    let mut report = String::new();
    for (id, &(_, input)) in (1..).zip(cases) {
        let feature = out.features().iter().find(|f| f.id() == Some(id)).unwrap();
        let column = match (feature.m_values(), feature.nested()) {
            ([m], []) => m_value(m),
            ([], [nested]) => list(nested),
            other => panic!("expected one typed column, got {other:?}"),
        };
        writeln!(report, "{input:?} -> {column}").unwrap();
    }
    report
}

fn strings(cases: &[(i32, &str)]) -> TileLayer {
    let cases: Vec<(i32, Option<&str>)> = cases.iter().map(|&(v, s)| (v, Some(s))).collect();
    strings_or_null(&cases)
}

fn strings_or_null(cases: &[(i32, Option<&str>)]) -> TileLayer {
    let mut layer = TileLayer::builder("l", 4096).unwrap();
    layer.set_z_step(ZStep::new(0).unwrap()).unwrap();
    let f = layer.add_property("f", PropKind::Str).unwrap();
    let name = layer.add_property("name", PropKind::Str).unwrap();
    for (id, &(vertices, input)) in (1_u64..).zip(cases) {
        let y = i32::try_from(id).unwrap();
        let line = LineString::from((0..vertices).map(|x| (x, y)).collect::<Vec<_>>());
        let mut row = layer.feature(Geometry::from(line));
        row.id(Some(id));
        row.z((0..vertices).map(|i| 100 + i).collect()).unwrap();
        row.property(f, PropValue::Str(input.map(str::to_owned)))
            .unwrap();
        row.property(name, PropValue::Str(Some("unparsed".into())))
            .unwrap();
        row.finish().unwrap();
    }
    layer.finish()
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "any other m-value is a failed test"
)]
fn m_value(m: &MValue) -> String {
    let values = match m {
        MValue::I32(Some(v)) => format!("{v:?}"),
        MValue::U32(Some(v)) => format!("{v:?}"),
        MValue::I64(Some(v)) => format!("{v:?}"),
        MValue::U64(Some(v)) => format!("{v:?}"),
        MValue::Str(Some(v)) => format!("{v:?}"),
        other => panic!("unexpected m-value {other:?}"),
    };
    format!("m-value {} {values}", <&str>::from(m.kind()))
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "any other list is a failed test"
)]
fn list(nested: &NestedValue) -> String {
    let NestedValue::List(Some(items)) = nested else {
        panic!("unexpected nested value {nested:?}");
    };
    let leaves: Vec<&PropValue> = items
        .iter()
        .map(|item| match item {
            NestedValue::Leaf(v) => v,
            other => panic!("unexpected list item {other:?}"),
        })
        .collect();
    let values: Vec<String> = leaves
        .iter()
        .map(|v| match v {
            PropValue::I32(Some(v)) => v.to_string(),
            PropValue::U32(Some(v)) => v.to_string(),
            PropValue::I64(Some(v)) => v.to_string(),
            PropValue::U64(Some(v)) => v.to_string(),
            PropValue::Str(Some(v)) => format!("{v:?}"),
            other => panic!("unexpected leaf {other:?}"),
        })
        .collect();
    format!("{} [{}]", <&str>::from(leaves[0].kind()), values.join(", "))
}
