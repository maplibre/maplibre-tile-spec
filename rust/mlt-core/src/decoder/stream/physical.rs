use crate::MltError::ParsingStreamType;
use crate::MltRefResult;
use crate::decoder::{DictionaryType, LengthType, OffsetType, StreamType};
use crate::utils::parse_u8;

impl StreamType {
    pub fn from_bytes(input: &'_ [u8]) -> MltRefResult<'_, Self> {
        let (input, value) = parse_u8(input)?;
        let pt = Self::from_u8(value).ok_or(ParsingStreamType(value))?;
        Ok((input, pt))
    }

    fn from_u8(value: u8) -> Option<Self> {
        let high4 = value >> 4;
        let low4 = value & 0x0F;
        Some(match high4 {
            #[cfg(fuzzing)]
            // when fuzzing, we cannot have ignored bits, to preserve roundtrip-ability
            0 if low4 == 0 => StreamType::Present,
            #[cfg(not(fuzzing))]
            0 => Self::Present,
            1 => Self::Data(DictionaryType::try_from(low4).ok()?),
            2 => Self::Offset(OffsetType::try_from(low4).ok()?),
            3 => Self::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }
    #[must_use]
    pub fn as_u8(self) -> u8 {
        let proto_high4 = match self {
            Self::Present => 0,
            Self::Data(_) => 1,
            Self::Offset(_) => 2,
            Self::Length(_) => 3,
        };
        let high4 = proto_high4 << 4;
        let low4 = match self {
            Self::Present => 0,
            Self::Data(i) => i as u8,
            Self::Offset(i) => i as u8,
            Self::Length(i) => i as u8,
        };
        debug_assert!(low4 <= 0x0F, "secondary types should not exceed 4 bit");
        high4 | low4
    }
}
