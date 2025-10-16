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

use crate::layer::{Layer, RawLayer};

/// Parse a sequence of binary layers
pub fn parse_binary_stream(mut input: &[u8]) -> Result<Vec<Layer<'_>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = RawLayer::parse(input)?;
        result.push(layer.into());
    }
    Ok(result)
}
