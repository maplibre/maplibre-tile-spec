use crate::codecs::varint::parse_varint;
#[cfg(feature = "unstable-v2")]
use crate::decoder::root02::parse_layer02;
use crate::decoder::{Layer01, ParsedLayer01, Unknown};
#[cfg(feature = "unstable-v2")]
use crate::decoder::{Layer02, ParsedLayer02};
use crate::utils::{parse_u8, take};
use crate::{
    DecodeState, Decoder, Layer, Lazy, MltError, MltRefResult, MltResult, ParsedLayer, Parser,
};

impl<'a, S: DecodeState> Layer<'a, S> {
    /// The layer's name, whatever its tag, or `None` for a tag this build does not
    /// know.
    ///
    /// Returning the value rather than the layer is what makes this safe to offer:
    /// handing back one version's layer type would quietly drop the columns another
    /// version adds, which is why there is no such accessor.
    #[must_use]
    pub fn name(&self) -> Option<&'a str> {
        match self {
            Self::Tag01(l) => Some(l.name()),
            #[cfg(feature = "unstable-v2")]
            Self::Tag02(l) => Some(l.layer().name()),
            Self::Unknown(_) => None,
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
            #[cfg(feature = "unstable-v2")]
            2 => Layer::Tag02(parse_layer02(value, parser)?),
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
            Layer::Tag01(v) => Ok(Layer::Tag01(v.decode_all(dec)?)),
            #[cfg(feature = "unstable-v2")]
            Layer::Tag02(v) => Ok(Layer::Tag02(v.decode_all(dec)?)),
            Layer::Unknown(u) => Ok(Layer::Unknown(u)),
        }
    }
}

impl<'a> Layer01<'a, Lazy> {
    /// Decode all columns and transition to [`Layer01<Parsed>`].
    ///
    /// Consumes `self` (a `Layer01<Lazy>`) and returns a `Layer01<Parsed>` where every
    /// column field holds its parsed value directly, enabling infallible readonly access.
    pub fn decode_all(self, dec: &mut Decoder) -> MltResult<ParsedLayer01<'a>> {
        Ok(Layer01 {
            name: self.name,
            extent: self.extent,
            id: self.id.map(|id| id.into_parsed(dec)).transpose()?,
            geometry: self.geometry.into_parsed(dec)?,
            properties: self
                .properties
                .into_iter()
                .map(|p| p.into_parsed(dec))
                .collect::<MltResult<Vec<_>>>()?,
            #[cfg(fuzzing)]
            layer_order: self.layer_order,
        })
    }
}

#[cfg(feature = "unstable-v2")]
impl<'a> Layer02<'a, Lazy> {
    /// Decode every column, the shared ones and the two only v2 has.
    pub fn decode_all(self, dec: &mut Decoder) -> MltResult<ParsedLayer02<'a>> {
        let layer = Layer02 {
            layer: self.layer.decode_all(dec)?,
            nested: self
                .nested
                .into_iter()
                .map(|n| n.into_parsed(dec))
                .collect::<MltResult<Vec<_>>>()?,
            m_values: self
                .m_values
                .into_iter()
                .map(|m| m.into_parsed(dec))
                .collect::<MltResult<Vec<_>>>()?,
        };
        // An m-value column's length is only checkable once the geometry is decoded
        // too, which it now is. Every later walk of a column relies on this check.
        for column in &layer.m_values {
            column.check_length(&layer.layer.geometry)?;
        }
        Ok(layer)
    }
}
