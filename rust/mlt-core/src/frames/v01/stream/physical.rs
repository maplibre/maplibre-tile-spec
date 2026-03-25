use integer_encoding::VarInt as _;

use crate::MltError::ParsingStreamType;
use crate::codecs::bytes::{encode_u32s_to_bytes, encode_u64s_to_bytes};
use crate::codecs::fastpfor::encode_fastpfor;
use crate::utils::parse_u8;
use crate::v01::{
    DictionaryType, EncodedStreamData, LengthType, OffsetType, PhysicalEncoding, StreamType,
};
use crate::{MltError, MltRefResult, MltResult};

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

impl PhysicalEncoding {
    pub fn parse(value: u8) -> MltResult<Self> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalEncoding(value)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PhysicalEncoder {
    None,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPFOR`.
    VarInt,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    ///
    /// Does not support u64/i64 integers
    FastPFOR,
}

impl PhysicalEncoder {
    /// Physically encode a `u32` sequence into the appropriate `EncodedStreamData` variant.
    pub fn encode_u32s(self, values: Vec<u32>) -> MltResult<(EncodedStreamData, PhysicalEncoding)> {
        match self {
            Self::None => {
                let data = encode_u32s_to_bytes(&values);
                let stream = EncodedStreamData::Encoded(data);
                Ok((stream, PhysicalEncoding::None))
            }
            Self::VarInt => {
                let mut data = Vec::new();
                for v in values {
                    data.extend_from_slice(&u64::from(v).encode_var_vec());
                }
                let stream = EncodedStreamData::VarInt(data);
                Ok((stream, PhysicalEncoding::VarInt))
            }
            Self::FastPFOR => {
                let data = encode_fastpfor(&values)?;
                let stream = EncodedStreamData::Encoded(data);
                Ok((stream, PhysicalEncoding::FastPFOR))
            }
        }
    }

    /// Physically encode a `u64` sequence into the appropriate `EncodedStreamData` variant.
    pub fn encode_u64s(self, values: Vec<u64>) -> MltResult<(EncodedStreamData, PhysicalEncoding)> {
        match self {
            Self::None => {
                let data = encode_u64s_to_bytes(&values);
                let stream = EncodedStreamData::Encoded(data);
                Ok((stream, PhysicalEncoding::None))
            }
            Self::VarInt => {
                let mut data = Vec::new();
                for v in values {
                    data.extend_from_slice(&v.encode_var_vec());
                }
                let stream = EncodedStreamData::VarInt(data);
                Ok((stream, PhysicalEncoding::VarInt))
            }
            Self::FastPFOR => Err(MltError::UnsupportedPhysicalEncoding("FastPFOR on u64")),
        }
    }
}
