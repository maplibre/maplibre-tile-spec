#![doc = include_str!("../README.md")]

pub(crate) mod analyse;
pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub(crate) mod enc_dec;
pub(crate) mod errors;
pub(crate) mod frames;
pub(crate) mod utils;

pub use analyse::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decoder::{Decoder, Parser};
pub use enc_dec::{Decode, DecodeState, Decoded, EncDec, Mixed};
pub use errors::{MltError, MltRefResult, MltResult};
pub use frames::{EncodedLayer, Layer, StagedLayer, unknown, v01};

#[cfg(any(test, feature = "__private"))]
mod test_helpers;

/// Private re-exports for benchmarks and integration tests. Not part of the public API.
#[cfg(any(test, feature = "__private"))]
#[doc(hidden)]
pub mod __private {
    pub use crate::codecs::{bytes, fastpfor, hilbert, morton, rle, zigzag};
    pub use crate::test_helpers::*;
}
