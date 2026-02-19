use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError::ParsingPhysicalStreamType;
use crate::v01::{DictionaryType, LengthType, OffsetType};
use crate::{MltError, MltRefResult, utils};

/// How should the stream be interpreted at the physical level (first pass of decoding)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhysicalStreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}
impl PhysicalStreamType {
    pub fn parse(input: &'_ [u8]) -> MltRefResult<'_, Self> {
        let (input, value) = utils::parse_u8(input)?;
        let pt = Self::from_u8(value).ok_or(ParsingPhysicalStreamType(value))?;
        Ok((input, pt))
    }

    fn from_u8(value: u8) -> Option<Self> {
        let high4 = value >> 4;
        let low4 = value & 0x0F;
        Some(match high4 {
            0 => PhysicalStreamType::Present,
            1 => PhysicalStreamType::Data(DictionaryType::try_from(low4).ok()?),
            2 => PhysicalStreamType::Offset(OffsetType::try_from(low4).ok()?),
            3 => PhysicalStreamType::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }
    #[must_use]
    pub fn as_u8(self) -> u8 {
        let proto_high4 = match self {
            PhysicalStreamType::Present => 0,
            PhysicalStreamType::Data(_) => 1,
            PhysicalStreamType::Offset(_) => 2,
            PhysicalStreamType::Length(_) => 3,
        };
        let high4 = proto_high4 << 4;
        let low4 = match self {
            PhysicalStreamType::Present => 0,
            PhysicalStreamType::Data(i) => i as u8,
            PhysicalStreamType::Offset(i) => i as u8,
            PhysicalStreamType::Length(i) => i as u8,
        };
        debug_assert!(low4 <= 0x0F, "secondary types should not exceed 4 bit");
        high4 | low4
    }
}

/// Physical decoder used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalDecoder {
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the decoder are easier to implement compared to `FastPfor`.
    VarInt = 2,
    /// Adaptive Lossless floating-Point Compression
    Alp = 3,
}

impl PhysicalDecoder {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalDecoder(value)))
    }
}
