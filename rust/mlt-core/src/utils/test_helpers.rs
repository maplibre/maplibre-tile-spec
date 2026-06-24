//! Shared helpers for unit tests, integration tests, and benchmarks.

use std::collections::{BTreeMap, BTreeSet};

use crate::decoder::Layer01;
use crate::{Decoder, Layer, MltRefResult, Parser, PropValue, TileLayer};

/// Default decoder for decoding in tests.
#[must_use]
pub fn dec() -> Decoder {
    Decoder::default()
}

/// Default parser for parsing in tests.
#[must_use]
pub fn parser() -> Parser {
    Parser::default()
}

pub fn assert_empty<T>(result: MltRefResult<T>) -> T {
    let (remaining, value) = result.unwrap();
    assert!(remaining.is_empty(), "{} bytes remain", remaining.len());
    value
}

#[must_use]
pub fn into_layer01(layer: Layer) -> Layer01 {
    match layer {
        Layer::Tag01(layer01) => layer01,
        Layer::Tag02(_) => panic!("expected Tag01 layer, got Tag02"),
        Layer::Unknown(_) => panic!("expected Tag01 layer, got Unknown"),
    }
}

/// Map a feature's properties by name. Property column order in [`TileLayer`]
/// can change after MVT normalization, so callers comparing two layers must
/// compare per-feature maps rather than parallel `Vec`s.
#[must_use]
pub fn feature_property_map(layer: &TileLayer, feat_idx: usize) -> BTreeMap<&str, &PropValue> {
    layer
        .property_names()
        .iter()
        .map(String::as_str)
        .zip(layer.features()[feat_idx].properties().iter())
        .collect()
}

/// Assert two layers are semantically equivalent after an MVT round-trip:
/// same name, extent, feature count, ids, geometries, and per-feature property
/// maps (compared by name, not column index).
pub fn assert_mvt_equivalent_layers(a: &TileLayer, b: &TileLayer) {
    assert_eq!(a.name(), b.name(), "layer name");
    assert_eq!(a.extent(), b.extent(), "layer extent");
    let names_a: BTreeSet<&str> = a.property_names().iter().map(String::as_str).collect();
    let names_b: BTreeSet<&str> = b.property_names().iter().map(String::as_str).collect();
    assert_eq!(names_a, names_b, "property name set");
    assert_eq!(a.features().len(), b.features().len(), "feature count");
    for (i, (af, bf)) in a.features().iter().zip(b.features().iter()).enumerate() {
        assert_eq!(af.id(), bf.id(), "feature id (index {i})");
        assert_eq!(af.geometry(), bf.geometry(), "feature geometry (index {i})");
        assert_eq!(
            feature_property_map(a, i),
            feature_property_map(b, i),
            "feature properties (index {i})"
        );
    }
}
