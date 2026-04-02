use std::fmt;

use crate::frames::v01::Layer01;
use crate::{DecodeState, Lazy, Parsed};

/// A layer that can be one of the known types, or an unknown.
///
/// The decode-state type parameter `S` mirrors [`Layer01<'a, S>`]:
/// - `Layer<'a>` / `Layer<'a, Lazy>` — freshly parsed; columns may still be raw bytes.
/// - `Layer<'a, Parsed>` — returned by [`Layer::decode_all`]; all columns are decoded. Use `ParsedLayer` alias.
pub enum Layer<'a, S: DecodeState = Lazy> {
    /// MVT-compatible layer (tag = 1)
    Tag01(Layer01<'a, S>),
    /// Unknown layer with tag, size, and value
    Unknown(Unknown<'a>),
}
pub type ParsedLayer<'a> = Layer<'a, Parsed>;

impl<'a, S: DecodeState> fmt::Debug for Layer<'a, S>
where
    Layer01<'a, S>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag01(l) => f.debug_tuple("Tag01").field(l).finish(),
            Self::Unknown(u) => f.debug_tuple("Unknown").field(u).finish(),
        }
    }
}

impl<'a, S: DecodeState> PartialEq for Layer<'a, S>
where
    Layer01<'a, S>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Tag01(a), Self::Tag01(b)) => a == b,
            (Self::Unknown(a), Self::Unknown(b)) => a == b,
            _ => false,
        }
    }
}

/// Unknown layer data, stored as encoded bytes
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Unknown<'a> {
    pub(crate) tag: u8,
    pub(crate) value: &'a [u8],
}
