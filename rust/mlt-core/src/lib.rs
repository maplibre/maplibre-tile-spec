#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "fastpfor-cpp", feature = "fastpfor-rust")))]
compile_error!("one of `fastpfor-cpp` or `fastpfor-rust` must be enabled");

mod analyse;
mod convert;
pub mod decoder;
mod enc_dec;
mod errors;
pub mod frames;
pub mod optimizer;
#[doc(hidden)]
pub mod utils;

pub use analyse::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decoder::Decoder;
pub use enc_dec::{Decode, EncDec};
pub use errors::{MltError, MltRefResult};
pub use frames::{EncodedLayer, Layer, StagedLayer, unknown, v01};

/// Parse a sequence of binary layers
pub fn parse_layers(mut input: &[u8]) -> Result<Vec<Layer<'_>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = Layer::from_bytes(input)?;
        result.push(layer);
    }
    Ok(result)
}
