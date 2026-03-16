use std::fs;
use std::path::Path;

use mlt_core::geojson::FeatureCollection;
use mlt_core::test_helpers::{dec, parser};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/expected/tag0x01" as geojson => geojson_test }

fn geojson_test([mlt]: [&Path; 1]) {
    let buffer = fs::read(mlt).unwrap();
    let mut p = parser();
    let mut layers = p.parse_layers(&buffer).unwrap();
    assert!(p.reserved() > 0);

    let mut d = dec();
    let fc = FeatureCollection::from_layers(&mut layers, &mut d).unwrap();
    assert!(d.consumed() > 0);
    assert!(!fc.features.is_empty(), "expected at least one feature");
}
