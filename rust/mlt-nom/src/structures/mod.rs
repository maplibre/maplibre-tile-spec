use borrowme::borrowme;
use nom::Err::Error as NomError;
use nom::bytes::complete::take;
use nom::combinator::complete;
use nom::error::{Error, ErrorKind};
use nom::multi::many0;
use nom::{IResult, Parser};

use crate::structures::v1::{FeatureMetaTable, FeatureTable};
use crate::utils;

pub(crate) mod v1;

/// A layer that can be either MVT-compatible or unknown
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Layer<'a> {
    /// MVT-compatible layer (tag = 1)
    Layer(FeatureTable<'a>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}

impl Layer<'_> {
    /// Parse a single binary tuple: size (varint), tag (varint), value (bytes)
    pub fn parse(input: &[u8]) -> IResult<&[u8], Layer<'_>> {
        let (input, size) = utils::parse_varint_usize(input)?;

        // tag is a varint, but we know fewer than 127 tags for now,
        // so we can use a faster u8 and fail if it is bigger than 127.
        let (input, tag) = utils::parse_u8(input)?;
        // 1 byte must be parsed for the tag, so if size is 0, it's invalid
        let size = size
            .checked_sub(1)
            .ok_or(NomError(Error::new(input, ErrorKind::Fail)))?;
        let (input, value) = take(size)(input)?;

        let layer = match tag {
            // For now we only support tag 0x01 layers, but more will be added soon
            1 => {
                let (data, meta) = FeatureMetaTable::parse(value)?;
                Layer::Layer(FeatureTable::parse(data, meta)?)
            }
            tag => Layer::Unknown(Unknown { tag, value }),
        };

        Ok((input, layer))
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

/// Parse a sequence of binary layers
pub fn parse_binary_stream(input: &[u8]) -> IResult<&[u8], Vec<Layer<'_>>> {
    many0(complete(Layer::parse)).parse(input)
}
