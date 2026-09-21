//! `GeoJSON` for the v2-only columns: vertex-scoped m-values and nested properties.

use insta::assert_snapshot;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{Coord, Geometry, LineString};
use mlt_core::geojson::FeatureCollection;
use mlt_core::test_helpers::{dec, parser};
use mlt_core::{MValue, NestedKind, NestedValue, PropKind, PropValue, TileLayer};

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_wire_version(WireVersion::V02)
}

fn line(from: i32) -> Geometry<i32> {
    Geometry::LineString(LineString::new(vec![
        Coord { x: from, y: from },
        Coord {
            x: from + 1,
            y: from + 1,
        },
    ]))
}

/// Encode as v2, parse, decode and render the layer as one `GeoJSON` line.
fn as_geojson(layer: TileLayer) -> String {
    let bytes = layer.encode(cfg_v2()).expect("encode");
    let mut parser = parser();
    let layers = parser.parse_layers(&bytes).expect("parse");
    let mut decoder = dec();
    let layers = decoder.decode_all(layers).expect("decode");
    let collection = FeatureCollection::from_layers(layers).expect("to geojson");
    serde_json::to_string(&collection).expect("serialize")
}

fn m_value_columns() -> Vec<(&'static str, MValue)> {
    vec![
        ("bool", MValue::Bool(Some(vec![true, false]))),
        ("i8", MValue::I8(Some(vec![i8::MIN, 2]))),
        ("u8", MValue::U8(Some(vec![0, u8::MAX]))),
        ("i32", MValue::I32(Some(vec![i32::MIN, 2]))),
        ("u32", MValue::U32(Some(vec![0, u32::MAX]))),
        ("i64", MValue::I64(Some(vec![i64::MIN, 2]))),
        ("u64", MValue::U64(Some(vec![0, u64::MAX]))),
        ("f32", MValue::F32(Some(vec![0.5, f32::INFINITY]))),
        ("f64", MValue::F64(Some(vec![-0.25, f64::NAN]))),
        ("str", MValue::Str(Some(vec!["a".into(), "b".into()]))),
    ]
}

#[test]
fn every_m_value_kind_rides_along_as_an_array_property() {
    let mut builder = TileLayer::builder("mvals", 4096).expect("builder");
    let columns = m_value_columns();
    let keys: Vec<_> = columns
        .iter()
        .map(|(name, value)| builder.add_m_value(*name, value.kind()).expect("add"))
        .collect();

    let mut measured = builder.feature(line(0));
    for (key, (_, value)) in keys.iter().zip(&columns) {
        measured.m_value(*key, value.clone()).expect("set");
    }
    measured.finish().expect("push");
    builder.feature(line(10)).finish().expect("push");

    assert_snapshot!(as_geojson(builder.finish()), @r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"_extent":4096,"_layer":"mvals","m:bool":[true,false],"m:f32":[0.5,"f32::INFINITY"],"m:f64":[-0.25,"f64::NAN"],"m:i32":[-2147483648,2],"m:i64":[-9223372036854775808,2],"m:i8":[-128,2],"m:str":["a","b"],"m:u32":[0,4294967295],"m:u64":[0,18446744073709551615],"m:u8":[0,255]},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}},{"type":"Feature","properties":{"_extent":4096,"_layer":"mvals"},"geometry":{"type":"LineString","coordinates":[[10,10],[11,11]]}}]}"#);
}

#[test]
fn nested_columns_serialize_under_their_own_name() {
    let kind = NestedKind::map([
        ("flag", NestedKind::Leaf(PropKind::Bool)),
        ("count", NestedKind::Leaf(PropKind::U64)),
        ("ratio", NestedKind::Leaf(PropKind::F64)),
        ("label", NestedKind::Leaf(PropKind::Str)),
        (
            "inner",
            NestedKind::map([("deep", NestedKind::Leaf(PropKind::I32))]),
        ),
    ]);
    let list_kind = NestedKind::list(NestedKind::Leaf(PropKind::I32));

    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let map_key = builder.add_nested("map", kind).expect("add map");
    let list_key = builder.add_nested("list", list_kind).expect("add list");

    let filled = NestedValue::map([
        ("flag", NestedValue::Leaf(PropValue::Bool(Some(true)))),
        ("count", NestedValue::Leaf(PropValue::U64(Some(u64::MAX)))),
        ("ratio", NestedValue::Leaf(PropValue::F64(Some(0.5)))),
        ("label", NestedValue::Leaf(PropValue::Str(Some("x".into())))),
        (
            "inner",
            NestedValue::map([("deep", NestedValue::Leaf(PropValue::I32(Some(-7))))]),
        ),
    ]);
    let all_null = NestedValue::map([
        ("flag", NestedValue::Leaf(PropValue::Bool(None))),
        ("count", NestedValue::Leaf(PropValue::U64(None))),
        ("ratio", NestedValue::Leaf(PropValue::F64(None))),
        ("label", NestedValue::Leaf(PropValue::Str(None))),
        ("inner", NestedValue::Map(None)),
    ]);

    let mut first = builder.feature(line(0));
    first.nested(map_key, filled).expect("set map");
    first
        .nested(
            list_key,
            NestedValue::list([
                NestedValue::Leaf(PropValue::I32(Some(1))),
                NestedValue::Leaf(PropValue::I32(Some(2))),
            ]),
        )
        .expect("set list");
    first.finish().expect("push");

    let mut second = builder.feature(line(10));
    second.nested(map_key, all_null).expect("set map");
    second
        .nested(list_key, NestedValue::list([]))
        .expect("set list");
    second.finish().expect("push");

    let mut third = builder.feature(line(20));
    third
        .nested(map_key, NestedValue::Map(None))
        .expect("set map");
    third
        .nested(list_key, NestedValue::List(None))
        .expect("set list");
    third.finish().expect("push");

    // Every field of the second feature is null, and a null field is an absent key.
    assert_snapshot!(as_geojson(builder.finish()), @r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"_extent":4096,"_layer":"nested","list":[1,2],"map":{"count":18446744073709551615,"flag":true,"inner":{"deep":-7},"label":"x","ratio":0.5}},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}},{"type":"Feature","properties":{"_extent":4096,"_layer":"nested","list":[],"map":{}},"geometry":{"type":"LineString","coordinates":[[10,10],[11,11]]}},{"type":"Feature","properties":{"_extent":4096,"_layer":"nested","list":null,"map":null},"geometry":{"type":"LineString","coordinates":[[20,20],[21,21]]}}]}"#);
}
