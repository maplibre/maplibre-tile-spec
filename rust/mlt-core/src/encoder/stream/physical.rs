use integer_encoding::VarInt as _;

use crate::codecs::bytes::{encode_u32s_to_bytes, encode_u64s_to_bytes};
use crate::codecs::fastpfor::encode_fastpfor;
use crate::v01::{EncodedStreamData, PhysicalEncoding};
use crate::{MltError, MltResult};

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
                Ok((stream, PhysicalEncoding::FastPFor256))
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
