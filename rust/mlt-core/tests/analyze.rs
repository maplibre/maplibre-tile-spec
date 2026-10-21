//! Statistics and stream walks over the layers a tile parses into.

use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, ExplicitEncoder, FloatEncoding, IntEncoder, Network, Profile,
    StagedId, StagedLayer, StagedProperty, StrEncoding, VertexBufferType, WireVersion,
};
use mlt_core::geo_types::{Coord, Geometry, LineString, Point};
use mlt_core::wire::{Analyze as _, StatType};
use mlt_core::{
    Decoder, GeometryValues, Layer, MValue, NestedKind, NestedValue, Parser, PropKind, PropValue,
    TileLayer,
};

fn point(x: i32, y: i32) -> Geometry<i32> {
    Geometry::Point(Point::new(x, y))
}

fn line(pts: &[(i32, i32)]) -> Geometry<i32> {
    Geometry::LineString(LineString::new(
        pts.iter().map(|&(x, y)| Coord { x, y }).collect(),
    ))
}

fn narrow_ints_and_long_strings() -> TileLayer {
    let mut builder = TileLayer::builder("layer", 4096).unwrap();
    let i8_key = builder.add_property("i8", PropKind::I8).unwrap();
    let u8_key = builder.add_property("u8", PropKind::U8).unwrap();
    let str_key = builder.add_property("name", PropKind::Str).unwrap();
    for i in 0..128_u8 {
        let mut feature = builder.feature(point(i32::from(i), 0));
        feature
            .property(i8_key, PropValue::I8(Some(i.cast_signed())))
            .unwrap();
        feature.property(u8_key, PropValue::U8(Some(i))).unwrap();
        let name = format!(
            "{i} long street name that repeats its words, long street name that repeats its words"
        );
        feature
            .property(str_key, PropValue::Str(Some(name.into())))
            .unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

#[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
fn stream_labels(layer: &Layer<'_>) -> Vec<String> {
    let mut labels = Vec::new();
    let mut push = |meta: mlt_core::wire::StreamMeta| {
        labels.push(format!("{} {}", meta.stream_type, meta.encoding.logical));
    };
    match layer {
        Layer::Tag01(l) => l.for_each_stream(&mut push),
        Layer::Tag02(l) => l.for_each_stream(&mut push),
        _ => panic!("expected a known layer"),
    }
    labels
}

fn statistics(layer: &impl mlt_core::wire::Analyze) -> [usize; 3] {
    [
        StatType::DecodedDataSize,
        StatType::DecodedMetaSize,
        StatType::FeatureCount,
    ]
    .map(|stat| layer.collect_statistic(stat))
}

#[test]
fn a_v1_layer_walks_every_stream_of_its_widened_ints_and_fsst_strings() {
    let bytes = narrow_ints_and_long_strings()
        .encode(EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)).with_fsst(true))
        .unwrap();
    let layers = Parser::default().parse_layers(&bytes).unwrap();

    insta::assert_debug_snapshot!(stream_labels(&layers[0]), @r#"
    [
        "length[var-binary] int/delta-rle",
        "data[vertex] vertex/componentwise-delta",
        "data int/delta-rle",
        "data int/delta-rle",
        "length[symbol] int/rle",
        "data[fsst] int/none",
        "length[dictionary] int/rle",
        "data[single] int/none",
        "offset[string] int/delta-rle",
    ]
    "#);
}

#[test]
fn a_plain_fsst_string_column_walks_and_decodes() {
    let mut geometry = GeometryValues::default();
    geometry.push_geom(&point(0, 0));
    geometry.push_geom(&point(1, 1));
    let staged = StagedLayer::new(
        "layer",
        4096,
        StagedId::None,
        geometry,
        vec![StagedProperty::str("name", ["alpha", "beta"])],
    )
    .unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Fsst),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02);
    let bytes = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .unwrap()
        .into_layer_bytes()
        .unwrap();
    let layers = Parser::default().parse_layers(&bytes).unwrap();
    let labels = stream_labels(&layers[0]);
    let tile = layers
        .into_iter()
        .next()
        .unwrap()
        .into_tile(&mut Decoder::default())
        .unwrap()
        .expect("a known layer");

    assert_eq!(
        labels[labels.len() - 4..],
        [
            "length[symbol] int/none",
            "data[fsst] int/none",
            "length[dictionary] int/none",
            "data[single] int/none",
        ]
    );
    assert_eq!(
        tile.features()
            .iter()
            .map(|f| f.properties()[0].clone())
            .collect::<Vec<_>>(),
        [
            PropValue::Str(Some("alpha".into())),
            PropValue::Str(Some("beta".into()))
        ]
    );
}

#[test]
fn a_lazy_v1_layer_reports_only_its_metadata_until_decoded() {
    let bytes = narrow_ints_and_long_strings()
        .encode(EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)).with_fsst(true))
        .unwrap();
    let mut layers = Parser::default().parse_layers(&bytes).unwrap();
    let Layer::Tag01(lazy) = layers.remove(0) else {
        panic!("expected a v1 layer")
    };
    let lazy_stats = statistics(&lazy);
    let parsed = lazy.decode_all(&mut Decoder::default()).unwrap();

    insta::assert_debug_snapshot!((lazy_stats, statistics(&parsed)), @"
    (
        [
            0,
            9,
            0,
        ],
        [
            12818,
            9,
            128,
        ],
    )
    ");
}

#[test]
fn a_v2_layer_walks_the_streams_of_its_m_values_and_nested_leaves() {
    let mut builder = TileLayer::builder("layer", 4096).unwrap();
    let signed = builder.add_property("p_i8", PropKind::I8).unwrap();
    let unsigned = builder.add_property("p_u8", PropKind::U8).unwrap();
    let m_i8 = builder.add_m_value("m_i8", PropKind::I8).unwrap();
    let m_f32 = builder.add_m_value("m_f32", PropKind::F32).unwrap();
    let m_str = builder.add_m_value("m_str", PropKind::Str).unwrap();
    let leaves = NestedKind::map([
        ("bool", NestedKind::Leaf(PropKind::Bool)),
        ("u8", NestedKind::Leaf(PropKind::U8)),
        ("u32", NestedKind::Leaf(PropKind::U32)),
        ("i64", NestedKind::Leaf(PropKind::I64)),
        ("f32", NestedKind::Leaf(PropKind::F32)),
    ]);
    let nested = builder.add_nested("n", leaves).unwrap();
    for i in 0..2 {
        let mut feature = builder.feature(line(&[(0, i), (1, i)]));
        feature.property(signed, PropValue::I8(Some(-1))).unwrap();
        feature.property(unsigned, PropValue::U8(Some(1))).unwrap();
        feature.m_value(m_i8, MValue::I8(Some(vec![1, 2]))).unwrap();
        feature
            .m_value(m_f32, MValue::F32(Some(vec![0.5, 1.5])))
            .unwrap();
        feature
            .m_value(m_str, MValue::Str(Some(vec!["a".into(), "b".into()])))
            .unwrap();
        feature
            .nested(
                nested,
                NestedValue::map([
                    ("bool", NestedValue::Leaf(PropValue::Bool(Some(true)))),
                    ("u8", NestedValue::Leaf(PropValue::U8(Some(1)))),
                    ("u32", NestedValue::Leaf(PropValue::U32(Some(2)))),
                    ("i64", NestedValue::Leaf(PropValue::I64(Some(-3)))),
                    ("f32", NestedValue::Leaf(PropValue::F32(Some(0.25)))),
                ]),
            )
            .unwrap();
        feature.finish().unwrap();
    }
    let bytes = builder
        .finish()
        .encode(
            EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
                .with_wire_version(WireVersion::V02),
        )
        .unwrap();
    let layers = Parser::default().parse_layers(&bytes).unwrap();

    insta::assert_debug_snapshot!(stream_labels(&layers[0]), @r#"
    [
        "length[parts] int/delta",
        "data[vertex] vertex/componentwise-delta",
        "data int/delta",
        "data int/delta",
        "data bool/none",
        "data float/none",
        "data int/delta",
        "data int/delta",
        "data int/delta",
        "data int/delta",
        "data float/none",
        "length[var-binary] int/rle",
        "data int/none",
    ]
    "#);
}
