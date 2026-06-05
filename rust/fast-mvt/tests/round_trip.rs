#![cfg(all(feature = "reader", feature = "writer"))]

use std::fs;
use std::path::Path;

use fast_mvt::{
    DEFAULT_EXTENT, MvtFeature, MvtLayer, MvtReaderRef, MvtResult, MvtTile, MvtTileBuilder,
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
    let bytes = tile.clone().encode()?;
    MvtReaderRef::new(&bytes).and_then(|v| v.to_tile())
}

#[test]
fn empty_tile_round_trips() {
    let bytes = MvtTileBuilder::new().finish();
    let decoded = MvtReaderRef::new(&bytes).unwrap().to_tile().unwrap();
    assert!(decoded.layers.is_empty());
}

#[test]
fn owned_builder_api_encodes_like_mvt_crate_surface() {
    let tile = MvtTileBuilder::new();
    let layer = tile.layer("layer");
    let mut feature = layer.feature(Geometry::Point((1, 2).into())).unwrap();
    feature.id(7);
    feature.tag_string("name", "place").unwrap();
    feature.tag_double("score", 1.5).unwrap();
    feature.tag_float("rank", 2.5).unwrap();
    feature.tag_int("i", -3).unwrap();
    feature.tag_uint("u", 4).unwrap();
    feature.tag_sint("s", -5).unwrap();
    feature.tag_bool("visible", true).unwrap();
    assert_eq!(feature.num_tags(), 7);
    let layer = feature.finish();
    assert_eq!(layer.name(), "layer");
    assert_eq!(layer.num_features(), 1);

    let tile = layer.finish();
    let tile = tile.layer("layer").finish();
    assert!(!tile.finish().is_empty());

    let bytes = MvtTileBuilder::new()
        .layer("layer")
        .feature(Geometry::Point((1, 2).into()))
        .unwrap()
        .finish()
        .finish()
        .finish();
    assert!(!bytes.is_empty());
}

#[test]
fn independently_built_layer_fields_concatenate_into_tile() {
    let roads = MvtLayer::new("roads", DEFAULT_EXTENT);
    let pois = MvtLayer::new("pois", DEFAULT_EXTENT);
    let roads_bytes = MvtTile {
        layers: vec![roads],
    }
    .encode()
    .unwrap();
    let pois_bytes = MvtTile { layers: vec![pois] }.encode().unwrap();

    let mut out = Vec::new();
    out.extend_from_slice(&roads_bytes);
    out.extend_from_slice(&pois_bytes);
    let decoded = MvtReaderRef::new(&out).unwrap().to_tile().unwrap();
    assert_eq!(decoded.layers.len(), 2);
    assert_eq!(decoded.layers[0].name, "roads");
    assert_eq!(decoded.layers[1].name, "pois");
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
