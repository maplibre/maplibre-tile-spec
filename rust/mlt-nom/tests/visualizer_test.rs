//! Tests for the visualizer module

use std::fs;

use mlt_nom::parse_layers;

#[test]
#[cfg(feature = "cli")]
fn test_visualizer_can_parse_simple_files() {
    // Test that we can successfully decode files that the visualizer will work with
    let test_files = [
        "test/expected/tag0x01/simple/point-boolean.mlt",
        "test/expected/tag0x01/simple/line-boolean.mlt",
        "test/expected/tag0x01/simple/polygon-boolean.mlt",
    ];

    for file in &test_files {
        let path = format!("../../{file}");
        let buffer = fs::read(&path).expect("Failed to read test file");
        let mut layers = parse_layers(&buffer).expect("Failed to parse layers");

        // Decode all layers
        for layer in &mut layers {
            layer.decode_all().expect("Failed to decode layer");
        }

        // Verify we have at least one layer
        assert!(!layers.is_empty(), "Expected at least one layer in {file}");

        // Verify the layer can be accessed
        let layer01 = layers[0].as_layer01().expect("Expected Layer01");

        // Verify the geometry is decoded
        if let mlt_nom::v01::Geometry::Decoded(geom) = &layer01.geometry {
            assert!(
                !geom.vector_types.is_empty(),
                "Expected at least one geometry"
            );
        } else {
            panic!("Geometry should be decoded");
        }
    }
}
