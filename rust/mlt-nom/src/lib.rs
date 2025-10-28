#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

mod decodable;
mod errors;
mod layer;
mod unknown;
mod utils;
pub mod v01;

pub use decodable::*;
pub use errors::{MltError, MltRefResult};

use crate::layer::Layer;

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
