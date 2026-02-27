use std::fmt;
use std::fmt::Debug;

use borrowme::borrowme;
use num_enum::TryFromPrimitive;
use num_traits::PrimInt;

use crate::MltError;
use crate::MltError::{DataWidthMismatch, ParsingLogicalTechnique, UnsupportedLogicalEncoding};
use crate::utils::{
    decode_componentwise_delta_vec2s, decode_rle, decode_zigzag, decode_zigzag_delta, encode_rle,
    encode_zigzag, encode_zigzag_delta,
};
use crate::v01::{MortonMeta, RleMeta, StreamMeta};

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
pub enum LogicalEncoding {
    None,
    Delta,
    DeltaRle(RleMeta),
    ComponentwiseDelta,
    Rle(RleMeta),
    Morton(MortonMeta),
    PseudoDecimal,
}

/// RLE-encode a sequence into `[run-lengths | unique-values]` and return the matching `RleMeta`.
/// `num_logical` is the expanded output length (stored in `RleMeta::num_rle_values`).
fn apply_rle<T: PrimInt + Debug>(
    data: &[T],
    num_logical: usize,
) -> Result<(Vec<T>, RleMeta), MltError> {
    let (runs_vec, vals_vec) = encode_rle(data);
    let meta = RleMeta {
        runs: u32::try_from(runs_vec.len())?,
        num_rle_values: u32::try_from(num_logical)?,
    };
    let mut combined = runs_vec;
    combined.extend(vals_vec);
    Ok((combined, meta))
}

impl Debug for LogicalEncoding {
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
        match self.meta.logical_encoding {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag::<i32>(&data)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag(&decoded))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::ComponentwiseDelta => match self.data {
                LogicalData::VecU32(data) => decode_componentwise_delta_vec2s(&data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, _>(data.as_slice())),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let rle_decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag_delta::<i32, _>(rle_decoded.as_slice()))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.logical_encoding,
                "i32",
            )),
        }
    }

    pub fn decode_u32(self) -> Result<Vec<u32>, MltError> {
        match self.meta.logical_encoding {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU32(data) => Ok(data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    decode_rle(&data, runs, num_rle_values)
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, u32>(data.as_slice())),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let rle_decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag_delta::<i32, u32>(rle_decoded.as_slice()))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.logical_encoding,
                "u32",
            )),
        }
    }

    pub fn decode_i64(self) -> Result<Vec<i64>, MltError> {
        match self.meta.logical_encoding {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag(&data)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, i64>(data.as_slice())),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let rle_decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag_delta::<i64, i64>(rle_decoded.as_slice()))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag(&decoded))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.logical_encoding,
                "i64",
            )),
        }
    }

    pub fn decode_u64(self) -> Result<Vec<u64>, MltError> {
        match self.meta.logical_encoding {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU64(data) => Ok(data),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    decode_rle(&data, runs, num_rle_values)
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, u64>(data.as_slice())),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    let runs = usize::try_from(rle.runs)?;
                    let num_rle_values = usize::try_from(rle.num_rle_values)?;
                    let rle_decoded = decode_rle(&data, runs, num_rle_values)?;
                    Ok(decode_zigzag_delta::<i64, u64>(rle_decoded.as_slice()))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.logical_encoding,
                "u64",
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum LogicalEncoder {
    #[default]
    None,
    Delta,
    DeltaRle,
    Rle,
    // FIXME: add more of the LogicalEncoding strategies
}
impl LogicalEncoder {
    /// Logically encode `u32` values, returning the physically-stored sequence and the concrete decoder.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_u32`] for the reverse operation.
    pub fn encode_u32s(self, values: &[u32]) -> Result<(Vec<u32>, LogicalEncoding), MltError> {
        match self {
            Self::None => Ok((values.to_vec(), LogicalEncoding::None)),
            Self::Delta => {
                let values = values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>();
                let u32s = encode_zigzag_delta(&values);
                Ok((u32s, LogicalEncoding::Delta))
            }
            Self::Rle => {
                let (u32s, meta) = apply_rle(values, values.len())?;
                Ok((u32s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let values = values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>();
                let delta = encode_zigzag_delta(&values);
                let (u32s, meta) = apply_rle(&delta, values.len())?;
                Ok((u32s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `i32` values into the `u32` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_i32`] for the reverse operation.
    pub fn encode_i32s(self, values: &[i32]) -> Result<(Vec<u32>, LogicalEncoding), MltError> {
        match self {
            Self::None => Ok((encode_zigzag(values), LogicalEncoding::None)),
            Self::Delta => Ok((encode_zigzag_delta(values), LogicalEncoding::Delta)),
            Self::Rle => {
                let (u32s, meta) = apply_rle(&encode_zigzag(values), values.len())?;
                Ok((u32s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let (u32s, meta) = apply_rle(&encode_zigzag_delta(values), values.len())?;
                Ok((u32s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `u64` values into the `u64` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_u64`] for the reverse operation.
    pub fn encode_u64s(self, values: &[u64]) -> Result<(Vec<u64>, LogicalEncoding), MltError> {
        match self {
            Self::None => Ok((values.to_vec(), LogicalEncoding::None)),
            Self::Delta => Ok((
                encode_zigzag_delta(&values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>()),
                LogicalEncoding::Delta,
            )),
            Self::Rle => {
                let (u64s, meta) = apply_rle(values, values.len())?;
                Ok((u64s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let delta = encode_zigzag_delta(
                    &values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>(),
                );
                let (u64s, meta) = apply_rle(&delta, values.len())?;
                Ok((u64s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `i64` values into the `u64` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_i64`] for the reverse operation.
    pub fn encode_i64s(self, values: &[i64]) -> Result<(Vec<u64>, LogicalEncoding), MltError> {
        match self {
            Self::None => Ok((encode_zigzag(values), LogicalEncoding::None)),
            Self::Delta => Ok((encode_zigzag_delta(values), LogicalEncoding::Delta)),
            Self::Rle => {
                let (u64s, meta) = apply_rle(&encode_zigzag(values), values.len())?;
                Ok((u64s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let (u64s, meta) = apply_rle(&encode_zigzag_delta(values), values.len())?;
                Ok((u64s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::v01::DictionaryType;
    use crate::v01::stream::physical::{PhysicalEncoding, StreamType};

    fn make_meta(logical_encoding: LogicalEncoding, num_values: usize) -> StreamMeta {
        let num_values =
            u32::try_from(num_values).expect("proptest to not generate that large of a vec");
        StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            logical_encoding,
            PhysicalEncoding::None,
            num_values,
        )
    }

    proptest! {
        #[test]
        fn test_u32_logical_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_u32s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta, LogicalData::VecU32(encoded))
                .decode_u32()
                .unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i32s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta, LogicalData::VecU32(encoded))
                .decode_i32()
                .unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_u64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta, LogicalData::VecU64(encoded))
                .decode_u64()
                .unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta, LogicalData::VecU64(encoded))
                .decode_i64()
                .unwrap();
            prop_assert_eq!(decoded, values);
        }
    }
}
