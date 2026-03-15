//! Shared helpers for unit tests, integration tests, and benchmarks.
//!
//! Always compiled but `#[doc(hidden)]` so it stays out of the main API.
//!
//! - Unit tests: `use crate::test_helpers::{dec, parser};`
//! - Integration tests: `use mlt_core::test_helpers::{dec, parser};`
//! - Benchmarks: `use mlt_core::test_helpers::{dec, parser};`

use crate::v01::Layer01;
use crate::{Decoder, Layer, Parser};

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

pub fn assert_empty(remaining: &[u8]) {
    assert!(remaining.is_empty(), "{} bytes remain", remaining.len());
}

#[must_use]
pub fn into_layer01(layer: Layer) -> Layer01 {
    match layer {
        Layer::Tag01(layer01) => layer01,
        Layer::Unknown(_) => panic!("expected Tag01 layer"),
    }
}
