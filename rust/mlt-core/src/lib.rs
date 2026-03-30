#![doc = include_str!("../README.md")]
extern crate core;

pub(crate) mod analyze;
pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub(crate) mod errors;
pub(crate) mod frames;
pub(crate) mod lazy_state;
pub(crate) mod utils;

pub use analyze::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decoder::{Decoder, Parser};
pub use errors::{MltError, MltRefResult, MltResult};
pub use frames::{EncodedLayer, Layer, ParsedLayer, StagedLayer, unknown, v01};
pub use lazy_state::{Decode, DecodeState, Lazy, LazyParsed, Parsed};

#[cfg(any(test, feature = "__private"))]
pub mod test_helpers;

/// Private re-exports for benchmarks and integration tests. Not part of the public API.
#[cfg(any(test, feature = "__private"))]
#[doc(hidden)]
pub mod __private {
    pub use crate::codecs::{bytes, fastpfor, hilbert, morton, rle, zigzag};
    pub use crate::test_helpers::*;
}
