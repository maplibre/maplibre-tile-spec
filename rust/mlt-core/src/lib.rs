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
pub use decoder::{Decoder, MemBudget};
pub use enc_dec::{Decode, EncDec};
pub use errors::{MltError, MltRefResult};
pub use frames::{EncodedLayer, Layer, StagedLayer, unknown, v01};

/// Parse a sequence of binary layers, reserving decoded memory against `budget`.
pub fn parse_layers<'a>(
    mut input: &'a [u8],
    budget: &mut MemBudget,
) -> Result<Vec<Layer<'a>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = Layer::from_bytes(input, budget)?;
        result.push(layer);
    }
    Ok(result)
}
