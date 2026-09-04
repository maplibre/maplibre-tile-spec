use crate::codecs::varint::parse_varint;
#[cfg(feature = "unstable-v2")]
use crate::decoder::root02::parse_layer02;
use crate::decoder::{Layer01, ParsedLayer01, Unknown};
#[cfg(feature = "unstable-v2")]
use crate::utils::parse_string;
use crate::utils::{parse_u8, take};
use crate::{
    DecodeState, Decoder, Layer, Lazy, MltError, MltRefResult, MltResult, ParsedLayer, Parser,
};

impl<'a, S: DecodeState> Layer<'a, S> {
    /// Returns the inner [`Layer01`] for any layer stored in the `Layer01`
    /// in-memory representation (both `Tag01` and `Tag02`), or `None` otherwise.
    #[must_use]
    pub fn as_layer01(&self) -> Option<&Layer01<'a, S>> {
        match self {
            Self::Tag01(l) => Some(l),
            #[cfg(feature = "unstable-v2")]
            Self::Tag02(l) => Some(l),
            Self::Unknown(_) => None,
        }
    }

    /// Consumes this layer and returns the inner [`Layer01`] for any layer stored
    /// in the `Layer01` in-memory representation, or `None` otherwise.
    #[must_use]
    pub fn into_layer01(self) -> Option<Layer01<'a, S>> {
        match self {
            Self::Tag01(l) => Some(l),
            #[cfg(feature = "unstable-v2")]
            Self::Tag02(l) => Some(l),
            Self::Unknown(_) => None,
        }
    }
}

/// Read a tile's name table record: a count, then that many strings.
#[cfg(feature = "unstable-v2")]
fn parse_name_table(input: &[u8]) -> MltResult<Vec<&str>> {
    let (mut input, count) = parse_varint::<u32>(input)?;
    let mut names = Vec::with_capacity(usize::min(count as usize, input.len()));
    for _ in 0..count {
        let name;
        (input, name) = parse_string(input)?;
        names.push(name);
    }
    if !input.is_empty() {
        return Err(MltError::TrailingLayerData(input.len()));
    }
    Ok(names)
}

impl<'a> Layer<'a> {
    /// Parse a single tuple that consists of `size (varint)`, `tag (varint)`, and `value (bytes)`.
    /// Reserves memory for decoded data against the parser's budget.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tests read one record at a time")
    )]
    pub(crate) fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltRefResult<'a, Self> {
        #[cfg(feature = "unstable-v2")]
        {
            let mut names = None;
            let (rest, layer) = Self::from_bytes_named(input, parser, &mut names)?;
            // A caller reading one record at a time has nowhere to keep a name table.
            let layer = layer.ok_or(MltError::NotImplemented(
                "a name table record read outside a whole-tile parse",
            ))?;
            Ok((rest, layer))
        }
        #[cfg(not(feature = "unstable-v2"))]
        {
            let (input, size) = parse_varint::<u32>(input)?;
            let (input, tag) = parse_u8(input)?;
            let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
            let (input, value) = take(input, size)?;
            let layer = match tag {
                1 => Layer::Tag01(Layer01::from_bytes(value, parser)?),
                tag => Layer::Unknown(Unknown { tag, value }),
            };
            Ok((input, layer))
        }
    }

    /// Parse one record, which is a layer unless it is the tile's name table.
    ///
    /// The table is read into `names` rather than returned, since it describes
    /// the records after it rather than standing on its own.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn from_bytes_named(
        input: &'a [u8],
        parser: &mut Parser,
        names: &mut Option<Vec<&'a str>>,
    ) -> MltRefResult<'a, Option<Self>> {
        let (input, size) = parse_varint::<u32>(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (input, value) = take(input, size)?;

        let layer = match tag {
            1 => Some(Layer::Tag01(Layer01::from_bytes(value, parser)?)),
            2 => Some(Layer::Tag02(parse_layer02(
                value,
                parser,
                names.as_deref(),
            )?)),
            crate::encoder::names02::NAME_TABLE_TAG => {
                *names = Some(parse_name_table(value)?);
                None
            }
            tag => Some(Layer::Unknown(Unknown { tag, value })),
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
