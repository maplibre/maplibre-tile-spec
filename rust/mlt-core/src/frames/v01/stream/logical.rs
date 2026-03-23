use std::fmt;
use std::fmt::Debug;
use std::iter::repeat_n;

use num_traits::{PrimInt, ToPrimitive as _};

use crate::MltError::{ParsingLogicalTechnique, RleRunLenInvalid, UnsupportedLogicalEncoding};
use crate::codecs::morton::{decode_morton_codes, decode_morton_delta};
use crate::codecs::rle::encode_rle;
use crate::codecs::zigzag::{
    decode_componentwise_delta_vec2s, decode_zigzag, decode_zigzag_delta, encode_zigzag,
    encode_zigzag_delta,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::AsUsize as _;
use crate::v01::{LogicalEncoding, LogicalTechnique, LogicalValue, RleMeta, StreamMeta};
use crate::{Decoder, MltError, MltResult};

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

        let alloc_size = self.num_rle_values.as_usize();
        let mut result = dec.alloc(alloc_size)?;
        for (&run_len, &val) in run_lens.iter().zip(values.iter()) {
            let run = run_len
                .to_usize()
                .ok_or_else(|| RleRunLenInvalid(run_len.to_i128().unwrap_or_default()))?;
            result.extend(repeat_n(val, run));
        }
        dec.adjust_alloc(&result, alloc_size);
        Ok(result)
    }

    fn calc_size<T: PrimInt + Debug>(run_lens: &[T]) -> MltResult<u32> {
        run_lens
            .iter()
            .try_fold(T::zero(), |a, v| a.checked_add(v))
            .and_then(|v| v.to_u32())
            .ok_or_else(|| RleRunLenInvalid(run_lens.len().to_i128().unwrap_or_default()))
    }
}

impl LogicalTechnique {
    pub fn parse(value: u8) -> MltResult<Self> {
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
    pub fn new(meta: StreamMeta) -> Self {
        Self { meta }
    }

    /// Logically decode `data` (physically decoded u32 words) into `Vec<i32>`.
    ///
    /// Never called for `LogicalEncoding::None` — that case is handled directly
    /// in the bridge (physical buffer decoded into a fresh output Vec).
    pub fn decode_i32(self, data: &[u32], dec: &mut Decoder) -> Result<Vec<i32>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => decode_zigzag(data, dec),
            LogicalEncoding::Rle(rle) => decode_zigzag(&rle.decode(data, dec)?, dec),
            LogicalEncoding::ComponentwiseDelta => decode_componentwise_delta_vec2s(data, dec),
            LogicalEncoding::Delta => decode_zigzag_delta::<i32, _>(data, dec),
            LogicalEncoding::DeltaRle(rle) => {
                let expanded = rle.decode(data, dec)?;
                decode_zigzag_delta::<i32, _>(&expanded, dec)
            }
            LogicalEncoding::Morton(meta) => decode_morton_codes(data, meta, dec),
            LogicalEncoding::MortonDelta(meta) => decode_morton_delta(data, meta, dec),
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

    /// Logically decode `data` (physically decoded u32 words) into `Vec<u32>`.
    ///
    /// Not called for `LogicalEncoding::None` — that case is handled entirely
    /// in the bridge (physical buffer decoded directly into the output Vec).
    pub fn decode_u32(self, data: &[u32], dec: &mut Decoder) -> Result<Vec<u32>, MltError> {
        let num = self.meta.num_values.as_usize();
        match self.meta.encoding.logical {
            LogicalEncoding::None => {
                // Caller should have used the direct-output path; this is a fallback.
                dec.consume_items::<u32>(num)?;
                Ok(data.to_vec())
            }
            LogicalEncoding::Rle(rle) => rle.decode(data, dec),
            LogicalEncoding::Delta => decode_zigzag_delta::<i32, _>(data, dec),
            LogicalEncoding::DeltaRle(rle) => {
                decode_zigzag_delta::<i32, _>(&rle.decode(data, dec)?, dec)
            }
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "u32",
            )),
        }
    }

    /// Logically decode `data` (physically decoded u64 words) into `Vec<i64>`.
    ///
    /// Never called for `LogicalEncoding::None` — that case is handled directly
    /// in the bridge (physical buffer decoded into a fresh output Vec).
    pub fn decode_i64(self, data: &[u64], dec: &mut Decoder) -> Result<Vec<i64>, MltError> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => decode_zigzag(data, dec),
            LogicalEncoding::Delta => decode_zigzag_delta::<i64, _>(data, dec),
            LogicalEncoding::DeltaRle(rle) => {
                let expanded = rle.decode(data, dec)?;
                decode_zigzag_delta::<i64, _>(&expanded, dec)
            }
            LogicalEncoding::Rle(rle) => {
                // rle.decode() charges for expanded u64 vec; decode_zigzag charges for i64 vec
                let expanded = rle.decode(data, dec)?;
                decode_zigzag(&expanded, dec)
            }
            _ => Err(UnsupportedLogicalEncoding(
                self.meta.encoding.logical,
                "i64",
            )),
        }
    }

    /// Logically decode `data` (physically decoded u64 words) into `Vec<u64>`.
    ///
    /// Not called for `LogicalEncoding::None` — that case is handled entirely
    /// in the bridge (physical buffer decoded directly into the output Vec).
    pub fn decode_u64(self, data: &[u64], dec: &mut Decoder) -> Result<Vec<u64>, MltError> {
        let num = self.meta.num_values.as_usize();
        match self.meta.encoding.logical {
            LogicalEncoding::None => {
                // Caller should have used the direct-output path; this is a fallback.
                dec.consume_items::<u64>(num)?;
                Ok(data.to_vec())
            }
            LogicalEncoding::Rle(rle) => rle.decode(data, dec),
            LogicalEncoding::Delta => decode_zigzag_delta::<i64, _>(data, dec),
            LogicalEncoding::DeltaRle(rle) => {
                let expanded = rle.decode(data, dec)?;
                decode_zigzag_delta::<i64, _>(&expanded, dec)
            }
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
            let decoded = LogicalValue::new(meta).decode_u32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i32s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_u64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u64(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i64(&encoded, &mut dec()).unwrap();
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
