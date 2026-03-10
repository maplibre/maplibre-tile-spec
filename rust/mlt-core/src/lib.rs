#![expect(dead_code)]
#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "fastpfor-cpp", feature = "fastpfor-rust")))]
compile_error!("one of `fastpfor-cpp` or `fastpfor-rust` must be enabled");

mod analyse;
mod convert;
mod decode;
mod encode;
mod errors;
pub mod frames;
pub mod optimizer;
#[doc(hidden)]
pub mod utils;

pub use analyse::{Analyze, StatType};
// reexport borrowme to make it easier to use in other crates
pub use borrowme;
pub use convert::{geojson, mvt};
pub use decode::*;
pub use encode::*;
pub use errors::{MltError, MltRefResult};
pub use frames::{Layer, OwnedLayer, unknown, v01};

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
