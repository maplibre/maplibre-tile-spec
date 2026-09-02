//! Differential test of the v2 wire format over every MVT fixture v2 can encode.

use std::fs;
use std::path::Path;

use mlt_core::dump::annotate_tile;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::{Decoder, Layer, MltError, Parser, TileLayer};
use rstest::rstest;
use test_each_file::test_each_path;

test_each_path! { for ["mvt"] in "../test/fixtures" as tag02_fixtures => differential_fixture }

fn cfg(version: WireVersion) -> EncoderConfig {
    EncoderConfig::default()
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false)
        .with_wire_version(version)
}

#[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
fn decode(bytes: &[u8], expected_tag: u8) -> TileLayer {
    let mut parser = Parser::default();
    let layers = parser.parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1, "expected a single layer");
    let tag = match &layers[0] {
        Layer::Tag01(_) => 1,
        Layer::Tag02(_) => 2,
        _ => panic!("unexpected layer kind"),
    };
    assert_eq!(tag, expected_tag);
    let layer = layers
        .into_iter()
        .next()
        .unwrap()
        .into_layer01()
        .expect("layer01 representation");
    layer.into_tile(&mut Decoder::default()).expect("into_tile")
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
    assert_eq!(cursor, bytes.len(), "dump does not cover the whole tile");
}

fn differential_fixture([path]: [&Path; 1]) {
    let Ok(layers) = mvt_to_tile_layers(fs::read(path).expect("read fixture")) else {
        return;
    };
    for layer in layers {
        let v2_bytes = match layer.clone().encode(cfg(WireVersion::V02)) {
            Ok(bytes) => bytes,
            // Strings, shared dictionaries and tessellation have no v2 encoder yet.
            Err(MltError::NotImplemented(_)) => continue,
            Err(e) => panic!("{}: v2 encode failed: {e}", path.display()),
        };
        let v1_bytes = layer.encode(cfg(WireVersion::V01)).expect("v1 encode");
        assert_eq!(
            decode(&v1_bytes, 1),
            decode(&v2_bytes, 2),
            "{}: v1 and v2 decoded layers must be identical",
            path.display()
        );
        assert_dump_covers(&v2_bytes);
    }
}

#[rstest]
#[case::amazon_5_5_11("amazon/5_5_11.mvt", 15)]
#[case::amazon_11_1037_704("amazon/11_1037_704.mvt", 15)]
#[case::amazon_5_8_12("amazon/5_8_12.mvt", 15)]
fn named_fixtures_still_reach_the_v2_encoder(#[case] fixture: &str, #[case] expected: usize) {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../test/fixtures"));
    let layers = mvt_to_tile_layers(fs::read(root.join(fixture)).expect("read fixture"))
        .expect("decode fixture");
    let encodable = layers
        .into_iter()
        .filter(|l| l.clone().encode(cfg(WireVersion::V02)).is_ok())
        .count();
    assert_eq!(encodable, expected);
}
