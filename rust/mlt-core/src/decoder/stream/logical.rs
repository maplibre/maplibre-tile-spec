use std::fmt::Debug;
use std::iter::repeat_n;

use num_traits::{PrimInt, ToPrimitive as _};
use usize_cast::IntoUsize as _;

use crate::MltError::{ParsingLogicalTechnique, RleRunLenInvalid, UnsupportedLogicalEncoding};
use crate::codecs::zigzag::{decode_componentwise_delta_vec2s, decode_zigzag, decode_zigzag_delta};
use crate::decoder::{LogicalEncoding, LogicalTechnique, LogicalValue, RleMeta, StreamMeta};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::{Decoder, MltResult};

impl RleMeta {
    /// Decode RLE (Run-Length Encoding) data.
    /// Charges the decoder for the expanded output allocation.
    pub fn decode<T: PrimInt + Debug>(self, data: &[T], dec: &mut Decoder) -> MltResult<Vec<T>> {
        match self {
            Self::Split {
                runs,
                num_rle_values,
            } => Self::decode_split(runs, num_rle_values, data, dec),
            Self::Interleaved { num_rle_values } => {
                Self::decode_interleaved(num_rle_values, data, dec)
            }
        }
    }

    /// Tag `0x01` layout: `[run_len × runs][value × runs]`.
    fn decode_split<T: PrimInt + Debug>(
        runs: u32,
        num_rle_values: u32,
        data: &[T],
        dec: &mut Decoder,
    ) -> MltResult<Vec<T>> {
        let expected_len = runs.into_usize().checked_mul(2).or_overflow()?;
        fail_if_invalid_stream_size(data.len(), expected_len)?;

        let (run_lens, values) = data.split_at(runs.into_usize());
        fail_if_invalid_stream_size(
            num_rle_values.into_usize(),
            Self::calc_size(run_lens)?.into_usize(),
        )?;

        let alloc_size = num_rle_values.into_usize();
        let mut result = dec.alloc(alloc_size)?;
        for (&run_len, &val) in run_lens.iter().zip(values.iter()) {
            let run = run_len
                .to_usize()
                .ok_or_else(|| RleRunLenInvalid(run_len.to_i128().unwrap_or_default()))?;
            result.extend(repeat_n(val, run));
        }
        dec.adjust_alloc(&result, alloc_size)?;
        Ok(result)
    }

    /// Tag `0x02` layout: `(run_len, value)` pairs. The run count is derived from
    /// the data length; `num_rle_values` comes from the stream's count context.
    fn decode_interleaved<T: PrimInt + Debug>(
        num_rle_values: u32,
        data: &[T],
        dec: &mut Decoder,
    ) -> MltResult<Vec<T>> {
        if !data.len().is_multiple_of(2) {
            return Err(RleRunLenInvalid(data.len().to_i128().unwrap_or_default()));
        }
        let alloc_size = num_rle_values.into_usize();
        let mut result = dec.alloc(alloc_size)?;
        for pair in data.chunks_exact(2) {
            let run = pair[0]
                .to_usize()
                .filter(|&run| run <= alloc_size - result.len())
                .ok_or_else(|| RleRunLenInvalid(pair[0].to_i128().unwrap_or_default()))?;
            result.extend(repeat_n(pair[1], run));
        }
        // The expanded count must exactly match the count declared by the stream context.
        fail_if_invalid_stream_size(result.len(), alloc_size)?;
        dec.adjust_alloc(&result, alloc_size)?;
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

impl LogicalValue {
    #[must_use]
    pub fn new(meta: StreamMeta) -> Self {
        Self { meta }
    }

    /// Logically decode `data` (physically decoded u32 words) into `Vec<i32>`.
    ///
    /// Never called for `LogicalEncoding::None` — that case is handled directly
    /// in the bridge (physical buffer decoded into a fresh output Vec).
    pub fn decode_i32(self, data: &[u32], dec: &mut Decoder) -> MltResult<Vec<i32>> {
        match self.meta.encoding.logical {
            LogicalEncoding::None => decode_zigzag(data, dec),
            LogicalEncoding::Rle(v) => decode_zigzag(&v.decode(data, dec)?, dec),
            LogicalEncoding::ComponentwiseDelta => decode_componentwise_delta_vec2s(data, dec),
            LogicalEncoding::Delta => decode_zigzag_delta::<i32, _>(data, dec),
            LogicalEncoding::DeltaRle(v) => {
                let expanded = v.decode(data, dec)?;
                decode_zigzag_delta::<i32, _>(&expanded, dec)
            }
            LogicalEncoding::Morton(v) => v.decode_codes(data, dec),
            LogicalEncoding::MortonDelta(v) => v.decode_delta(data, dec),
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
    pub fn decode_u32(self, data: &[u32], dec: &mut Decoder) -> MltResult<Vec<u32>> {
        let num = self.meta.num_values.into_usize();
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
    pub fn decode_i64(self, data: &[u64], dec: &mut Decoder) -> MltResult<Vec<i64>> {
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
    pub fn decode_u64(self, data: &[u64], dec: &mut Decoder) -> MltResult<Vec<u64>> {
        let num = self.meta.num_values.into_usize();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MltError::InvalidDecodingStreamSize;
    use crate::test_helpers::dec;

    #[test]
    fn test_decode_rle_empty() {
        let rle = RleMeta::Split {
            runs: 0,
            num_rle_values: 0,
        };
        assert!(rle.decode::<u32>(&[], &mut dec()).unwrap().is_empty());
    }

    #[test]
    fn test_decode_rle_invalid_stream_size() {
        // Valid RLE for runs=2 needs 4 elements (2 run lengths + 2 values). Only 3 provided.
        let rle = RleMeta::Split {
            runs: 2,
            num_rle_values: 3,
        };
        let data = [1u32, 2, 3];
        let err = rle.decode::<u32>(&data, &mut dec()).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(3, 4)));
    }

    #[test]
    fn test_decode_rle_interleaved() {
        let rle = RleMeta::Interleaved { num_rle_values: 6 };
        // (3 × 7), (1 × 9), (2 × 7)
        let data = [3u32, 7, 1, 9, 2, 7];
        let decoded = rle.decode(&data, &mut dec()).unwrap();
        assert_eq!(decoded, vec![7, 7, 7, 9, 7, 7]);
    }

    #[test]
    fn test_decode_rle_interleaved_empty() {
        let rle = RleMeta::Interleaved { num_rle_values: 0 };
        assert!(rle.decode::<u32>(&[], &mut dec()).unwrap().is_empty());
    }

    #[test]
    fn test_decode_rle_interleaved_count_mismatch() {
        // Runs sum to 4, but the context count declares 5.
        let rle = RleMeta::Interleaved { num_rle_values: 5 };
        let data = [3u32, 7, 1, 9];
        assert!(rle.decode(&data, &mut dec()).is_err());
    }

    #[test]
    fn test_decode_rle_interleaved_odd_length() {
        let rle = RleMeta::Interleaved { num_rle_values: 3 };
        let data = [3u32, 7, 1];
        assert!(rle.decode(&data, &mut dec()).is_err());
    }

    #[test]
    fn test_decode_rle_interleaved_overflowing_run() {
        // A single run larger than the declared count must not over-allocate.
        let rle = RleMeta::Interleaved { num_rle_values: 2 };
        let data = [u32::MAX, 7];
        assert!(rle.decode(&data, &mut dec()).is_err());
    }
}
