use crate::codecs::varint::parse_varint;
use crate::decoder::{Layer01, Unknown};
use crate::utils::{parse_u8, take};
use crate::v2::decode_v2_layer;
use crate::{DecodeState, Decoder, Layer, MltError, MltRefResult, MltResult, ParsedLayer, Parser};

impl<'a, S: DecodeState> Layer<'a, S> {
    /// Returns the inner `Layer01` if this is a Tag01 layer, or `None` otherwise.
    #[must_use]
    pub fn as_layer01(&self) -> Option<&Layer01<'a, S>> {
        match self {
            Self::Tag01(l) => Some(l),
            Self::Tag02(_) | Self::Unknown(_) => None,
        }
    }

    /// Consumes this layer and returns the inner `Layer01`, or `None` if it is not a Tag01 layer.
    #[must_use]
    pub fn into_layer01(self) -> Option<Layer01<'a, S>> {
        match self {
            Self::Tag01(l) => Some(l),
            Self::Tag02(_) | Self::Unknown(_) => None,
        }
    }
}

impl<'a> Layer<'a> {
    /// Parse a single tuple that consists of `size (varint)`, `tag (varint)`, and `value (bytes)`.
    /// Reserves memory for decoded data against the parser's budget.
    pub(crate) fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
        let (input, size) = parse_varint::<u32>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            1 => Layer::Tag01(Layer01::from_bytes(value, parser)?),
            2 => Layer::Tag02(decode_v2_layer(value)?),
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
    }

    /// Decode all columns and return a fully-decoded [`ParsedLayer`].
    ///
    /// Consumes `self`.  For partial / incremental decoding, destructure with
    /// `Layer::Tag01(lazy)` and call the individual methods on [`Layer01`].
    pub fn decode_all(self, dec: &mut Decoder) -> MltResult<ParsedLayer<'a>> {
        match self {
            Layer::Tag01(lazy) => Ok(Layer::Tag01(lazy.decode_all(dec)?)),
            Layer::Tag02(t) => Ok(Layer::Tag02(t)),
            Layer::Unknown(u) => Ok(Layer::Unknown(u)),
        }
    }
}
