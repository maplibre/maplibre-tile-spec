//! What copying a layer's string properties out costs the decoder budget.

use std::path::Path;

use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, ExplicitEncoder, FloatEncoding, IntEncoder, Network, Profile,
    StagedId, StagedLayer, StagedProperty, StrEncoding, VertexBufferType,
};
use mlt_core::geo_types::{Geometry, Point};
use mlt_core::{
    Decoder, GeometryValues, Layer, LendingIterator as _, ParsedLayer, ParsedLayer01, Parser,
    PropValueRef,
};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/synthetic" as string_copy_budget => charge_is_the_bytes_of_every_copied_row }

fn charge_is_the_bytes_of_every_copied_row([path]: [&Path; 1]) {
    let data = std::fs::read(path).unwrap();
    let layers = Parser::default().parse_layers(&data).unwrap();
    let mut dec = Decoder::default();
    for layer in layers {
        let parsed = layer.decode_all(&mut dec).unwrap();
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "ParsedLayer is non_exhaustive"
        )]
        let layer: &ParsedLayer01<'_> = match &parsed {
            ParsedLayer::Tag01(l) => l,
            ParsedLayer::Tag02(l) => l.layer(),
            _ => continue,
        };

        let mut copied = 0;
        let mut features = layer.iter_features();
        while let Some(feature) = features.next() {
            for value in feature.unwrap().iter_all_properties().flatten() {
                if let PropValueRef::Str(s) = value {
                    copied += s.len();
                }
            }
        }

        let mut charged = Decoder::default();
        layer.charge_string_copies(&mut charged).unwrap();
        assert_eq!(usize::try_from(charged.consumed()).unwrap(), copied);
    }
}

#[test]
fn a_dictionary_entry_named_past_the_budget_is_refused() {
    let entry = "x".repeat(1000);
    let mut geometry = GeometryValues::default();
    for i in 0..100 {
        geometry.push_geom(&Geometry::Point(Point::new(i, i)));
    }
    let staged = StagedLayer::new(
        "layer",
        4096,
        StagedId::None,
        geometry,
        vec![StagedProperty::str("name", vec![entry.as_str(); 100])],
    )
    .unwrap();
    let explicit = ExplicitEncoder {
        vertex_buffer_type: VertexBufferType::Vec2,
        force_stream: Box::new(|_| false),
        get_int_encoder: Box::new(|_| IntEncoder::varint()),
        get_str_encoding: Box::new(|_| StrEncoding::Dict),
        get_float_encoding: Box::new(|_| FloatEncoding::None),
    };
    let cfg = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
    let bytes = staged
        .encode_into(
            Encoder::with_explicit(cfg, explicit),
            &mut Codecs::default(),
        )
        .unwrap()
        .into_layer_bytes()
        .unwrap();
    assert_eq!(bytes.len(), 1446);

    let Some(Layer::Tag01(layer)) = Parser::default().parse_layers(&bytes).unwrap().pop() else {
        panic!("expected a v1 layer")
    };
    let mut dec = Decoder::with_max_size(64 * 1024);
    let parsed = layer.decode_all(&mut dec).unwrap();
    let err = parsed.charge_string_copies(&mut dec).unwrap_err();
    insta::assert_snapshot!(err.to_string(), @"memory limit exceeded: limit=65536, used=2408, requested=100000");
}
