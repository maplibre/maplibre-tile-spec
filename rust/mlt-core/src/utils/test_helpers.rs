//! Shared helpers for unit tests, integration tests, and benchmarks.

use crate::decoder::Layer01;
use crate::{Decoder, Layer, MltRefResult, Parser};

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
