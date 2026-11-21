//! A [`LayerWriter`] holding nested columns encodes to the same bytes as a [`TileLayer`].

use geo_types::{Coord, Geometry, Point};
use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
use mlt_core::fast_mvt::MvtTileBuilder;
use mlt_core::mvt::tile_layers_to_mvt;
use mlt_core::{
    Decoder, GeometryType, Layer, LayerWriter, MltError, NestedKey, NestedKind, NestedValue,
    Parser, PropKind, PropValue, TileLayer,
};
use rstest::rstest;

fn v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
}

fn pt(x: i32, y: i32) -> Coord<i32> {
    Coord { x, y }
}

fn s(v: &str) -> NestedValue {
    NestedValue::Leaf(PropValue::Str(Some(v.into())))
}

fn i(v: i64) -> NestedValue {
    NestedValue::Leaf(PropValue::I64(Some(v)))
}

fn scalars() -> NestedKind {
    NestedKind::list(NestedKind::Leaf(PropKind::I64))
}

fn objects() -> NestedKind {
    NestedKind::list(NestedKind::map([
        ("name", NestedKind::Leaf(PropKind::Str)),
        ("n", NestedKind::Leaf(PropKind::I64)),
    ]))
}

fn deep() -> NestedKind {
    NestedKind::map([
        ("tags", NestedKind::list(NestedKind::Leaf(PropKind::Str))),
        ("id", NestedKind::Leaf(PropKind::I64)),
    ])
}

/// One column's kind and the value each of the three features sets, `None` for none at all.
type Case = (NestedKind, [Option<NestedValue>; 3]);

fn scalar_case() -> Case {
    (
        scalars(),
        [
            Some(NestedValue::list([i(1), i(2), i(3)])),
            Some(NestedValue::List(None)),
            Some(NestedValue::list([])),
        ],
    )
}

fn object_case() -> Case {
    (
        objects(),
        [
            Some(NestedValue::list([
                NestedValue::map([("name", s("a")), ("n", i(1))]),
                NestedValue::map([("name", s("b"))]),
            ])),
            None,
            Some(NestedValue::list([NestedValue::map([("n", i(9))])])),
        ],
    )
}

fn deep_case() -> Case {
    (
        deep(),
        [
            Some(NestedValue::map([
                ("tags", NestedValue::list([s("x"), s("y")])),
                ("id", i(7)),
            ])),
            Some(NestedValue::map([("id", i(8))])),
            Some(NestedValue::Map(None)),
        ],
    )
}

fn owned(cases: &[Case]) -> TileLayer {
    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let keys: Vec<NestedKey> = cases
        .iter()
        .enumerate()
        .map(|(n, (kind, _))| builder.add_nested(format!("col{n}"), kind.clone()).unwrap())
        .collect();
    for (f, x) in (0..3).zip(0_i32..) {
        let mut feature = builder.feature(Geometry::Point(Point::new(x, 2 * x)));
        for (key, (_, values)) in keys.iter().zip(cases) {
            if let Some(value) = &values[f] {
                feature.nested(*key, value.clone()).unwrap();
            }
        }
        feature.finish().unwrap();
    }
    builder.finish()
}

fn written(cases: &[Case]) -> LayerWriter<'static> {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    fill(&mut writer, cases);
    writer
}

fn fill(writer: &mut LayerWriter<'static>, cases: &[Case]) {
    let keys: Vec<NestedKey> = cases
        .iter()
        .enumerate()
        .map(|(n, (kind, _))| writer.add_nested(&format!("col{n}"), kind.clone()).unwrap())
        .collect();
    for (f, x) in (0..3).zip(0_i32..) {
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([pt(x, 2 * x)]).unwrap();
        for (key, (_, values)) in keys.iter().zip(cases) {
            if let Some(value) = &values[f] {
                feature.nested(*key, value.clone()).unwrap();
            }
        }
        feature.finish().unwrap();
    }
}

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let Some(Layer::Tag02(layer)) = layers.into_iter().next() else {
        panic!("expected one v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).unwrap()
}

#[rstest]
fn writer_nested_encodes_like_tile_layer(
    #[values(
        vec![scalar_case()],
        vec![object_case()],
        vec![deep_case()],
        vec![scalar_case(), object_case(), deep_case()]
    )]
    cases: Vec<Case>,
) {
    let layer = owned(&cases);
    let writer = written(&cases);
    assert_eq!(writer.encode(v2()).unwrap(), layer.encode(v2()).unwrap());
}

#[rstest]
fn writer_nested_decodes_to_the_values_written(
    #[values(scalar_case(), object_case(), deep_case())] case: Case,
) {
    let writer = written(std::slice::from_ref(&case));
    let decoded = decode(&writer.encode(v2()).unwrap());
    let expected = owned(std::slice::from_ref(&case));
    for (got, want) in decoded.features().iter().zip(expected.features()) {
        assert_eq!(got.nested(), want.nested());
    }
}

#[test]
fn feature_without_a_value_is_null() {
    let writer = written(&[object_case()]);
    let decoded = decode(&writer.encode(v2()).unwrap());
    assert!(decoded.features()[1].nested()[0].is_null());
}

#[test]
fn nested_column_added_after_features_encodes_like_tile_layer() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    for n in 0..2 {
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([pt(n, n)]).unwrap();
        feature.finish().unwrap();
        builder
            .feature(Geometry::Point(Point::new(n, n)))
            .finish()
            .unwrap();
    }
    let key = writer.add_nested("late", scalars()).unwrap();
    let owned_key = builder.add_nested("late", scalars()).unwrap();
    let value = NestedValue::list([i(4)]);
    let mut feature = writer.feature(GeometryType::Point);
    feature.points([pt(2, 2)]).unwrap();
    feature.nested(key, value.clone()).unwrap();
    feature.finish().unwrap();
    let mut feature = builder.feature(Geometry::Point(Point::new(2, 2)));
    feature.nested(owned_key, value).unwrap();
    feature.finish().unwrap();
    assert_eq!(
        writer.encode(v2()).unwrap(),
        builder.finish().encode(v2()).unwrap()
    );
}

#[test]
fn dropped_feature_leaves_the_layer_as_it_was() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let key = writer.add_nested("col0", scalars()).unwrap();
    {
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([pt(9, 9)]).unwrap();
        feature.nested(key, NestedValue::list([i(5)])).unwrap();
    }
    let mut fresh = LayerWriter::new("pois", 4096).unwrap();
    fresh.add_nested("col0", scalars()).unwrap();
    assert_eq!(writer.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn reset_writer_encodes_like_a_new_one() {
    let first = [scalar_case()];
    let second = [object_case(), deep_case()];
    let mut reused = written(&first);
    reused.reset("pois", 4096).unwrap();
    fill(&mut reused, &second);
    assert_eq!(
        reused.encode(v2()).unwrap(),
        written(&second).encode(v2()).unwrap()
    );
}

#[test]
fn reset_writer_drops_its_nested_columns() {
    let mut reused = written(&[scalar_case()]);
    reused.reset("pois", 4096).unwrap();
    let mut feature = reused.feature(GeometryType::Point);
    feature.points([pt(1, 1)]).unwrap();
    feature.finish().unwrap();
    let mut fresh = LayerWriter::new("pois", 4096).unwrap();
    let mut feature = fresh.feature(GeometryType::Point);
    feature.points([pt(1, 1)]).unwrap();
    feature.finish().unwrap();
    assert_eq!(reused.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn rejects_value_of_another_shape() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let key = writer.add_nested("col", scalars()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    assert!(matches!(
        feature.nested(key, NestedValue::Map(None)).err().unwrap(),
        MltError::NestedShapeMismatch { index: 0 }
    ));
}

#[test]
fn rejects_value_the_kind_does_not_accept() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let key = writer.add_nested("col", scalars()).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    assert!(matches!(
        feature
            .nested(key, NestedValue::list([s("not a number")]))
            .err()
            .unwrap(),
        MltError::NestedValueMismatch { index: 0, .. }
    ));
}

#[test]
fn rejects_unknown_key() {
    let mut other = LayerWriter::new("pois", 4096).unwrap();
    other.add_nested("a", scalars()).unwrap();
    let foreign = other.add_nested("b", scalars()).unwrap();
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    assert!(matches!(
        feature
            .nested(foreign, NestedValue::List(None))
            .err()
            .unwrap(),
        MltError::UnknownNestedKey {
            index: 1,
            columns: 0
        }
    ));
}

#[rstest]
#[case::scalar_root(NestedKind::Leaf(PropKind::I64))]
#[case::empty_map(NestedKind::Map(std::collections::BTreeMap::default()))]
#[case::too_deep(
    (0..8).fold(NestedKind::Leaf(PropKind::I64), |kind, _| NestedKind::list(kind))
)]
fn rejects_bad_kind_like_tile_layer(#[case] kind: NestedKind) {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    assert_eq!(
        writer
            .add_nested("col", kind.clone())
            .unwrap_err()
            .to_string(),
        builder.add_nested("col", kind).unwrap_err().to_string()
    );
}

#[test]
fn rejects_nested_named_like_a_property() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    writer.add_property("name", PropKind::Str).unwrap();
    assert!(writer.add_nested("name", scalars()).is_err());
    writer.add_nested("tags", scalars()).unwrap();
    assert!(writer.add_property("tags", PropKind::Str).is_err());
    assert!(writer.add_nested("tags", scalars()).is_err());
}

#[test]
fn v1_rejects_writer_nested_like_tile_layer() {
    let cases = [scalar_case()];
    let v1 = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V01);
    assert_eq!(
        written(&cases).encode(v1).unwrap_err().to_string(),
        owned(&cases).encode(v1).unwrap_err().to_string()
    );
}

#[test]
fn writer_mvt_drops_nested_like_tile_layer() {
    let cases = [scalar_case(), deep_case()];
    assert_eq!(
        written(&cases)
            .write_mvt(MvtTileBuilder::new())
            .unwrap()
            .encode(),
        tile_layers_to_mvt([&owned(&cases)]).unwrap()
    );
}
