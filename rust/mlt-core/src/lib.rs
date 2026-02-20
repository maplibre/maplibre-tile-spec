#![expect(dead_code)]
#![expect(unused_assignments)]
#![expect(unused_variables)]
#![doc = include_str!("../README.md")]

mod analyse;
mod convert;
mod decode;
mod encode;
mod errors;
pub use convert::{geojson, mvt};
pub mod layer;
pub use layer::{unknown, v01};
mod utils;

pub use analyse::{Analyze, StatType};
pub use decode::*;
pub use encode::*;
pub use errors::{MltError, MltRefResult};
pub use layer::{Layer, OwnedLayer};

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
