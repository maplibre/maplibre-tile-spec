use super::*;
use crate::structures::v1::{Column, ColumnType, FeatureMetaTable, FeatureTable};
use crate::structures::{Layer, Unknown, parse_binary_stream};
use crate::utils::{encode_str, encode_varint, parse_varint};

#[test]
fn test_varint_parsing() {
    // Test single byte varint
    assert_eq!(parse_varint(&[0x00]), Ok((&[][..], 0)));
    assert_eq!(parse_varint(&[0x7F]), Ok((&[][..], 127)));

    // Test multi-byte varint
    assert_eq!(parse_varint(&[0x80, 0x01]), Ok((&[][..], 128)));
    assert_eq!(
        parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
        Ok((&[][..], 0xFFFF_FFFF))
    );
}

#[test]
fn test_tuple_parsing() {
    // Create test data with metadata + data for Layer
    let mut data = Vec::new();

    let mut layer = Vec::new();
    // Layer name: "roads" (length=5, then "roads")
    encode_str(&mut layer, b"roads");

    // Extent: 4096
    encode_varint(&mut layer, 4096);

    // Column count: 6
    encode_varint(&mut layer, 6);

    // Column 1: type=1 (ID) - no name for ID columns
    encode_varint(&mut layer, 1); // column_type (Id)

    // Column 2: type=2 (Geometry) - no name for Geometry columns
    encode_varint(&mut layer, 2); // column_type (Geometry)

    // Column 3: type=3 (StringProperty) - has name
    encode_varint(&mut layer, 3); // column_type (StringProperty)
    encode_str(&mut layer, b"name"); // name

    // Column 4: type=4 (FloatProperty) - has name
    encode_varint(&mut layer, 4); // column_type (FloatProperty)
    encode_str(&mut layer, b"price"); // name

    // Column 5: type=6 (IntProperty) - has name
    encode_varint(&mut layer, 6); // column_type (IntProperty)
    encode_str(&mut layer, b"count"); // name

    // Column 6: type=9 (BoolProperty) - has name
    encode_varint(&mut layer, 9); // column_type (BoolProperty)
    encode_str(&mut layer, b"active"); // name

    // Add some additional data after metadata
    layer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);

    encode_varint(&mut data, layer.len() as u64 + 1); // size (1 for tag + data)
    encode_varint(&mut data, 1); // tag
    data.extend_from_slice(&layer); // value

    // Create expected metadata
    let expected_meta = FeatureMetaTable {
        name: "roads",
        extent: 4096,
        columns: vec![
            Column {
                typ: ColumnType::Id,
                name: None,
            },
            Column {
                typ: ColumnType::Geometry,
                name: None,
            },
            Column {
                typ: ColumnType::StringProperty,
                name: Some("name"),
            },
            Column {
                typ: ColumnType::FloatProperty,
                name: Some("price"),
            },
            Column {
                typ: ColumnType::IntProperty,
                name: Some("count"),
            },
            Column {
                typ: ColumnType::BoolProperty,
                name: Some("active"),
            },
        ],
    };

    let expected = Layer::Layer(FeatureTable {
        meta: expected_meta,
        data: &[0x01, 0x02, 0x03, 0x04, 0x05],
    });

    println!("Test data: {data:?}");
    let result = Layer::parse(&data);
    println!("Parse result: {result:?}");
    assert_eq!(result, Ok((&[][..], expected)));
}

#[test]
fn test_unknown_tuple_parsing() {
    let data = vec![
        0x04, // size = 4 (1 for tag + 3 for value)
        0x02, // tag = 2 (not 1, so should be Unknown)
        0x01, 0x02, 0x03, // value
    ];

    let expected = Layer::Unknown(Unknown {
        tag: 2,
        value: &[0x01, 0x02, 0x03],
    });

    assert_eq!(Layer::parse(&data), Ok((&[][..], expected)));
}

#[test]
fn test_layer_v1_meta_parsing() {
    // Create test data: name="test_layer", column_count=2, then two columns
    let mut data = Vec::new();

    // Layer name: "test_layer" (length=10, then "test_layer")
    encode_str(&mut data, b"test_layer");

    // Extent: 4096
    encode_varint(&mut data, 4096);

    // Column count: 2
    encode_varint(&mut data, 2);

    // Column 1: type=1 (ID) - no name for ID columns
    encode_varint(&mut data, 1); // column_type (Id)

    // Column 2: type=2 (Geometry) - no name for Geometry columns
    encode_varint(&mut data, 2); // column_type (Geometry)

    let expected = FeatureMetaTable {
        name: "test_layer",
        extent: 4096,
        columns: vec![
            Column {
                typ: ColumnType::Id,
                name: None,
            },
            Column {
                typ: ColumnType::Geometry,
                name: None,
            },
        ],
    };

    assert_eq!(FeatureMetaTable::parse(&data), Ok((&[][..], expected)));
}

#[test]
fn test_stream_parsing() {
    let data = create_test_data();
    let result = parse_binary_stream(&data);

    assert!(result.is_ok());
    let (remaining, layers) = result.unwrap();
    assert!(remaining.is_empty());
    assert_eq!(layers.len(), 3);

    // Check first layer (tag=1, should be Layer)
    match &layers[0] {
        Layer::Layer(layer_v1) => {
            assert_eq!(layer_v1.meta.name, "roads");
            assert_eq!(layer_v1.meta.extent, 4096);
            assert_eq!(layer_v1.meta.columns.len(), 6);
            assert_eq!(layer_v1.meta.columns[0].name, None);
            assert_eq!(layer_v1.meta.columns[0].typ, ColumnType::Id);
            assert_eq!(layer_v1.meta.columns[1].name, None);
            assert_eq!(layer_v1.meta.columns[1].typ, ColumnType::Geometry);
            assert_eq!(layer_v1.meta.columns[2].name, Some("name"));
            assert_eq!(layer_v1.meta.columns[2].typ, ColumnType::StringProperty);
            assert_eq!(layer_v1.meta.columns[3].name, Some("price"));
            assert_eq!(layer_v1.meta.columns[3].typ, ColumnType::FloatProperty);
            assert_eq!(layer_v1.meta.columns[4].name, Some("count"));
            assert_eq!(layer_v1.meta.columns[4].typ, ColumnType::IntProperty);
            assert_eq!(layer_v1.meta.columns[5].name, Some("active"));
            assert_eq!(layer_v1.meta.columns[5].typ, ColumnType::BoolProperty);
            assert_eq!(layer_v1.data, &[0x01, 0x02, 0x03, 0x04, 0x05]);
        }
        Layer::Unknown(_) => panic!("Expected Layer layer"),
    }

    // Check second layer (tag=42, should be Unknown)
    match &layers[1] {
        Layer::Unknown(unknown) => {
            assert_eq!(unknown.tag, 42);
            assert_eq!(unknown.value, &[0xAA, 0xBB]);
        }
        Layer::Layer(_) => panic!("Expected Unknown layer"),
    }

    // Check third layer (tag=100, should be Unknown)
    match &layers[2] {
        Layer::Unknown(unknown) => {
            assert_eq!(unknown.tag, 100);
            assert!(unknown.value.is_empty());
        }
        Layer::Layer(_) => panic!("Expected Unknown layer"),
    }
}
