#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "fastpfor-cpp", feature = "fastpfor-rust")))]
compile_error!("one of `fastpfor-cpp` or `fastpfor-rust` must be enabled");

pub mod decoder;
mod analyse;
mod convert;
mod enc_dec;
mod errors;
pub mod frames;
pub mod optimizer;
#[doc(hidden)]
pub mod utils;

pub use decoder::{Decoder, Parser};
pub use analyse::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use enc_dec::{Decode, EncDec};
pub use errors::{MltError, MltRefResult};
pub use frames::{EncodedLayer, Layer, StagedLayer, unknown, v01};

/// Helpers for tests and benchmarks (e.g. default `parser` and `dec`).
#[doc(hidden)]
pub mod test_helpers;

/// Parse a sequence of binary layers, reserving decoded memory against the parser's budget.
pub fn parse_layers<'a>(
    mut input: &'a [u8],
    parser: &mut Parser,
) -> Result<Vec<Layer<'a>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = Layer::from_bytes(input, parser)?;
        result.push(layer);
    }
    Ok(result)
}
