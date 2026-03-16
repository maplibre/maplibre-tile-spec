#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "fastpfor-cpp", feature = "fastpfor-rust")))]
compile_error!("one of `fastpfor-cpp` or `fastpfor-rust` must be enabled");

pub(crate) mod analyse;
pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub(crate) mod enc_dec;
pub(crate) mod errors;
pub(crate) mod frames;
pub(crate) mod optimizer;
#[doc(hidden)]
pub(crate) mod utils;

pub use analyse::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decoder::{Decoder, Parser};
pub use enc_dec::{Decode, EncDec};
pub use errors::{MltError, MltRefResult};
pub use frames::{EncodedLayer, Layer, StagedLayer, unknown, v01};

/// Helpers for tests and benchmarks (e.g. default `parser` and `dec`).
#[doc(hidden)]
pub(crate) mod test_helpers;

// re-export publicly for benchmarks
#[cfg(test)]
pub use crate::codecs::morton::{decode_morton_codes, decode_morton_delta};
