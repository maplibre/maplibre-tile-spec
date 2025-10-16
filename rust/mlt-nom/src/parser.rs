use std::borrow::Cow;
use std::io;
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::utils::take;
use crate::v0x01::RawFeatureTable;
use crate::{MltError, MltRefResult, utils};

/// A layer that can be either MVT-compatible or unknown
#[borrowme]
#[derive(Debug, PartialEq)]
#[expect(clippy::large_enum_variant)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Tag01(RawFeatureTable<'a>),
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
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            // For now, we only support tag 0x01 layers, but more will be added soon
            1 => Layer::Tag01(RawFeatureTable::parse(value)?),
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }
}

impl OwnedLayer {
    /// Write Layer's binary representation to a Write stream without allocating a Vec
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let (tag, buffer) = match self {
            OwnedLayer::Tag01(layer) => {
                let mut buffer = Vec::new();
                layer.write_to(&mut buffer)?;
                (1_u8, Cow::Owned(buffer))
            }
            OwnedLayer::Unknown(unknown) => (unknown.tag, Cow::Borrowed(&unknown.value)),
        };

        let size = buffer.len().checked_add(1);
        let size = size.ok_or(io::Error::other(MltError::IntegerOverflow))?;
        let size = u64::try_from(size);
        let size = size.map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        writer.write_varint(size)?;

        writer.write_all(&[tag])?;
        writer.write_all(&buffer)
    }
}

/// Unknown layer data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Unknown<'a> {
    pub tag: u8,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub value: &'a [u8],
}

impl Unknown<'_> {
    /// Write Unknown's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.value)
    }
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
