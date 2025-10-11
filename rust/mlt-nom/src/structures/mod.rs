// use borrowme::borrowme;

use crate::MltError::Fail;
use crate::structures::v1::FeatureTable;
use crate::utils::take;
use crate::{MltError, MltRefResult, utils};

pub mod complex_enums;
pub mod enums;
pub mod v1;

/// A layer that can be either MVT-compatible or unknown
//#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Layer(FeatureTable<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

impl Layer<'_> {
    /// Parse a single binary tuple: size (varint), tag (varint), value (bytes)
    pub fn parse(input: &[u8]) -> MltRefResult<'_, Layer<'_>> {
        let (input, size) = utils::parse_varint::<usize>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = utils::parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(Fail)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            // For now, we only support tag 0x01 layers, but more will be added soon
            1 => Layer::Layer(FeatureTable::parse(value)?),
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }
}

/// Unknown layer data
//#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    // #[borrowme(borrow_with = Vec::as_slice)]
    pub value: &'a [u8],
}

/// Parse a sequence of binary layers
pub fn parse_binary_stream(mut input: &[u8]) -> Result<Vec<Layer<'_>>, MltError> {
    let mut result = Vec::new();
    while !input.is_empty() {
        let layer;
        (input, layer) = Layer::parse(input)?;
        result.push(layer);
    }
    Ok(result)
}
