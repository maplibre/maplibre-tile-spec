//! Round-trip every MVT fixture in `test/fixtures/**/*.mvt`. The first
//! encode normalizes spec-permissible quirks (consecutive duplicate vertices,
//! axis-aligned collinear polygon points); subsequent re-encodes must be a
//! fixpoint, so we compare the once- and twice-normalized layers.

use std::fs;
use std::path::Path;

use mlt_core::encoder::{
    Codecs, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder, Presence, StagedId, StagedLayer,
    StagedProperty, StagedSharedDict, StrEncoding,
};
use mlt_core::geo_types::{Geometry, Point};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::test_helpers::assert_mvt_equivalent_layers;
use mlt_core::wire::{Analyze, DictionaryType, StreamType};
use mlt_core::{Decoder, GeometryValues, Parser, PropValue, TileLayer};
use test_each_file::test_each_path;

test_each_path! { for ["mvt"] in "../test/fixtures" as mvt_round_trip => round_trip_fixture }

fn round_trip_fixture([path]: [&Path; 1]) {
    let mvt_bytes = fs::read(path).expect("read fixture");
    let original = mvt_to_tile_layers(mvt_bytes)
        .unwrap_or_else(|e| panic!("{}: unexpected decode failure: {e}", path.display()));
    let normalized = re_encode(original);
    let again = re_encode(normalized.clone());

    assert_eq!(normalized.len(), again.len(), "layer count");
    for (a, b) in normalized.iter().zip(again.iter()) {
        assert_mvt_equivalent_layers(a, b);
    }
}

#[test]
fn fsst_shared_dict_uses_shared_data_stream_type() {
    let en = vec![
        Some("Main Street".to_string()),
        Some("Market Square".to_string()),
        Some("River Road".to_string()),
    ];
    let de = vec![
        Some("Hauptstrasse".to_string()),
        Some("Marktplatz".to_string()),
        Some("Flussweg".to_string()),
    ];
    let shared = StagedSharedDict::new(
        "name:",
        [
            ("en", en.clone(), Presence::AllPresent),
            ("de", de.clone(), Presence::AllPresent),
        ],
    )
    .expect("shared dict");

    let layer = StagedLayer::new(
        "shared_fsst",
        4096,
        StagedId::None,
        point_geometry(3),
        vec![StagedProperty::SharedDict(shared)],
    )
    .expect("staged layer");
    let mut codecs = Codecs::default();
    let bytes = layer
        .encode_into(
            Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all_with_str(IntEncoder::varint(), StrEncoding::Fsst),
            ),
            &mut codecs,
        )
        .expect("encode layer")
        .into_layer_bytes()
        .expect("layer bytes");

    let mut parser = Parser::default();
    let raw = parser
        .parse_layers(&bytes)
        .expect("parse layer")
        .remove(0)
        .into_layer01()
        .expect("tag 01 layer");
    let mut stream_types = Vec::new();
    raw.for_each_stream(&mut |meta| stream_types.push(meta.stream_type));

    assert!(
        stream_types.contains(&StreamType::Data(DictionaryType::Shared)),
        "shared FSST corpus stream should be tagged Data(Shared): {stream_types:#?}"
    );
    assert!(
        !stream_types.contains(&StreamType::Data(DictionaryType::Single)),
        "shared FSST corpus stream must not be tagged Data(Single): {stream_types:#?}"
    );

    let mut decoder = Decoder::default();
    let decoded = raw
        .decode_all(&mut decoder)
        .expect("decode layer")
        .into_tile(&mut decoder)
        .expect("tile layer");

    assert_eq!(decoded.property_names(), &["name:en", "name:de"]);
    for (i, feature) in decoded.features().iter().enumerate() {
        assert_eq!(
            feature.properties(),
            &[PropValue::Str(en[i].clone()), PropValue::Str(de[i].clone()),],
        );
    }
}

fn point_geometry(n: usize) -> GeometryValues {
    let mut geometry = GeometryValues::default();
    for i in 0..n {
        let point = Point::new(i as i32, i as i32);
        geometry.push_geom(&Geometry::Point(point));
    }
    geometry
}

fn re_encode(layers: Vec<TileLayer>) -> Vec<TileLayer> {
    let bytes = tile_layers_to_mvt(layers).expect("encode mvt");
    mvt_to_tile_layers(bytes).expect("decode re-encoded mvt")
}
