#![cfg(all(feature = "reader", feature = "writer"))]

use std::fs;
use std::path::Path;

use fast_mvt::{
    DEFAULT_EXTENT, MvtFeature, MvtLayer, MvtReaderRef, MvtResult, MvtTile, encode_to_vec,
};
use geo_types::{Geometry, LineString, Polygon};
use test_each_file::test_each_path;

test_each_path! { for ["mvt"] in "../test/fixtures" as test_fixtures => round_trip_file }
test_each_path! { for ["mvt"] in "../test/mvt-fixtures/real-world" as mvt_test_fixtures => round_trip_file }

fn round_trip_file([path]: [&Path; 1]) {
    let data = fs::read(path).expect("read MVT file");
    let original = MvtReaderRef::new(&data)
        .and_then(|v| v.to_tile())
        .unwrap_or_else(|e| panic!("{}: decode failed: {e}", path.display()));
    let normalized = try_re_encode(&original)
        .unwrap_or_else(|e| panic!("{}: round-trip failed: {e}", path.display()));
    let again = try_re_encode(&normalized).expect("second re-encode");
    assert_eq!(normalized, again);
}

fn try_re_encode(tile: &MvtTile) -> MvtResult<MvtTile> {
    let bytes = encode_to_vec(tile)?;
    MvtReaderRef::new(&bytes).and_then(|v| v.to_tile())
}

#[test]
fn empty_tile_round_trips() {
    let tile = MvtTile::default();
    let bytes = encode_to_vec(&tile).unwrap();
    let decoded = MvtReaderRef::new(&bytes).unwrap().to_tile().unwrap();
    assert!(decoded.layers.is_empty());
}

#[test]
fn owned_builder_api_encodes_like_mvt_crate_surface() {
    let mut tile = MvtTile::new();
    let mut layer = MvtLayer::new("layer", DEFAULT_EXTENT);
    let mut feature = MvtFeature::new(Geometry::Point((1, 2).into()));
    feature.set_id(7);
    feature.add_tag_string("name", "place");
    feature.add_tag_double("score", 1.5);
    feature.add_tag_float("rank", 2.5);
    feature.add_tag_int("i", -3);
    feature.add_tag_uint("u", 4);
    feature.add_tag_sint("s", -5);
    feature.add_tag_bool("visible", true);
    assert_eq!(feature.num_tags(), 7);
    layer.add_feature(feature);
    assert_eq!(layer.name(), "layer");
    assert_eq!(layer.num_features(), 1);

    tile.add_layer(layer).unwrap();
    assert_eq!(tile.num_layers(), 1);
    assert!(matches!(
        tile.add_layer(MvtLayer::new("layer", DEFAULT_EXTENT)),
        Err(fast_mvt::MvtError::DuplicateLayer(name)) if name == "layer"
    ));

    let bytes = tile.to_bytes().unwrap();
    assert_eq!(tile.compute_size().unwrap(), bytes.len());
    let mut out = Vec::new();
    tile.write_to(&mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn ring_is_implicitly_closed() {
    let tile = MvtTile {
        layers: vec![MvtLayer {
            name: "layer".to_string(),
            extent: DEFAULT_EXTENT,
            features: vec![MvtFeature {
                id: Some(1),
                geometry: Geometry::Polygon(Polygon::new(
                    LineString(vec![
                        (0, 0).into(),
                        (10, 0).into(),
                        (10, 10).into(),
                        (0, 10).into(),
                        (0, 0).into(),
                    ]),
                    vec![],
                )),
                properties: Vec::new(),
            }],
        }],
    };
    let decoded = try_re_encode(&tile).unwrap();
    let Geometry::Polygon(poly) = &decoded.layers[0].features[0].geometry else {
        panic!("expected polygon");
    };
    assert_eq!(poly.exterior().0.len(), 5);
    assert_eq!(poly.exterior().0.first(), poly.exterior().0.last());
}
