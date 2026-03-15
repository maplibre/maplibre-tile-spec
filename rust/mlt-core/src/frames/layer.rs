use std::borrow::Cow;
use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter as _;
use utils::BinarySerializer as _;

use crate::frames::Unknown;
use crate::frames::v01::Layer01;
use crate::utils::{checked_sum2, parse_u8, parse_varint, take};
use crate::{Decoder, EncodedLayer, Layer, MemBudget, MltError, MltRefResult, utils};

impl<'a> Layer<'a> {
    /// Returns the inner `Layer01` if this is a Tag01 layer, or `None` otherwise.
    #[must_use]
    pub fn as_layer01(&self) -> Option<&Layer01<'a>> {
        match self {
            Layer::Tag01(l) => Some(l),
            Layer::Unknown(_) => None,
        }
    }

    /// Returns the inner `Layer01` if this is a Tag01 layer, or `None` otherwise.
    #[must_use]
    pub fn as_layer01_mut(&mut self) -> Option<&mut Layer01<'a>> {
        match self {
            Layer::Tag01(l) => Some(l),
            Layer::Unknown(_) => None,
        }
    }

    pub fn decoded_layer01_mut(&mut self, dec: &mut Decoder) -> Result<&mut Layer01<'a>, MltError> {
        let layer = self
            .as_layer01_mut()
            .ok_or(MltError::NotDecoded("expected Tag01 layer"))?;
        layer.decode_id(dec)?;
        layer.decode_geometry(dec)?;
        Ok(layer)
    }

    /// Parse a single tuple that consists of `size (varint)`, `tag (varint)`, and `value (bytes)`.
    /// Reserves memory for decoded data against `budget`.
    pub fn from_bytes(input: &'a [u8], budget: &mut MemBudget) -> MltRefResult<'a, Layer<'a>> {
        let (input, size) = parse_varint::<u32>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            // For now, we only support tag 0x01 layers, but more will be added soon
            1 => Layer::Tag01(Layer01::from_bytes(value, budget)?),
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }

    pub fn decode_all(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        match self {
            Layer::Tag01(layer) => layer.decode_all(dec),
            Layer::Unknown(_) => Ok(()),
        }
    }
}

impl EncodedLayer {
    /// Write layer's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let (tag, buffer) = match self {
            EncodedLayer::Tag01(layer) => {
                let mut buffer = Vec::new();
                layer.write_to(&mut buffer)?;
                (1_u8, Cow::Owned(buffer))
            }
            EncodedLayer::Unknown(unknown) => (unknown.tag, Cow::Borrowed(&unknown.value)),
        };

        let buffer_len = u32::try_from(buffer.len()).map_err(MltError::from)?;
        let size = checked_sum2(buffer_len, 1)?;
        writer.write_varint(size)?;
        writer.write_u8(tag)?;
        writer.write_all(&buffer)
    }
}
