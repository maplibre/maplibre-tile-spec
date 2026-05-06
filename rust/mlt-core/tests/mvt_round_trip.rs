//! Round-trip every MVT fixture in `test/fixtures/**/*.mvt`. The first
//! encode normalizes spec-permissible quirks (consecutive duplicate vertices,
//! axis-aligned collinear polygon points); subsequent re-encodes must be a
//! fixpoint, so we compare the once- and twice-normalized layers.

use std::fs;
use std::path::Path;

use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::test_helpers::assert_mvt_equivalent_layers;
use test_each_file::test_each_path;

/// Fixtures the upstream `mvt-reader` cannot decode (MVT spec v3). Listed
/// explicitly so a regression that breaks *more* fixtures still trips the
/// test instead of silently passing.
const UNSUPPORTED_PARENT_DIRS: &[&str] = &["bing"];

test_each_path! { for ["mvt"] in "../test/fixtures" as mvt_round_trip => round_trip_fixture }

fn round_trip_fixture([path]: [&Path; 1]) {
    let mvt_bytes = fs::read(path).expect("read fixture");
    let parent_name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let expected_unsupported = UNSUPPORTED_PARENT_DIRS.contains(&parent_name);

    let original = match mvt_to_tile_layers(mvt_bytes) {
        Ok(layers) => {
            assert!(
                !expected_unsupported,
                "{} unexpectedly decoded — remove its parent dir from \
                 UNSUPPORTED_PARENT_DIRS so it gets full round-trip coverage",
                path.display()
            );
            layers
        }
        Err(e) => {
            assert!(
                expected_unsupported,
                "{}: unexpected decode failure: {e}",
                path.display()
            );
            return;
        }
    };
    let normalized = re_encode(original);
    let again = re_encode(normalized.clone());

    assert_eq!(normalized.len(), again.len(), "layer count");
    for (a, b) in normalized.iter().zip(again.iter()) {
        assert_mvt_equivalent_layers(a, b);
    }
}

fn re_encode(layers: Vec<mlt_core::TileLayer>) -> Vec<mlt_core::TileLayer> {
    let bytes = tile_layers_to_mvt(layers).expect("encode mvt");
    mvt_to_tile_layers(bytes).expect("decode re-encoded mvt")
}
