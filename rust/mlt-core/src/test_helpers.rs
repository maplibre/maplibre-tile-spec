//! Shared test helpers for unit and integration tests.
//!
//! Use from unit tests: `use crate::test_helpers::{dec, parser};`
//! Use from integration tests: `use mlt_core::test_helpers::{dec, parser};`

use crate::{Decoder, Parser};

/// Default decoder for decoding in tests.
#[inline]
#[must_use]
pub fn dec() -> Decoder {
    Decoder::default()
}

/// Default parser for parsing in tests.
#[inline]
#[must_use]
pub fn parser() -> Parser {
    Parser::default()
}
