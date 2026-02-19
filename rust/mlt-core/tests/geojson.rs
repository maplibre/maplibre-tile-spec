use std::fs;
use std::path::Path;

use mlt_core::geojson::FeatureCollection;
use mlt_core::parse_layers;
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/expected/tag0x01" as geojson => geojson_test }

fn geojson_test([mlt]: [&Path; 1]) {
    let buffer = fs::read(mlt).unwrap();
    let mut layers = parse_layers(&buffer).unwrap();
    for layer in &mut layers {
        layer.decode_all().unwrap();
    }
    let fc = FeatureCollection::from_layers(&layers).unwrap();
    assert!(!fc.features.is_empty(), "expected at least one feature");
}
