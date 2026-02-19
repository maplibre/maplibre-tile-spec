use std::fmt;
use std::fmt::Debug;

use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError::{DataWidthMismatch, ParsingLogicalTechnique, UnsupportedLogicalDecoder};
use crate::utils::{decode_componentwise_delta_vec2s, decode_rle, decode_zigzag_delta};
use crate::v01::{MortonMeta, RleMeta, StreamMeta};
use crate::{MltError, utils};

/// Logical encoding technique used for a column, as stored in the tile
#[borrowme]
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum LogicalTechnique {
    None = 0,
    Delta = 1,
    ComponentwiseDelta = 2,
    Rle = 3,
    Morton = 4,
    PseudoDecimal = 5,
}

impl LogicalTechnique {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(ParsingLogicalTechnique(value)))
    }
}
/// How should the stream be interpreted at the logical level (second pass of decoding)
#[derive(Clone, Copy, PartialEq)]
pub enum LogicalDecoder {
    None,
    Delta,
    DeltaRle(RleMeta),
    ComponentwiseDelta,
    Rle(RleMeta),
    Morton(MortonMeta),
    PseudoDecimal,
}

impl Debug for LogicalDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Delta => write!(f, "Delta"),
            Self::ComponentwiseDelta => write!(f, "ComponentwiseDelta"),
            Self::PseudoDecimal => write!(f, "PseudoDecimal"),
            Self::DeltaRle(v) => write!(f, "DeltaRle({v:?})"),
            Self::Rle(v) => write!(f, "Rle({v:?})"),
            Self::Morton(v) => write!(f, "Morton({v:?})"),
        }
    }
}

/// Representation of decoded stream data
/// TODO: decoded stream data representation has not been finalized yet
#[derive(Debug, PartialEq)]
pub enum LogicalData {
    VecU32(Vec<u32>),
    VecU64(Vec<u64>),
}

/// Representation of a decoded value
/// TODO: decoded stream data representation has not been finalized yet
#[derive(Debug, PartialEq)]
pub struct LogicalValue {
    meta: StreamMeta,
    data: LogicalData,
}

impl LogicalValue {
    #[must_use]
    pub fn new(meta: StreamMeta, data: LogicalData) -> Self {
        Self { meta, data }
    }

    pub fn decode_i32(self) -> Result<Vec<i32>, MltError> {
        match self.meta.logical_decoder {
            LogicalDecoder::None => match self.data {
                LogicalData::VecU32(data) => Ok(utils::decode_zigzag::<i32>(&data)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalDecoder::Rle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    let decoded =
                        decode_rle(&data, rle.runs as usize, rle.num_rle_values as usize)?;
                    Ok(utils::decode_zigzag::<i32>(&decoded))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalDecoder::ComponentwiseDelta => match self.data {
                LogicalData::VecU32(data) => decode_componentwise_delta_vec2s(&data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalDecoder::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, _>(data.as_slice())),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalDecoder::DeltaRle(rle_meta) => {
                match self.data {
                    LogicalData::VecU32(data) => {
                        // First decode RLE, then apply ZigZag Delta decoding
                        let rle_decoded = decode_rle(
                            &data,
                            rle_meta.runs as usize,
                            rle_meta.num_rle_values as usize,
                        )?;
                        Ok(decode_zigzag_delta::<i32, _>(rle_decoded.as_slice()))
                    }
                    LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
                }
            }
            _ => Err(UnsupportedLogicalDecoder(self.meta.logical_decoder, "i32")),
        }
    }

    pub fn decode_u32(self) -> Result<Vec<u32>, MltError> {
        match self.meta.logical_decoder {
            LogicalDecoder::None => match self.data {
                LogicalData::VecU32(data) => Ok(data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalDecoder::Rle(value) => match self.data {
                LogicalData::VecU32(data) => {
                    decode_rle(&data, value.runs as usize, value.num_rle_values as usize)
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalDecoder::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, u32>(data.as_slice())),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalDecoder::DeltaRle(rle_meta) => {
                match self.data {
                    LogicalData::VecU32(data) => {
                        // First decode RLE, then apply ZigZag Delta decoding
                        let rle_decoded = decode_rle(
                            &data,
                            rle_meta.runs as usize,
                            rle_meta.num_rle_values as usize,
                        )?;
                        Ok(decode_zigzag_delta::<i32, u32>(rle_decoded.as_slice()))
                    }
                    LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
                }
            }
            _ => Err(UnsupportedLogicalDecoder(self.meta.logical_decoder, "u32")),
        }
    }

    pub fn decode_i64(self) -> Result<Vec<i64>, MltError> {
        match self.meta.logical_decoder {
            LogicalDecoder::None => match self.data {
                LogicalData::VecU64(data) => Ok(utils::decode_zigzag::<i64>(&data)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalDecoder::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, i64>(data.as_slice())),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalDecoder::DeltaRle(rle_meta) => match self.data {
                LogicalData::VecU64(data) => {
                    let rle_decoded = decode_rle(
                        &data,
                        rle_meta.runs as usize,
                        rle_meta.num_rle_values as usize,
                    )?;
                    Ok(decode_zigzag_delta::<i64, i64>(rle_decoded.as_slice()))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalDecoder::Rle(value) => match self.data {
                LogicalData::VecU64(data) => {
                    let decoded =
                        decode_rle(&data, value.runs as usize, value.num_rle_values as usize)?;
                    Ok(utils::decode_zigzag::<i64>(&decoded))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            _ => Err(UnsupportedLogicalDecoder(self.meta.logical_decoder, "i64")),
        }
    }

    pub fn decode_u64(self) -> Result<Vec<u64>, MltError> {
        match self.meta.logical_decoder {
            LogicalDecoder::None => match self.data {
                LogicalData::VecU64(data) => Ok(data),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalDecoder::Rle(value) => match self.data {
                LogicalData::VecU64(data) => {
                    decode_rle(&data, value.runs as usize, value.num_rle_values as usize)
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalDecoder::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, u64>(data.as_slice())),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalDecoder::DeltaRle(rle_meta) => {
                match self.data {
                    LogicalData::VecU64(data) => {
                        // First decode RLE, then apply ZigZag Delta decoding
                        let rle_decoded = decode_rle(
                            &data,
                            rle_meta.runs as usize,
                            rle_meta.num_rle_values as usize,
                        )?;
                        Ok(decode_zigzag_delta::<i64, u64>(rle_decoded.as_slice()))
                    }
                    LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
                }
            }
            _ => Err(UnsupportedLogicalDecoder(self.meta.logical_decoder, "u64")),
        }
    }
}
