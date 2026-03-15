use std::fmt;
use std::fmt::Debug;
use std::iter::repeat_n;

use num_traits::{PrimInt, ToPrimitive as _};

use crate::MltError::{
    DataWidthMismatch, ParsingLogicalTechnique, RleRunLenInvalid, UnsupportedLogicalEncoding,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{
    AsUsize as _, decode_componentwise_delta_vec2s, decode_morton_codes, decode_morton_delta,
    decode_zigzag, decode_zigzag_delta, encode_rle, encode_zigzag, encode_zigzag_delta,
};
use crate::v01::{
    LogicalData, LogicalEncoding, LogicalTechnique, LogicalValue, RleMeta, StreamMeta,
};
use crate::{Decoder, MltError};

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

impl RleMeta {
    /// Decode RLE (Run-Length Encoding) data.
    /// Charges the decoder for the expanded output allocation.
    pub fn decode<T: PrimInt + Debug>(
        self,
        data: &[T],
        dec: &mut Decoder,
    ) -> Result<Vec<T>, MltError> {
        let expected_len = self.runs.as_usize().checked_mul(2).or_overflow()?;
        fail_if_invalid_stream_size(data.len(), expected_len)?;

        let (run_lens, values) = data.split_at(self.runs.as_usize());
        fail_if_invalid_stream_size(self.num_rle_values, Self::calc_size(run_lens)?)?;

        let mut result = dec.alloc(self.num_rle_values.as_usize())?;
        for (&run_len, &val) in run_lens.iter().zip(values.iter()) {
            let run = run_len
                .to_usize()
                .ok_or_else(|| RleRunLenInvalid(run_len.to_i128().unwrap_or_default()))?;
            result.extend(repeat_n(val, run));
        }
        Ok(result)
    }

    fn calc_size<T: PrimInt + Debug>(run_lens: &[T]) -> Result<u32, MltError> {
        run_lens
            .iter()
            .try_fold(T::zero(), |a, v| a.checked_add(v))
            .and_then(|v| v.to_u32())
            .ok_or_else(|| RleRunLenInvalid(run_lens.len().to_i128().unwrap_or_default()))
    }
}

impl LogicalTechnique {
    pub fn parse(value: u8) -> Result<Self, MltError> {
        Self::try_from(value).or(Err(ParsingLogicalTechnique(value)))
    }
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
            Self::MortonDelta(v) => write!(f, "MortonDelta({v:?})"),
            Self::MortonRle(v) => write!(f, "MortonRle({v:?})"),
        }
    }
}

impl LogicalValue {
    #[must_use]
    pub fn new(meta: StreamMeta, data: LogicalData) -> Self {
        Self { meta, data }
    }

    pub fn decode_i32(self, dec: &mut Decoder) -> Result<Vec<i32>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag(&data)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag(&rle.decode(&data, dec)?)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::ComponentwiseDelta => match self.data {
                LogicalData::VecU32(data) => decode_componentwise_delta_vec2s(&data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, _>(&data)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    Ok(decode_zigzag_delta::<i32, _>(&rle.decode(&data, dec)?))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::Morton(meta) => match self.data {
                LogicalData::VecU32(data) => Ok(decode_morton_codes(
                    &data,
                    meta.num_bits,
                    meta.coordinate_shift,
                )),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::MortonDelta(meta) => match self.data {
                LogicalData::VecU32(data) => Ok(decode_morton_delta(
                    &data,
                    meta.num_bits,
                    meta.coordinate_shift,
                )),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "i32")),
            },
            LogicalEncoding::MortonRle(_) => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "i32 (MortonRle)",
            )),
            LogicalEncoding::PseudoDecimal => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "i32",
            )),
        }
    }

    pub fn decode_u32(self, dec: &mut Decoder) -> Result<Vec<u32>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU32(data) => Ok(data),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU32(data) => rle.decode(&data, dec),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU32(data) => Ok(decode_zigzag_delta::<i32, _>(&data)),
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU32(data) => {
                    Ok(decode_zigzag_delta::<i32, _>(&rle.decode(&data, dec)?))
                }
                LogicalData::VecU64(_) => Err(DataWidthMismatch("u64", "u32")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "u32",
            )),
        }
    }

    pub fn decode_i64(self, dec: &mut Decoder) -> Result<Vec<i64>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag(&data)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, _>(&data)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    Ok(decode_zigzag_delta::<i64, _>(&rle.decode(&data, dec)?))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag(&rle.decode(&data, dec)?)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "i64")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "i64",
            )),
        }
    }

    pub fn decode_u64(self, dec: &mut Decoder) -> Result<Vec<u64>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => match self.data {
                LogicalData::VecU64(data) => Ok(data),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::Rle(rle) => match self.data {
                LogicalData::VecU64(data) => rle.decode(&data, dec),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::Delta => match self.data {
                LogicalData::VecU64(data) => Ok(decode_zigzag_delta::<i64, _>(&data)),
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            LogicalEncoding::DeltaRle(rle) => match self.data {
                LogicalData::VecU64(data) => {
                    Ok(decode_zigzag_delta::<i64, _>(&rle.decode(&data, dec)?))
                }
                LogicalData::VecU32(_) => Err(DataWidthMismatch("u32", "u64")),
            },
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "u64",
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default, strum::EnumIter)]
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
    use crate::MltError::InvalidDecodingStreamSize;
    use crate::test_helpers::dec;
    use crate::v01::{DictionaryType, IntEncoding, PhysicalEncoding, StreamType};

    fn make_meta(logical_encoding: LogicalEncoding, num_values: usize) -> StreamMeta {
        let num_values =
            u32::try_from(num_values).expect("proptest to not generate that large of a vec");
        StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            IntEncoding::new(logical_encoding, PhysicalEncoding::None),
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
                .decode_u32(&mut dec())
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
                .decode_i32(&mut dec())
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
                .decode_u64(&mut dec())
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
                .decode_i64(&mut dec())
                .unwrap();
            prop_assert_eq!(decoded, values);
        }
    }

    #[test]
    fn test_decode_rle_empty() {
        let rle = RleMeta {
            runs: 0,
            num_rle_values: 0,
        };
        assert!(rle.decode::<u32>(&[], &mut dec()).unwrap().is_empty());
    }

    #[test]
    fn test_decode_rle_invalid_stream_size() {
        // Valid RLE for runs=2 needs 4 elements (2 run lengths + 2 values). Only 3 provided.
        let rle = RleMeta {
            runs: 2,
            num_rle_values: 3,
        };
        let data = [1u32, 2, 3];
        let err = rle.decode::<u32>(&data, &mut dec()).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(3, 4)));
    }
}
