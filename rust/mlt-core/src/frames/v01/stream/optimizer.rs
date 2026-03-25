use num_traits::{AsPrimitive as _, PrimInt as _, WrappingSub, Zero as _};
use zigzag::ZigZag;

use crate::MltResult;
use crate::v01::EncodedStreamData;
use crate::v01::stream::IntEncoder;

/// Minimum number of values to profile / compete on.
///
/// Below this threshold the full slice is used regardless of its length.
const MIN_SAMPLE: usize = 512;

/// Hard upper bound on competition sample size.
const MAX_SAMPLE: usize = 4_096;

/// RLE is only worthwhile when runs are on average at least this long.
const RLE_MIN_AVG_RUN_LENGTH: f64 = 2.0;

/// Delta encoding is useful when the absolute delta values fit in fewer bits
/// than the original values.  Require at least this many bits of reduction
/// before enabling Delta on an unsorted stream.
const DELTA_BIT_SAVINGS_THRESHOLD: u8 = 4;

/// Sampling-based encoder selection
///
/// # Strategy
///
/// 1. [`Self::prune_candidates`] - **"Prune"**:
///    Compute lightweight statistics over a representative sample
///    of the data (average run length, sort order, max bit-width) and use them to prune obviously unsuitable candidates early.
/// 2. [`Self::compete_u32`] / [`Self::compete_u64`] - **"Compete"**:
///    Encode the same sample with every surviving candidate and
///    pick the one whose encoded output is smallest.
///    In case of a tie
///    - the physical priority order is `FastPFOR` > `VarInt` > `None` and,
///    - at the logical level, more complex transforms are deprioritized.
#[derive(Debug, Clone, Default)]
pub struct DataProfile {
    /// Number of values in the sample that was analyzed.
    _sample_len: usize,

    /// Average run length in the sample.
    ///
    /// A run is a maximal sequence of identical consecutive values.
    /// `avg_run_length = sample_len / num_runs`.
    avg_run_length: f64,

    /// `true` if the sample values are sorted in ascending or descending order.
    is_sorted: bool,

    /// Maximum number of bits required to represent any value in the sample
    /// (`T::BITS - v.leading_zeros()`).
    max_bit_width: u8,

    /// Maximum bit-width after zigzag-delta encoding.
    ///
    /// A value lower than `max_bit_width` signals that Delta compression will
    /// reduce value magnitudes and therefore benefit downstream integer
    /// encoders.
    delta_max_bit_width: u8,
}

impl DataProfile {
    /// Profile a `u32` sample in a single pass.
    #[must_use]
    #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn profile<T>(sample: &[T::UInt]) -> Self
    where
        T: ZigZag,
        <T as ZigZag>::UInt: WrappingSub,
    {
        if sample.is_empty() {
            return Self::default();
        }

        let mut runs: usize = 1;
        let mut is_sorted_rising = true;
        let mut is_sorted_falling = true;
        let mut max_val: T::UInt = sample[0];
        let mut max_delta: T::UInt = T::UInt::zero();
        let mut prev = sample[0];

        for &v in &sample[1..] {
            if v != prev {
                runs += 1;
            }
            if v < prev {
                is_sorted_rising = false;
            } else if prev < v {
                is_sorted_falling = false;
            }
            let delta_bits: T::UInt = v.wrapping_sub(&prev);
            let delta_signed: T = delta_bits.as_();
            let zz = T::encode(delta_signed);
            max_delta = max_delta.max(zz);
            max_val = v.max(max_val);
            prev = v;
        }

        Self {
            _sample_len: sample.len(),
            avg_run_length: sample.len() as f64 / runs as f64,
            is_sorted: is_sorted_rising || is_sorted_falling,
            max_bit_width: (T::zero().leading_zeros() - max_val.leading_zeros()) as u8,
            delta_max_bit_width: (T::zero().leading_zeros() - max_delta.leading_zeros()) as u8,
        }
    }

    /// Profile a representative sample to prune unsuitable candidates.
    #[must_use]
    pub fn prune_candidates<T>(values: &[T::UInt]) -> Vec<IntEncoder>
    where
        T: ZigZag,
        <T as ZigZag>::UInt: WrappingSub,
    {
        if values.is_empty() {
            return vec![IntEncoder::plain()];
        }

        let target = sample_size(values.len());
        let sample = block_sample(values, target);

        let profile = Self::profile::<T>(sample);
        profile.candidates(T::zero().count_zeros() == 32)
    }

    pub fn compete_u32(candidates: &[IntEncoder], data: &[u32]) -> IntEncoder {
        candidates
            .iter()
            .copied()
            .min_by_key(|&enc| encoded_size_u32(data, enc))
            .unwrap_or_else(IntEncoder::fastpfor)
    }
    pub fn compete_u64(candidates: &[IntEncoder], data: &[u64]) -> IntEncoder {
        candidates
            .iter()
            .copied()
            .min_by_key(|&enc| encoded_size_u64(data, enc))
            .unwrap_or_else(IntEncoder::varint)
    }

    /// Return the list of `Encoder` variants worth trying for `u32` data given the
    /// supplied profile.
    ///
    /// `FastPFOR` is always preferred over `VarInt`;
    /// `VarInt` is included as a fallback and for compatibility with gzip-compressed output.
    ///
    /// The returned vec is ordered from most- to least-complex so the competition
    /// loop breaks ties deterministically (first match wins on equal sizes).
    #[must_use]
    fn candidates(&self, fastpfor_is_allowed: bool) -> Vec<IntEncoder> {
        let mut out = Vec::with_capacity(8);

        // DeltaRle – only when both transforms pay off.
        if self.delta_is_beneficial() && self.rle_is_viable() {
            if fastpfor_is_allowed {
                out.push(IntEncoder::delta_rle_fastpfor());
            }
            out.push(IntEncoder::delta_rle_varint());
        }

        // Delta-only.
        if self.delta_is_beneficial() {
            if fastpfor_is_allowed {
                out.push(IntEncoder::delta_fastpfor());
            }
            out.push(IntEncoder::delta_varint());
        }

        // RLE-only (no delta).
        if self.rle_is_viable() {
            if fastpfor_is_allowed {
                out.push(IntEncoder::rle_fastpfor());
            }
            out.push(IntEncoder::rle_varint());
        }

        // Plain FastPFOR / VarInt are always candidates.
        if fastpfor_is_allowed {
            out.push(IntEncoder::fastpfor());
        }
        out.push(IntEncoder::varint());

        out
    }

    /// Returns `true` if RLE is a sensible candidate based on this profile.
    ///
    /// An average run length above the threshold means values repeat frequently
    /// enough that the run-length and unique-value arrays will be compact.
    #[must_use]
    fn rle_is_viable(&self) -> bool {
        self.avg_run_length >= RLE_MIN_AVG_RUN_LENGTH
    }

    /// Returns `true` if Delta encoding is expected to be beneficial.
    #[must_use]
    fn delta_is_beneficial(&self) -> bool {
        let bit_width_saving = self.max_bit_width.saturating_sub(self.delta_max_bit_width);
        self.is_sorted || bit_width_saving >= DELTA_BIT_SAVINGS_THRESHOLD
    }
}

fn block_sample<T: Clone + Copy>(values: &[T], target: usize) -> &[T] {
    if values.len() <= target {
        return values;
    }
    // Pick a starting point (could be middle or random)
    // and take a contiguous chunk to preserve RLE/Delta patterns.
    let start = (values.len() / 2).saturating_sub(target / 2);
    &values[start..start + target]
}

/// Compute the target sample size from the full stream length.
///
/// - Streams shorter than `MIN_SAMPLE` are sampled fully.
/// - Larger streams are sampled at ~1 % of their length, clamped to
///   `[MIN_SAMPLE, MAX_SAMPLE]`.
#[inline]
fn sample_size(len: usize) -> usize {
    if len <= MIN_SAMPLE {
        len
    } else {
        (len / 100).clamp(MIN_SAMPLE, MAX_SAMPLE)
    }
}

/// Encode `values` with `encoder` and return the number of bytes in the
/// physical payload (excluding stream metadata).
///
/// Returns `usize::MAX` on error so that a broken candidate is always ranked
/// last.
fn encoded_size_u32(values: &[u32], encoder: IntEncoder) -> usize {
    let result: MltResult<_> = (|| {
        let (physical_u32s, _logical_enc) = encoder.logical.encode_u32s(values)?;
        let (data, _physical_enc) = encoder.physical.encode_u32s(physical_u32s)?;
        Ok(data_byte_len(data))
    })();
    result.unwrap_or(usize::MAX)
}

fn encoded_size_u64(values: &[u64], encoder: IntEncoder) -> usize {
    let result: MltResult<_> = (|| {
        let (physical_u64s, _logical_enc) = encoder.logical.encode_u64s(values)?;
        let (data, _physical_enc) = encoder.physical.encode_u64s(physical_u64s)?;
        Ok(data_byte_len(data))
    })();
    result.unwrap_or(usize::MAX)
}

/// Return the byte length stored inside an `EncodedStreamData`.
fn data_byte_len(data: EncodedStreamData) -> usize {
    match data {
        EncodedStreamData::VarInt(v) | EncodedStreamData::Encoded(v) => v.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v01::PhysicalEncoder;

    #[test]
    fn candidates_rle_excluded_when_short_runs() {
        // All-distinct stream → avg_run_length == 1 → no RLE candidate.
        let data: Vec<u32> = (0..100).collect();
        let candidates = DataProfile::prune_candidates::<i32>(&data);
        insta::assert_debug_snapshot!(candidates, @"
        [
            IntEncoder {
                logical: Delta,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ]
        ");
    }

    #[test]
    fn candidates_u64_never_includes_fastpfor() {
        // FastPFOR is a 32-bit-only codec and must never appear for u64 streams.
        let data: Vec<u64> = (0..200).collect();
        let candidates = DataProfile::prune_candidates::<i64>(&data);
        for enc in &candidates {
            assert_ne!(
                enc.physical,
                PhysicalEncoder::FastPFOR,
                "FastPFOR invalid for u64"
            );
        }

        insta::assert_debug_snapshot!(candidates, @"
        [
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ]
        ");
        let enc = DataProfile::compete_u64(&candidates, &data);
        assert_eq!(enc, IntEncoder::delta_varint());
    }

    #[test]
    fn select_u32_sequential_picks_delta() {
        let data: Vec<u32> = (0..1_000).collect();
        let enc = DataProfile::prune_candidates::<i32>(&data);
        insta::assert_debug_snapshot!(enc, @"
        [
            IntEncoder {
                logical: Delta,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ]
        ");
        let enc = DataProfile::compete_u32(&enc, &data);
        assert_eq!(enc, IntEncoder::delta_fastpfor());
    }

    #[test]
    fn select_u32_constant_picks_rle() {
        let data = vec![1234u32; 500];
        let enc = DataProfile::prune_candidates::<i32>(&data);
        insta::assert_debug_snapshot!(enc, @"
        [
            IntEncoder {
                logical: DeltaRle,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: DeltaRle,
                physical: VarInt,
            },
            IntEncoder {
                logical: Delta,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: Rle,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: Rle,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: FastPFOR,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ]
        ");
        let enc = DataProfile::compete_u32(&enc, &data);
        assert_eq!(enc, IntEncoder::rle_varint());
    }

    #[test]
    fn select_u64_sequential_picks_delta() {
        let data: Vec<u64> = (0u64..500).collect();
        let enc = DataProfile::prune_candidates::<i64>(&data);
        insta::assert_debug_snapshot!(enc, @"
        [
            IntEncoder {
                logical: Delta,
                physical: VarInt,
            },
            IntEncoder {
                logical: None,
                physical: VarInt,
            },
        ]
        ");
        let enc = DataProfile::compete_u64(&enc, &data);
        assert_eq!(enc, IntEncoder::delta_varint());
    }

    #[test]
    fn select_u32_empty_fallback() {
        let enc = DataProfile::prune_candidates::<i32>(&[]);
        assert_eq!(enc, vec![IntEncoder::plain()]);
        let enc = DataProfile::compete_u64(&enc, &[]);
        assert_eq!(enc, IntEncoder::plain());
    }

    #[test]
    fn select_u64_empty_fallback() {
        let enc = DataProfile::prune_candidates::<i64>(&[]);
        assert_eq!(enc, vec![IntEncoder::plain()]);
        let enc = DataProfile::compete_u32(&enc, &[]);
        assert_eq!(enc, IntEncoder::plain());
    }
}
