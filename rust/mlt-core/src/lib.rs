#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "fastpfor-cpp", feature = "fastpfor-rust")))]
compile_error!("one of `fastpfor-cpp` or `fastpfor-rust` must be enabled");

mod analyse;
mod convert;
mod decode;
mod enc_dec;
mod encode;
mod errors;
pub mod frames;
pub mod optimizer;
#[doc(hidden)]
pub mod utils;

pub use analyse::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decode::Decode;
pub(crate) use decode::{Decodable, DecodeInto};
pub use enc_dec::EncDec;
pub use encode::Encodable;
pub(crate) use encode::FromDecoded;
pub use errors::{MltError, MltRefResult};
pub use frames::{EncodedLayer, Layer, unknown, v01};

/// Parse a sequence of binary layers
pub fn parse_layers(mut input: &[u8]) -> Result<Vec<Layer<'_>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = Layer::parse(input)?;
        result.push(layer);
    }
    Ok(result)
}
