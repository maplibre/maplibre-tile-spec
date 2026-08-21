//! Shared helpers for unit tests, integration tests, and benchmarks.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use crate::decoder::Layer01;
use crate::{Decoder, Layer, MltRefResult, Parser, PropValue, TileLayer};

/// Assert that `len`/`size_hint` equal the number of items actually left, at every
/// position reachable by consuming from the front, the back, or both alternately.
///
/// `make` must produce a fresh iterator over `expected` on each call.
pub fn assert_size_hint_exact<I, T>(make: impl Fn() -> I, expected: &[T])
where
    I: ExactSizeIterator<Item = T> + DoubleEndedIterator,
    T: PartialEq + Debug,
{
    fn check<I: ExactSizeIterator>(iter: &I, left: usize) {
        assert_eq!(iter.len(), left, "len");
        assert_eq!(iter.size_hint(), (left, Some(left)), "size_hint");
    }

    let count = expected.len();

    for taken in 0..=count {
        let mut iter = make();
        for _ in 0..taken {
            iter.next().expect("front item");
        }
        check(&iter, count - taken);
        assert_eq!(iter.collect::<Vec<_>>().as_slice(), &expected[taken..]);
    }

    for taken in 0..=count {
        let mut iter = make();
        for _ in 0..taken {
            iter.next_back().expect("back item");
        }
        check(&iter, count - taken);
        assert_eq!(
            iter.collect::<Vec<_>>().as_slice(),
            &expected[..count - taken]
        );
    }

    let mut iter = make();
    let mut left = count;
    check(&iter, left);
    while left > 0 {
        iter.next().expect("front item");
        left -= 1;
        check(&iter, left);
        if left > 0 {
            iter.next_back().expect("back item");
            left -= 1;
            check(&iter, left);
        }
    }
    assert!(iter.next().is_none());
    assert!(iter.next_back().is_none());
    check(&iter, 0);
}

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
        Layer::Tag01(v) => v,
        #[cfg(feature = "unstable-v2")]
        Layer::Tag02(v) => v,
        Layer::Unknown(v) => panic!("expected Tag01/02 layer, got Tag{:02x}", v.tag),
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
