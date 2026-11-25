//! A [`LayerWriter`] holding nested columns encodes to the same bytes as a [`TileLayer`].

use std::collections::BTreeMap;

use geo_types::{Coord, Geometry, Point};
use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
use mlt_core::fast_mvt::MvtTileBuilder;
use mlt_core::mvt::tile_layers_to_mvt;
use mlt_core::{
    Decoder, GeometryType, Layer, LayerWriter, MltError, NestedKind, NestedValue, Parser, PropKind,
    PropValue, TileLayer,
};
use rstest::rstest;

fn v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
}

fn text(v: &str) -> NestedValue {
    NestedValue::Leaf(PropValue::Str(Some(v.into())))
}

fn int(v: i64) -> NestedValue {
    NestedValue::Leaf(PropValue::I64(Some(v)))
}

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let Some(Layer::Tag02(layer)) = layers.into_iter().next() else {
        panic!("expected one v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).unwrap()
}

#[test]
fn list_of_scalars_encodes_like_tile_layer() {
    let kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer.add_nested("numbers", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1), int(2), int(3)]))
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 2 }]).unwrap();
    feature.nested(numbers, NestedValue::List(None)).unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 2, y: 4 }]).unwrap();
    feature.nested(numbers, NestedValue::list([])).unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let numbers = builder.add_nested("numbers", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(numbers, NestedValue::list([int(1), int(2), int(3)]))
        .unwrap();
    feature.finish().unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(1, 2)));
    feature.nested(numbers, NestedValue::List(None)).unwrap();
    feature.finish().unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(2, 4)));
    feature.nested(numbers, NestedValue::list([])).unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn list_of_maps_encodes_like_tile_layer() {
    let kind = NestedKind::list(NestedKind::map([
        ("name", NestedKind::Leaf(PropKind::Str)),
        ("n", NestedKind::Leaf(PropKind::I64)),
    ]));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let entries = writer.add_nested("entries", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(
            entries,
            NestedValue::list([
                NestedValue::map([("name", text("a")), ("n", int(1))]),
                NestedValue::map([("name", text("b"))]),
            ]),
        )
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 2 }]).unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let entries = builder.add_nested("entries", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(
            entries,
            NestedValue::list([
                NestedValue::map([("name", text("a")), ("n", int(1))]),
                NestedValue::map([("name", text("b"))]),
            ]),
        )
        .unwrap();
    feature.finish().unwrap();
    builder
        .feature(Geometry::Point(Point::new(1, 2)))
        .finish()
        .unwrap();

    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn map_with_a_nested_list_encodes_like_tile_layer() {
    let kind = NestedKind::map([
        ("tags", NestedKind::list(NestedKind::Leaf(PropKind::Str))),
        ("id", NestedKind::Leaf(PropKind::I64)),
    ]);

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let record = writer.add_nested("record", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(
            record,
            NestedValue::map([
                ("tags", NestedValue::list([text("x"), text("y")])),
                ("id", int(7)),
            ]),
        )
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 2 }]).unwrap();
    feature.nested(record, NestedValue::Map(None)).unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let record = builder.add_nested("record", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(
            record,
            NestedValue::map([
                ("tags", NestedValue::list([text("x"), text("y")])),
                ("id", int(7)),
            ]),
        )
        .unwrap();
    feature.finish().unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(1, 2)));
    feature.nested(record, NestedValue::Map(None)).unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn two_nested_columns_encode_like_tile_layer() {
    let numbers_kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));
    let tags_kind = NestedKind::list(NestedKind::Leaf(PropKind::Str));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer.add_nested("numbers", numbers_kind.clone()).unwrap();
    let tags = writer.add_nested("tags", tags_kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature
        .nested(tags, NestedValue::list([text("a")]))
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 2 }]).unwrap();
    feature
        .nested(tags, NestedValue::list([text("b")]))
        .unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let numbers = builder.add_nested("numbers", numbers_kind).unwrap();
    let tags = builder.add_nested("tags", tags_kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature
        .nested(tags, NestedValue::list([text("a")]))
        .unwrap();
    feature.finish().unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(1, 2)));
    feature
        .nested(tags, NestedValue::list([text("b")]))
        .unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn decodes_to_the_values_written_and_null_where_none_was() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let entries = writer
        .add_nested(
            "entries",
            NestedKind::list(NestedKind::map([
                ("name", NestedKind::Leaf(PropKind::Str)),
                ("n", NestedKind::Leaf(PropKind::I64)),
            ])),
        )
        .unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(
            entries,
            NestedValue::list([NestedValue::map([("name", text("a")), ("n", int(1 << 40))])]),
        )
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 2 }]).unwrap();
    feature.finish().unwrap();

    let decoded = decode(&writer.encode(v2()).unwrap());

    assert_eq!(
        decoded.features()[0].nested(),
        [NestedValue::list([NestedValue::map([
            ("name", text("a")),
            ("n", int(1 << 40))
        ])])]
    );
    assert!(decoded.features()[1].nested()[0].is_null());
}

#[test]
fn nested_column_added_after_features_encodes_like_tile_layer() {
    let kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature.finish().unwrap();
    let late = writer.add_nested("late", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 1 }]).unwrap();
    feature.nested(late, NestedValue::list([int(4)])).unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    builder
        .feature(Geometry::Point(Point::new(0, 0)))
        .finish()
        .unwrap();
    let late = builder.add_nested("late", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(1, 1)));
    feature.nested(late, NestedValue::list([int(4)])).unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn dropped_feature_leaves_the_layer_as_it_was() {
    let kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer.add_nested("numbers", kind.clone()).unwrap();
    {
        let mut unfinished = writer.feature(GeometryType::Point);
        unfinished.points([Coord { x: 9, y: 9 }]).unwrap();
        unfinished
            .nested(numbers, NestedValue::list([int(5)]))
            .unwrap();
    }

    let mut fresh = LayerWriter::new("pois", 4096).unwrap();
    fresh.add_nested("numbers", kind).unwrap();

    assert_eq!(writer.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn reset_writer_encodes_like_a_new_one() {
    let tags_kind = NestedKind::list(NestedKind::Leaf(PropKind::Str));

    let mut reused = LayerWriter::new("pois", 4096).unwrap();
    let numbers = reused
        .add_nested("numbers", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let mut feature = reused.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1), int(2)]))
        .unwrap();
    feature.finish().unwrap();
    reused.reset("pois", 4096).unwrap();
    let tags = reused.add_nested("tags", tags_kind.clone()).unwrap();
    let mut feature = reused.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 1 }]).unwrap();
    feature
        .nested(tags, NestedValue::list([text("a")]))
        .unwrap();
    feature.finish().unwrap();

    let mut fresh = LayerWriter::new("pois", 4096).unwrap();
    let tags = fresh.add_nested("tags", tags_kind).unwrap();
    let mut feature = fresh.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 1 }]).unwrap();
    feature
        .nested(tags, NestedValue::list([text("a")]))
        .unwrap();
    feature.finish().unwrap();

    assert_eq!(reused.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn reset_writer_drops_its_nested_columns() {
    let mut reused = LayerWriter::new("pois", 4096).unwrap();
    let numbers = reused
        .add_nested("numbers", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let mut feature = reused.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature.finish().unwrap();
    reused.reset("pois", 4096).unwrap();
    let mut feature = reused.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 1 }]).unwrap();
    feature.finish().unwrap();

    let mut fresh = LayerWriter::new("pois", 4096).unwrap();
    let mut feature = fresh.feature(GeometryType::Point);
    feature.points([Coord { x: 1, y: 1 }]).unwrap();
    feature.finish().unwrap();

    assert_eq!(reused.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn rejects_value_of_another_shape() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer
        .add_nested("numbers", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let mut feature = writer.feature(GeometryType::Point);

    let err = feature
        .nested(numbers, NestedValue::Map(None))
        .err()
        .unwrap();

    assert!(matches!(err, MltError::NestedShapeMismatch { index: 0 }));
}

#[test]
fn rejects_value_the_kind_does_not_accept() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer
        .add_nested("numbers", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let mut feature = writer.feature(GeometryType::Point);

    let err = feature
        .nested(numbers, NestedValue::list([text("not a number")]))
        .err()
        .unwrap();

    assert!(matches!(
        err,
        MltError::NestedValueMismatch { index: 0, .. }
    ));
}

#[test]
fn rejects_a_key_of_another_writer() {
    let mut other = LayerWriter::new("pois", 4096).unwrap();
    other
        .add_nested("a", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let foreign = other
        .add_nested("b", NestedKind::list(NestedKind::Leaf(PropKind::I64)))
        .unwrap();
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut feature = writer.feature(GeometryType::Point);

    let err = feature
        .nested(foreign, NestedValue::List(None))
        .err()
        .unwrap();

    assert!(matches!(
        err,
        MltError::UnknownNestedKey {
            index: 1,
            columns: 0
        }
    ));
}

#[rstest]
#[case::scalar_root(NestedKind::Leaf(PropKind::I64))]
#[case::empty_map(NestedKind::Map(BTreeMap::default()))]
#[case::nine_levels(NestedKind::list(NestedKind::list(NestedKind::list(NestedKind::list(
    NestedKind::list(NestedKind::list(NestedKind::list(NestedKind::list(NestedKind::Leaf(
        PropKind::I64
    )))))
)))))]
fn rejects_a_bad_kind_with_the_error_tile_layer_gives(#[case] kind: NestedKind) {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut builder = TileLayer::builder("pois", 4096).unwrap();

    let writer_err = writer.add_nested("col", kind.clone()).unwrap_err();
    let builder_err = builder.add_nested("col", kind).unwrap_err();

    assert_eq!(writer_err.to_string(), builder_err.to_string());
}

#[test]
fn rejects_a_nested_column_named_like_a_property() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    writer.add_property("name", PropKind::Str).unwrap();

    let err = writer.add_nested("name", NestedKind::list(NestedKind::Leaf(PropKind::I64)));

    assert!(err.is_err());
}

#[test]
fn rejects_a_property_or_second_nested_column_named_like_a_nested_column() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    writer
        .add_nested("tags", NestedKind::list(NestedKind::Leaf(PropKind::Str)))
        .unwrap();

    assert!(writer.add_property("tags", PropKind::Str).is_err());
    assert!(
        writer
            .add_nested("tags", NestedKind::list(NestedKind::Leaf(PropKind::Str)))
            .is_err()
    );
}

#[test]
fn v1_rejects_nested_columns_with_the_error_tile_layer_gives() {
    let kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));
    let v1 = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V01);

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer.add_nested("numbers", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let numbers = builder.add_nested("numbers", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.encode(v1).unwrap_err().to_string(),
        builder.finish().encode(v1).unwrap_err().to_string()
    );
}

#[test]
fn mvt_drops_nested_columns_like_tile_layer() {
    let kind = NestedKind::list(NestedKind::Leaf(PropKind::I64));

    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let numbers = writer.add_nested("numbers", kind.clone()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([Coord { x: 0, y: 0 }]).unwrap();
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let numbers = builder.add_nested("numbers", kind).unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
    feature
        .nested(numbers, NestedValue::list([int(1)]))
        .unwrap();
    feature.finish().unwrap();

    assert_eq!(
        writer.write_mvt(MvtTileBuilder::new()).unwrap().encode(),
        tile_layers_to_mvt([&builder.finish()]).unwrap()
    );
}
