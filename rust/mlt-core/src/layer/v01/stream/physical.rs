use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError::ParsingStreamType;
use crate::utils::{
    encode_fastpfor, encode_u32s_to_bytes, encode_u64s_to_bytes, encode_varint, parse_u8,
};
use crate::v01::{
    DictionaryType, LengthType, OffsetType, OwnedDataVarInt, OwnedEncodedData, OwnedStreamData,
};
use crate::{MltError, MltRefResult};

/// How should the stream be interpreted at the physical level (first pass of decoding)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}
impl StreamType {
    pub fn parse(input: &'_ [u8]) -> MltRefResult<'_, Self> {
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
            0 => StreamType::Present,
            1 => StreamType::Data(DictionaryType::try_from(low4).ok()?),
            2 => StreamType::Offset(OffsetType::try_from(low4).ok()?),
            3 => StreamType::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }
    #[must_use]
    pub fn as_u8(self) -> u8 {
        let proto_high4 = match self {
            StreamType::Present => 0,
            StreamType::Data(_) => 1,
            StreamType::Offset(_) => 2,
            StreamType::Length(_) => 3,
        };
        let high4 = proto_high4 << 4;
        let low4 = match self {
            StreamType::Present => 0,
            StreamType::Data(i) => i as u8,
            StreamType::Offset(i) => i as u8,
            StreamType::Length(i) => i as u8,
        };
        debug_assert!(low4 <= 0x0F, "secondary types should not exceed 4 bit");
        high4 | low4
    }
}

/// Physical encoding used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum PhysicalEncoding {
    None = 0,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR = 1,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPfor`.
    VarInt = 2,
    /// Adaptive Lossless floating-Point Compression
    Alp = 3,
}

impl PhysicalEncoding {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalEncoding(value)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Physically encode a `u32` sequence into the appropriate `OwnedStreamData` variant.
    pub fn encode_u32s(
        self,
        values: Vec<u32>,
    ) -> Result<(OwnedStreamData, PhysicalEncoding), MltError> {
        match self {
            Self::None => {
                let data = encode_u32s_to_bytes(&values);
                let stream = OwnedStreamData::Encoded(OwnedEncodedData { data });
                Ok((stream, PhysicalEncoding::None))
            }
            Self::VarInt => {
                let mut data = Vec::new();
                for v in values {
                    encode_varint(&mut data, u64::from(v));
                }
                let stream = OwnedStreamData::VarInt(OwnedDataVarInt { data });
                Ok((stream, PhysicalEncoding::VarInt))
            }
            Self::FastPFOR => {
                let data = encode_fastpfor(&values)?;
                let stream = OwnedStreamData::Encoded(OwnedEncodedData { data });
                Ok((stream, PhysicalEncoding::FastPFOR))
            }
        }
    }

    /// Physically encode a `u64` sequence into the appropriate `OwnedStreamData` variant.
    pub fn encode_u64s(
        self,
        values: Vec<u64>,
    ) -> Result<(OwnedStreamData, PhysicalEncoding), MltError> {
        match self {
            Self::None => {
                let data = encode_u64s_to_bytes(&values);
                let stream = OwnedStreamData::Encoded(OwnedEncodedData { data });
                Ok((stream, PhysicalEncoding::None))
            }
            Self::VarInt => {
                let mut data = Vec::new();
                for v in values {
                    encode_varint(&mut data, v);
                }
                let stream = OwnedStreamData::VarInt(OwnedDataVarInt { data });
                Ok((stream, PhysicalEncoding::VarInt))
            }
            Self::FastPFOR => Err(MltError::UnsupportedPhysicalEncoding("FastPFOR on u64")),
        }
    }
}
