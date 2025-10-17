use std::borrow::Cow;
use std::io;
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::decodable::{FromRaw, impl_decodable};
use crate::unknown::{OwnedUnknown, Unknown};
use crate::utils::take;
use crate::v01::{DecodedLayer01, Layer01, OwnedLayer01, RawLayer01};
use crate::{MltError, MltRefResult, utils};

#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Layer<'a> {
    Raw(RawLayer<'a>),
    Decoded(DecodedLayer),
}

/// A layer that can be one of the known types, or an unknown
#[borrowme]
#[derive(Debug, PartialEq)]
#[expect(clippy::large_enum_variant)]
pub enum RawLayer<'a> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

#[derive(Debug, Clone, PartialEq)]
#[expect(clippy::large_enum_variant)]
pub enum DecodedLayer {
    /// MVT-compatible layer (tag = 1)
    Tag01(DecodedLayer01),
    /// Unknown layer with tag, size, and value
    Unknown(OwnedUnknown), // Unknown data is not decoded, so same as owned-raw
}

impl_decodable!(Layer<'a>, RawLayer<'a>, DecodedLayer);

impl<'a> From<RawLayer<'a>> for Layer<'a> {
    fn from(value: RawLayer<'a>) -> Self {
        Self::Raw(value)
    }
}

impl RawLayer<'_> {
    /// Parse a single tuple that consists of `size (varint)`, `tag (varint)`, and `value (bytes)`
    pub fn parse(input: &[u8]) -> MltRefResult<'_, RawLayer<'_>> {
        let (input, size) = utils::parse_varint::<usize>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = utils::parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            // For now, we only support tag 0x01 layers, but more will be added soon
            1 => RawLayer::Tag01(RawLayer01::parse(value)?.into()),
            tag => RawLayer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }
}

impl Default for DecodedLayer {
    fn default() -> Self {
        Self::Unknown(OwnedUnknown::default())
    }
}

impl OwnedRawLayer {
    /// Write layer's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let (tag, buffer) = match self {
            OwnedRawLayer::Tag01(layer) => match layer {
                OwnedLayer01::Raw(layer) => {
                    let mut buffer = Vec::new();
                    layer.write_to(&mut buffer)?;
                    (1_u8, Cow::Owned(buffer))
                }
                OwnedLayer01::Decoded(_) => {
                    todo!("need to encode DecodedLayer01 to OwnedRawLayer01 first")
                }
            },
            OwnedRawLayer::Unknown(unknown) => (unknown.tag, Cow::Borrowed(&unknown.value)),
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

impl<'a> FromRaw<'a> for DecodedLayer {
    type Input = RawLayer<'a>;

    fn from_raw(value: RawLayer<'_>) -> Result<Self, MltError> {
        match value {
            RawLayer::Tag01(v) => Ok(DecodedLayer::Tag01(v.decode()?)),
            RawLayer::Unknown(v) => Ok(DecodedLayer::Unknown(borrowme::ToOwned::to_owned(&v))),
        }
    }
}
