use std::borrow::Cow;
use std::io;
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;
use utils::BinarySerializer as _;

use crate::unknown::Unknown;
use crate::utils::take;
use crate::v01::Layer01;
use crate::{MltError, MltRefResult, utils};

/// A layer that can be one of the known types, or an unknown
#[borrowme]
#[derive(Debug, PartialEq)]
#[expect(clippy::large_enum_variant)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

impl<'a> Layer<'a> {
    /// Returns the inner `Layer01` if this is a Tag01 layer, or `None` otherwise.
    #[must_use]
    pub fn as_layer01(&self) -> Option<&Layer01<'a>> {
        match self {
            Layer::Tag01(l) => Some(l),
            Layer::Unknown(_) => None,
        }
    }

    /// Parse a single tuple that consists of `size (varint)`, `tag (varint)`, and `value (bytes)`
    pub fn parse(input: &'a [u8]) -> MltRefResult<'a, Layer<'a>> {
        let (input, size) = utils::parse_varint::<usize>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = utils::parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            // For now, we only support tag 0x01 layers, but more will be added soon
            1 => Layer::Tag01(Layer01::parse(value)?),
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }

    pub fn decode_all(&mut self) -> Result<(), MltError> {
        match self {
            Layer::Tag01(layer) => layer.decode_all(),
            Layer::Unknown(_) => Ok(()),
        }
    }
}

impl OwnedLayer {
    /// Write layer's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // (size, tag, data) => need to encode to a buffer to write the size
        let (tag, buffer) = match self {
            OwnedLayer::Tag01(layer) => {
                let mut buffer = Vec::new();
                layer.write_to(&mut buffer)?;
                (1_u8, Cow::Owned(buffer))
            }
            OwnedLayer::Unknown(unknown) => (unknown.tag, Cow::Borrowed(&unknown.value)),
        };

        let size = buffer
            .len()
            .checked_add(1)
            .ok_or(io::Error::other(MltError::IntegerOverflow))?;
        let size = u64::try_from(size).map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        writer.write_varint(size)?;
        writer.write_u8(tag)?;
        writer.write_all(&buffer)
    }
}
