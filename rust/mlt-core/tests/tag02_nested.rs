use std::collections::BTreeMap;

use mlt_core::dump::{RenderOpts, annotate_tile, render};
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{Coord, Geometry, LineString, Point};
use mlt_core::{
    Decoder, Layer, MltError, NestedKind, NestedValue, Parser, PropKind, PropValue, TileLayer,
};
use rstest::rstest;

fn cfg_v2() -> EncoderConfig {
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_wire_version(WireVersion::V02)
}

fn decode(bytes: &[u8]) -> TileLayer {
    let mut parser = Parser::default();
    let layers = parser.parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1);
    let Layer::Tag02(layer) = layers.into_iter().next().expect("a layer") else {
        panic!("expected a v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).expect("into_tile")
}

fn decode_err(bytes: &[u8]) -> MltError {
    let mut parser = Parser::default();
    match parser.parse_layers(bytes) {
        Err(e) => e,
        Ok(layers) => layers
            .into_iter()
            .next()
            .expect("a layer")
            .into_layer01()
            .expect("layer01")
            .into_tile(&mut Decoder::default())
            .expect_err("expected a rejection"),
    }
}

fn assert_round_trips_as_v2(layer: &TileLayer) -> Vec<u8> {
    let bytes = layer.clone().encode(cfg_v2()).expect("v2 encode");
    assert_eq!(&decode(&bytes), layer);
    assert_dump_covers(&bytes);
    bytes
}

fn assert_dump_covers(bytes: &[u8]) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let mut leaves: Vec<(usize, usize)> = tree
        .regions
        .iter()
        .filter(|r| !r.container)
        .map(|r| (r.offset, r.len))
        .collect();
    leaves.sort_unstable();

    let mut cursor = 0;
    for (offset, len) in &leaves {
        assert_eq!(*offset, cursor, "gap/overlap at offset {offset}");
        cursor += len;
    }
    assert_eq!(cursor, bytes.len());
    render(&tree, bytes, &RenderOpts::default(), &mut Vec::new()).expect("render");
}

fn point(x: i32, y: i32) -> Geometry<i32> {
    Geometry::Point(Point::new(x, y))
}

fn line(pts: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(LineString::new(
        pts.iter().map(|&(x, y)| Coord { x, y }).collect(),
    ))
}

fn leaf(kind: PropKind) -> NestedKind {
    NestedKind::Leaf(kind)
}

fn map_kind(fields: &[(&str, NestedKind)]) -> NestedKind {
    NestedKind::map(fields.iter().map(|(k, v)| (*k, v.clone())))
}

fn entries(values: &[(&str, NestedValue)]) -> NestedValue {
    NestedValue::map(values.iter().map(|(k, v)| (*k, v.clone())))
}

fn str_value(value: &str) -> NestedValue {
    NestedValue::Leaf(PropValue::Str(Some(value.to_string())))
}

fn i32_value(value: i32) -> NestedValue {
    NestedValue::Leaf(PropValue::I32(Some(value)))
}

fn nested_layer(
    kind: NestedKind,
    geometries: &[Geometry<i32>],
    values: &[NestedValue],
) -> TileLayer {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let key = builder.add_nested("n", kind).expect("add_nested");
    for (geometry, value) in geometries.iter().zip(values) {
        let mut feature = builder.feature(geometry.clone());
        feature.nested(key, value.clone()).expect("set_nested");
        feature.finish().expect("push");
    }
    builder.finish()
}

fn two_points() -> Vec<Geometry<i32>> {
    vec![point(0, 0), point(1, 1)]
}

#[test]
fn a_struct_of_every_leaf_type_round_trips() {
    let kind = map_kind(&[
        ("b", leaf(PropKind::Bool)),
        ("i8", leaf(PropKind::I8)),
        ("u8", leaf(PropKind::U8)),
        ("i32", leaf(PropKind::I32)),
        ("u32", leaf(PropKind::U32)),
        ("i64", leaf(PropKind::I64)),
        ("u64", leaf(PropKind::U64)),
        ("f32", leaf(PropKind::F32)),
        ("f64", leaf(PropKind::F64)),
        ("s", leaf(PropKind::Str)),
    ]);
    let value = |n: i32| {
        entries(&[
            ("b", NestedValue::Leaf(PropValue::Bool(Some(n % 2 == 0)))),
            ("i8", NestedValue::Leaf(PropValue::I8(Some(-1)))),
            ("u8", NestedValue::Leaf(PropValue::U8(Some(2)))),
            ("i32", i32_value(n)),
            ("u32", NestedValue::Leaf(PropValue::U32(Some(4)))),
            ("i64", NestedValue::Leaf(PropValue::I64(Some(-5)))),
            ("u64", NestedValue::Leaf(PropValue::U64(Some(6)))),
            ("f32", NestedValue::Leaf(PropValue::F32(Some(7.5)))),
            ("f64", NestedValue::Leaf(PropValue::F64(Some(8.25)))),
            ("s", str_value("ok")),
        ])
    };
    let layer = nested_layer(kind, &two_points(), &[value(0), value(1)]);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_struct_inside_a_struct_round_trips() {
    let kind = map_kind(&[
        ("inner", map_kind(&[("deep", leaf(PropKind::Str))])),
        ("flat", leaf(PropKind::I32)),
    ]);
    let value = |s: &str, n: i32| {
        entries(&[
            ("inner", entries(&[("deep", str_value(s))])),
            ("flat", i32_value(n)),
        ])
    };
    let layer = nested_layer(kind, &two_points(), &[value("a", 1), value("b", 2)]);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_list_of_scalars_round_trips() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::list([i32_value(1), i32_value(2), i32_value(3)]),
        NestedValue::list([i32_value(4)]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_list_of_structs_round_trips() {
    let kind = NestedKind::list(map_kind(&[
        ("name", leaf(PropKind::Str)),
        ("rank", leaf(PropKind::U32)),
    ]));
    let stop = |name: &str, rank: u32| {
        entries(&[
            ("name", str_value(name)),
            ("rank", NestedValue::Leaf(PropValue::U32(Some(rank)))),
        ])
    };
    let values = vec![
        NestedValue::list([stop("a", 1), stop("b", 2)]),
        NestedValue::list([stop("c", 3)]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn an_empty_list_and_a_null_list_stay_apart() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::List(Some(Vec::new())),
        NestedValue::List(None),
        NestedValue::list([i32_value(7)]),
    ];
    let layer = nested_layer(kind, &[point(0, 0), point(1, 1), point(2, 2)], &values);
    assert_round_trips_as_v2(&layer);

    let decoded = decode(&layer.clone().encode(cfg_v2()).expect("encode"));
    assert_eq!(
        decoded.features()[0].nested()[0],
        NestedValue::List(Some(Vec::new()))
    );
    assert_eq!(decoded.features()[1].nested()[0], NestedValue::List(None));
}

#[test]
fn a_map_of_strings_round_trips() {
    let kind = map_kind(&[
        ("amenity", leaf(PropKind::Str)),
        ("name", leaf(PropKind::Str)),
        ("cuisine", leaf(PropKind::Str)),
    ]);
    let values = vec![
        entries(&[("amenity", str_value("cafe")), ("name", str_value("Ada"))]),
        entries(&[
            ("amenity", str_value("bar")),
            ("cuisine", str_value("tapas")),
        ]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_map_inside_a_list_round_trips() {
    let kind = NestedKind::list(map_kind(&[
        ("k", leaf(PropKind::Str)),
        ("v", leaf(PropKind::Str)),
    ]));
    let pair = |k: &str, v: &str| entries(&[("k", str_value(k)), ("v", str_value(v))]);
    let values = vec![
        NestedValue::list([pair("a", "1"), pair("b", "2")]),
        NestedValue::list([pair("c", "3")]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_null_root_round_trips() {
    let kind = map_kind(&[("a", leaf(PropKind::I32))]);
    let values = vec![
        entries(&[("a", i32_value(1))]),
        NestedValue::Map(None),
        entries(&[("a", i32_value(3))]),
    ];
    let layer = nested_layer(kind, &[point(0, 0), point(1, 1), point(2, 2)], &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_null_struct_field_is_an_absent_key() {
    let kind = map_kind(&[("a", leaf(PropKind::I32)), ("b", leaf(PropKind::I32))]);
    let values = vec![
        entries(&[("a", i32_value(1)), ("b", i32_value(2))]),
        entries(&[("a", i32_value(3))]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);

    let decoded = decode(&layer.clone().encode(cfg_v2()).expect("encode"));
    let NestedValue::Map(Some(second)) = &decoded.features()[1].nested()[0] else {
        panic!("a map")
    };
    assert_eq!(second.keys().collect::<Vec<_>>(), vec!["a"]);
}

#[test]
fn a_null_list_element_keeps_its_place() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::list([
            i32_value(1),
            NestedValue::Leaf(PropValue::I32(None)),
            i32_value(3),
        ]),
        NestedValue::list([i32_value(4)]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_nested_root_shares_a_bitfield_with_a_flat_column() {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let flat = builder
        .add_property("flat", PropKind::I32)
        .expect("add_property");
    let key = builder
        .add_nested("n", map_kind(&[("a", leaf(PropKind::I32))]))
        .expect("add_nested");
    for (index, present) in [true, false, true].into_iter().enumerate() {
        let mut feature = builder.feature(point(i32::try_from(index).expect("small"), 0));
        if present {
            feature
                .property(flat, PropValue::I32(Some(1)))
                .expect("property");
            feature
                .nested(key, entries(&[("a", i32_value(2))]))
                .expect("nested");
        }
        feature.finish().expect("push");
    }
    let layer = builder.finish();
    let bytes = assert_round_trips_as_v2(&layer);

    let tree = annotate_tile(&bytes).expect("annotate_tile");
    let shared: Vec<&str> = tree
        .regions
        .iter()
        .map(|r| r.label.as_str())
        .filter(|label| *label == "present[0]" || *label == "present[1]")
        .collect();
    assert_eq!(shared, vec!["present[0]"]);
    let nibbles: Vec<String> = tree
        .regions
        .iter()
        .filter(|r| r.label == "type")
        .filter_map(|r| r.bits.first().map(|b| b.meaning.clone()))
        .collect();
    assert_eq!(
        nibbles,
        vec![
            "presence = Shared(0)".to_string(),
            "presence = Shared(0)".to_string(),
            "node presence = AllPresent".to_string(),
        ]
    );
}

#[test]
fn m_values_and_a_nested_column_live_in_one_layer() {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let m = builder
        .add_m_value("m", PropKind::I32)
        .expect("add_m_value");
    let key = builder
        .add_nested("n", NestedKind::list(leaf(PropKind::Str)))
        .expect("add_nested");
    for i in 0..2 {
        let mut feature = builder.feature(line(&[(0, i), (1, i)]));
        feature
            .m_value(m, mlt_core::MValue::I32(Some(vec![i, i + 1])))
            .expect("m_value");
        feature
            .nested(key, NestedValue::list([str_value("x")]))
            .expect("nested");
        feature.finish().expect("push");
    }
    let layer = builder.finish();
    assert_round_trips_as_v2(&layer);
}

#[test]
fn a_nested_column_becomes_a_json_object() {
    let kind = map_kind(&[
        ("a", leaf(PropKind::I32)),
        ("list", NestedKind::list(leaf(PropKind::Str))),
    ]);
    let values = vec![
        entries(&[
            ("a", i32_value(1)),
            ("list", NestedValue::list([str_value("p"), str_value("q")])),
        ]),
        NestedValue::Map(None),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    let bytes = layer.encode(cfg_v2()).expect("encode");
    let mut parser = Parser::default();
    let layers = parser.parse_layers(&bytes).expect("parse");
    let mut dec = Decoder::default();
    let parsed = dec.decode_all(layers).expect("decode_all");
    let collection =
        mlt_core::geojson::FeatureCollection::from_layers(parsed).expect("feature collection");

    assert_eq!(
        collection.features[0].properties["n"],
        serde_json::json!({"a": 1, "list": ["p", "q"]})
    );
    assert_eq!(
        collection.features[1].properties["n"],
        serde_json::Value::Null
    );
}

#[rstest]
#[case::leaf_root(NestedKind::Leaf(PropKind::I32))]
#[case::leaf_root_str(NestedKind::Leaf(PropKind::Str))]
fn a_scalar_root_is_rejected(#[case] kind: NestedKind) {
    let mut layer = TileLayer::new("nested", 4096).expect("layer");
    assert!(matches!(
        layer.add_nested("n", kind),
        Err(MltError::NestedRootIsLeaf(name)) if name == "n"
    ));
}

#[test]
fn an_empty_struct_is_rejected() {
    let mut layer = TileLayer::new("nested", 4096).expect("layer");
    assert!(matches!(
        layer.add_nested("n", NestedKind::Map(BTreeMap::new())),
        Err(MltError::EmptyStructNode)
    ));
}

#[test]
fn a_nested_name_that_repeats_a_property_is_rejected() {
    let mut layer = TileLayer::new("nested", 4096).expect("layer");
    layer.add_property("n", PropKind::I32).expect("property");
    assert_eq!(
        layer
            .add_nested("n", map_kind(&[("a", leaf(PropKind::I32))]))
            .unwrap_err()
            .to_string(),
        "duplicate column name n: the nested column repeats the property column"
    );
}

#[test]
fn nesting_past_eight_levels_is_rejected() {
    let mut kind = leaf(PropKind::I32);
    for _ in 0..8 {
        kind = NestedKind::list(kind);
    }
    let mut layer = TileLayer::new("nested", 4096).expect("layer");
    assert!(matches!(
        layer.add_nested("n", kind),
        Err(MltError::NestedTooDeep(9))
    ));
}

fn region(bytes: &[u8], label: &str, n: usize) -> (usize, usize) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let found = tree
        .regions
        .iter()
        .filter(|r| r.label == label)
        .nth(n)
        .unwrap_or_else(|| panic!("no region {n} labelled {label}"));
    (found.offset, found.len)
}

fn root_data_type(bytes: &[u8]) -> String {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    tree.regions
        .iter()
        .filter(|r| r.label == "type")
        .find_map(|r| r.bits.get(1).map(|b| b.meaning.clone()))
        .expect("a column type byte")
}

fn payload_of(bytes: &[u8], label: &str, n: usize) -> (usize, usize) {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let start = tree
        .regions
        .iter()
        .filter(|r| r.label == label && r.container)
        .nth(n)
        .expect("a stream region")
        .offset;
    let found = tree
        .regions
        .iter()
        .find(|r| r.label == "data" && !r.container && r.offset >= start)
        .expect("a payload");
    (found.offset, found.len)
}

fn struct_of_strings(keys: &[&str]) -> NestedKind {
    NestedKind::map(keys.iter().map(|k| (*k, leaf(PropKind::Str))))
}

#[test]
fn many_sparse_keys_are_written_as_a_map_node() {
    let keys: Vec<String> = (0..12).map(|i| format!("key{i}")).collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let geometries: Vec<Geometry<i32>> = (0..12).map(|i| point(i, i)).collect();
    let values: Vec<NestedValue> = keys
        .iter()
        .map(|key| NestedValue::map([(key.clone(), str_value("v"))]))
        .collect();
    let layer = nested_layer(struct_of_strings(&refs), &geometries, &values);
    let bytes = assert_round_trips_as_v2(&layer);
    assert_eq!(root_data_type(&bytes), "data type = Map");
}

#[test]
fn a_dense_key_set_is_written_as_a_struct_node() {
    let geometries: Vec<Geometry<i32>> = (0..12).map(|i| point(i, i)).collect();
    let values: Vec<NestedValue> = (0..12)
        .map(|_| entries(&[("alpha", str_value("a")), ("beta", str_value("b"))]))
        .collect();
    let layer = nested_layer(struct_of_strings(&["alpha", "beta"]), &geometries, &values);
    let bytes = assert_round_trips_as_v2(&layer);
    assert_eq!(root_data_type(&bytes), "data type = Struct");
}

#[test]
fn the_smaller_of_the_two_shapes_is_the_one_kept() {
    let keys: Vec<String> = (0..12).map(|i| format!("key{i}")).collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    let geometries: Vec<Geometry<i32>> = (0..12).map(|i| point(i, i)).collect();
    let sparse: Vec<NestedValue> = keys
        .iter()
        .map(|key| NestedValue::map([(key.clone(), str_value("v"))]))
        .collect();
    let dense: Vec<NestedValue> = (0..12)
        .map(|_| entries(&[("alpha", str_value("a")), ("beta", str_value("b"))]))
        .collect();

    let as_map = nested_layer(struct_of_strings(&refs), &geometries, &sparse)
        .encode(cfg_v2())
        .expect("encode");
    let as_struct = nested_layer(struct_of_strings(&["alpha", "beta"]), &geometries, &dense)
        .encode(cfg_v2())
        .expect("encode");
    assert_eq!(root_data_type(&as_map), "data type = Map");
    assert_eq!(root_data_type(&as_struct), "data type = Struct");
}

fn struct_column_bytes() -> Vec<u8> {
    let kind = map_kind(&[("aa", leaf(PropKind::I32)), ("ab", leaf(PropKind::I64))]);
    let layer = nested_layer(
        kind,
        &two_points(),
        &[
            entries(&[
                ("aa", i32_value(1)),
                ("ab", NestedValue::Leaf(PropValue::I64(Some(9)))),
            ]),
            entries(&[
                ("aa", i32_value(2)),
                ("ab", NestedValue::Leaf(PropValue::I64(Some(8)))),
            ]),
        ],
    );
    layer.encode(cfg_v2()).expect("encode")
}

#[rstest]
#[case::reserved_node_presence(0b0010_0101)]
#[case::id_node(0b0000_0000)]
#[case::long_id_node(0b0000_0001)]
#[case::shared_dict_node(0b0000_1111)]
fn a_node_type_byte_the_format_has_no_meaning_for_is_rejected(#[case] byte: u8) {
    let mut bytes = struct_column_bytes();
    let (first_field_type, _) = region(&bytes, "type", 1);
    bytes[first_field_type] = byte;
    assert!(
        matches!(decode_err(&bytes), MltError::ParsingColumnType(b) if b == byte),
        "{:?}",
        decode_err(&bytes)
    );
}

#[test]
fn a_struct_node_with_no_fields_is_rejected() {
    let mut bytes = struct_column_bytes();
    let (offset, len) = region(&bytes, "field_count", 0);
    assert_eq!(len, 1);
    bytes[offset] = 0;
    assert!(matches!(decode_err(&bytes), MltError::EmptyStructNode));
}

#[test]
fn a_field_name_repeated_within_a_struct_is_rejected() {
    let mut bytes = struct_column_bytes();
    let at = bytes
        .windows(3)
        .position(|w| w == b"\x02ab")
        .expect("the second field's name");
    bytes[at + 2] = b'a';
    assert!(
        matches!(decode_err(&bytes), MltError::DuplicateFieldName(ref n) if n == "aa"),
        "{:?}",
        decode_err(&bytes)
    );
}

#[test]
fn a_nested_column_in_the_m_value_section_is_rejected() {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let m = builder
        .add_m_value("m", PropKind::I32)
        .expect("add_m_value");
    for i in 0..2 {
        let mut feature = builder.feature(line(&[(0, i), (1, i)]));
        feature
            .m_value(m, mlt_core::MValue::I32(Some(vec![i, i + 1])))
            .expect("m_value");
        feature.finish().expect("push");
    }
    let mut bytes = builder.finish().encode(cfg_v2()).expect("encode");
    let (offset, _) = region(&bytes, "type", 0);
    bytes[offset] = 0x0C;
    assert!(matches!(
        decode_err(&bytes),
        MltError::ParsingColumnType(0x0C)
    ));
}

#[test]
fn a_leading_stream_with_no_count_where_none_is_implied_is_rejected() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::list([i32_value(1), i32_value(2)]),
        NestedValue::list([i32_value(3)]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    let mut bytes = layer.encode(cfg_v2()).expect("encode");
    let (element_leaf_encoding, _) = region(&bytes, "encoding", encoding_index(&bytes, "data"));
    assert_ne!(bytes[element_leaf_encoding] & 0b1000_0000, 0);
    bytes[element_leaf_encoding] &= 0b0111_1111;
    assert!(
        matches!(decode_err(&bytes), MltError::NestedImplicitCount { .. }),
        "{:?}",
        decode_err(&bytes)
    );
}

fn encoding_index(bytes: &[u8], label: &str) -> usize {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    let mut seen = 0;
    let mut inside = false;
    for r in &tree.regions {
        if r.label == label && r.container {
            inside = true;
        }
        if r.label == "encoding" {
            if inside {
                return seen;
            }
            seen += 1;
        }
    }
    panic!("no encoding region inside {label}")
}

#[test]
fn a_lengths_sum_the_node_below_disagrees_with_is_rejected() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::list([i32_value(1), i32_value(2)]),
        NestedValue::list([i32_value(3)]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    let mut bytes = layer.encode(cfg_v2()).expect("encode");
    let (offset, len) = payload_of(&bytes, "lengths", 0);
    let zigzag_two_then_one = [4, 1];
    let zigzag_three = 6;
    assert_eq!(&bytes[offset..offset + len], &zigzag_two_then_one);
    bytes[offset] = zigzag_three;
    assert!(
        matches!(
            decode_err(&bytes),
            MltError::NestedCountMismatch {
                expected: 5,
                actual: 3
            }
        ),
        "{:?}",
        decode_err(&bytes)
    );
}

#[test]
fn a_nested_name_that_repeats_a_property_column_on_the_wire_is_rejected() {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let flat = builder
        .add_property("flat", PropKind::I32)
        .expect("add_property");
    let key = builder
        .add_nested("nest", map_kind(&[("a", leaf(PropKind::I32))]))
        .expect("add_nested");
    for i in 0..2 {
        let mut feature = builder.feature(point(i, i));
        feature
            .property(flat, PropValue::I32(Some(i)))
            .expect("property");
        feature
            .nested(key, entries(&[("a", i32_value(i))]))
            .expect("nested");
        feature.finish().expect("push");
    }
    let mut bytes = builder.finish().encode(cfg_v2()).expect("encode");
    let at = bytes
        .windows(5)
        .position(|w| w == b"\x04nest")
        .expect("the nested column's name");
    bytes[at + 1..at + 5].copy_from_slice(b"flat");
    assert_eq!(
        decode_err(&bytes).to_string(),
        "duplicate column name flat: the nested column repeats the property column"
    );
}

#[test]
fn an_m_value_name_that_repeats_a_nested_column_is_rejected() {
    let mut builder = TileLayer::builder("nested", 4096).expect("builder");
    let m = builder
        .add_m_value("mval", PropKind::I32)
        .expect("add_m_value");
    let key = builder
        .add_nested("nest", NestedKind::list(leaf(PropKind::Str)))
        .expect("add_nested");
    for i in 0..2 {
        let mut feature = builder.feature(line(&[(0, i), (1, i)]));
        feature
            .m_value(m, mlt_core::MValue::I32(Some(vec![i, i + 1])))
            .expect("m_value");
        feature
            .nested(key, NestedValue::list([str_value("x")]))
            .expect("nested");
        feature.finish().expect("push");
    }
    let mut bytes = builder.finish().encode(cfg_v2()).expect("encode");
    let at = bytes
        .windows(5)
        .position(|w| w == b"\x04mval")
        .expect("the m-value column's name");
    bytes[at + 1..at + 5].copy_from_slice(b"nest");
    assert_eq!(
        decode_err(&bytes).to_string(),
        "duplicate column name nest: the m-value column repeats the nested column"
    );
}

#[test]
fn a_root_holding_fewer_values_than_its_column_is_rejected() {
    let kind = NestedKind::list(leaf(PropKind::I32));
    let values = vec![
        NestedValue::list([i32_value(1)]),
        NestedValue::List(None),
        NestedValue::list([i32_value(2)]),
    ];
    let layer = nested_layer(kind, &[point(0, 0), point(1, 1), point(2, 2)], &values);
    let mut bytes = layer.encode(cfg_v2()).expect("encode");
    let (offset, len) = region(&bytes, "present", 0);
    assert_eq!(len, 1);
    assert_eq!(bytes[offset], 0b0000_0101);
    bytes[offset] = 0b0000_0111;
    assert!(
        matches!(
            decode_err(&bytes),
            MltError::NestedRootCountMismatch {
                ref name,
                expected: 3,
                actual: 2,
            } if name == "n"
        ),
        "{:?}",
        decode_err(&bytes)
    );
}

#[test]
fn a_node_presence_stream_that_is_not_a_raw_bitmap_is_rejected() {
    let kind = map_kind(&[("a", leaf(PropKind::I32)), ("b", leaf(PropKind::I32))]);
    let values = vec![
        entries(&[("a", i32_value(1)), ("b", i32_value(2))]),
        entries(&[("a", i32_value(3))]),
    ];
    let layer = nested_layer(kind, &two_points(), &values);
    let mut bytes = layer.encode(cfg_v2()).expect("encode");
    let (offset, _) = region(&bytes, "encoding", encoding_index(&bytes, "present"));
    bytes[offset] = (bytes[offset] & 0b1000_0000) | 0b0001_0000;
    let patched = bytes[offset];
    assert!(
        matches!(
            decode_err(&bytes),
            MltError::NestedPresenceEncoding { ref name, byte } if name == "n.b" && byte == patched
        ),
        "{:?}",
        decode_err(&bytes)
    );
}

/// What the high nibble of every type byte in the tile means, in wire order.
fn type_nibbles(bytes: &[u8]) -> Vec<String> {
    let tree = annotate_tile(bytes).expect("annotate_tile");
    tree.regions
        .iter()
        .filter(|r| r.label == "type")
        .filter_map(|r| r.bits.first().map(|b| b.meaning.clone()))
        .collect()
}

/// A layer whose string fields draw on one vocabulary, which one corpus holds.
fn shared_vocabulary_layer(sidewalk: [Option<&str>; 4]) -> TileLayer {
    const SURFACES: [&str; 4] = ["asphalt", "concrete", "gravel", "paving_stones"];
    let kind = map_kind(&[
        ("surface", leaf(PropKind::Str)),
        ("sidewalk", leaf(PropKind::Str)),
        ("shoulder", leaf(PropKind::Str)),
        ("lanes", leaf(PropKind::I32)),
    ]);
    let geometries: Vec<Geometry<i32>> = (0..4).map(|i| point(i, i)).collect();
    let values: Vec<NestedValue> = (0..4)
        .map(|i| {
            let mut fields = vec![
                ("surface", str_value(SURFACES[i])),
                ("shoulder", str_value(SURFACES[3 - i])),
                ("lanes", i32_value(i32::try_from(i).expect("a small index"))),
            ];
            if let Some(value) = sidewalk[i] {
                fields.push(("sidewalk", str_value(value)));
            }
            entries(&fields)
        })
        .collect();
    nested_layer(kind, &geometries, &values)
}

#[test]
fn string_fields_over_one_vocabulary_index_one_corpus() {
    let layer =
        shared_vocabulary_layer(["gravel", "asphalt", "paving_stones", "concrete"].map(Some));
    let bytes = assert_round_trips_as_v2(&layer);
    assert_eq!(
        type_nibbles(&bytes),
        [
            "presence = AllPresent",
            "node presence = AllPresent",
            "node presence = SharedAllPresent",
            "node presence = SharedAllPresent",
            "node presence = SharedAllPresent",
            "corpus = CorpusPlain",
        ]
    );
}

#[test]
fn a_shared_leaf_that_is_null_on_a_feature_round_trips() {
    let layer = shared_vocabulary_layer([Some("gravel"), None, Some("paving_stones"), None]);
    let bytes = assert_round_trips_as_v2(&layer);
    assert_eq!(
        type_nibbles(&bytes),
        [
            "presence = AllPresent",
            "node presence = AllPresent",
            "node presence = SharedAllPresent",
            "node presence = SharedStream",
            "node presence = SharedAllPresent",
            "corpus = CorpusPlain",
        ]
    );
}

#[test]
fn a_corpus_index_past_the_last_corpus_is_rejected() {
    let layer =
        shared_vocabulary_layer(["gravel", "asphalt", "paving_stones", "concrete"].map(Some));
    let mut bytes = layer.encode(cfg_v2()).expect("encode");
    let (offset, len) = region(&bytes, "corpus_index", 0);
    assert_eq!((len, bytes[offset]), (1, 0));
    bytes[offset] = 1;
    assert!(
        matches!(
            decode_err(&bytes),
            MltError::NestedCorpusOutOfRange { index: 1, len: 1 }
        ),
        "{:?}",
        decode_err(&bytes)
    );
}
