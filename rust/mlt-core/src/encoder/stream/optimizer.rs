use num_traits::{AsPrimitive as _, PrimInt as _, WrappingSub, Zero as _};
use zigzag::ZigZag;

use crate::encoder::IntEncoder;

/// Minimum number of values to profile / compete on.
///
/// Below this threshold the full slice is used regardless of its length.
const MIN_SAMPLE: usize = 512;

/// Hard upper bound on competition sample size.
const MAX_SAMPLE: usize = 16_384;

/// RLE is only worthwhile when runs are on average at least this long.
const RLE_MIN_AVG_RUN_LENGTH: f64 = 2.0;

/// Sampling-based encoder selection
#[derive(Debug, Clone, Default)]
pub struct DataProfile {
    /// Average run length in the sample.
    ///
    /// A run is a maximal sequence of identical consecutive values.
    /// `avg_run_length = sample_len / num_runs`.
    avg_run_length: f64,

    /// Average run length of the zigzag-encoded delta stream.
    ///
    /// This is computed over the deltas between consecutive sample values,
    /// excluding the initial/base value, so the effective stream length is
    /// `sample_len - 1`.
    ///
    /// For sequential data like `[1, 2, 3, ...]` the raw values are all
    /// distinct (`avg_run_length == 1`) but the delta stream is constant, so
    /// `delta_avg_run_length == N - 1`, making `DeltaRle` extremely effective.
    delta_avg_run_length: f64,

    /// `true` if the sample values are sorted in ascending or descending order.
    is_sorted: bool,

    /// Sum of per-value bit widths across the sample.
    ///
    /// Aggregate bit count captures the *typical* benefit of delta encoding,
    /// unlike max bit width which can be pinned by a single outlier.
    total_bits: u32,

    /// Sum of per-value bit widths of the zigzag-delta stream.
    delta_total_bits: u32,
}

impl DataProfile {
    /// Profile a `u32` sample in a single pass.
    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    fn profile<T>(sample: &[T::UInt]) -> Self
    where
        T: ZigZag,
        <T as ZigZag>::UInt: WrappingSub,
    {
        if sample.is_empty() {
            return Self::default();
        }

        let type_bits = T::zero().leading_zeros();
        let mut runs: usize = 1;
        let mut delta_runs: usize = 1;
        let mut is_sorted_rising = true;
        let mut is_sorted_falling = true;
        let mut total_bits: u32 = type_bits - sample[0].leading_zeros();
        let mut delta_total_bits: u32 = 0;
        let mut prev = sample[0];
        let mut prev_zz: T::UInt = T::UInt::zero();

        for (i, &v) in sample[1..].iter().enumerate() {
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
            if i == 0 {
                prev_zz = zz;
            } else if zz != prev_zz {
                delta_runs += 1;
                prev_zz = zz;
            }
            total_bits += type_bits - v.leading_zeros();
            delta_total_bits += type_bits - zz.leading_zeros();
            prev = v;
        }

        let delta_len = sample.len().saturating_sub(1).max(1);
        Self {
            avg_run_length: sample.len() as f64 / runs as f64,
            delta_avg_run_length: delta_len as f64 / delta_runs as f64,
            is_sorted: is_sorted_rising || is_sorted_falling,
            total_bits,
            delta_total_bits,
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
        Self::profile::<T>(sample).candidates(size_of::<T>() == 4)
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

        // DeltaRle – when delta pays off AND either raw or delta-transformed values have runs.
        if self.delta_is_beneficial() && (self.rle_is_viable() || self.delta_rle_is_viable()) {
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

    /// Returns `true` if RLE on the delta-transformed stream is viable.
    ///
    /// For sequential or constant-delta data the raw values are all distinct
    /// but the zigzag-delta values are identical, making `DeltaRle` optimal.
    #[must_use]
    fn delta_rle_is_viable(&self) -> bool {
        self.delta_avg_run_length >= RLE_MIN_AVG_RUN_LENGTH
    }

    /// Returns `true` if Delta encoding is expected to be beneficial.
    #[must_use]
    fn delta_is_beneficial(&self) -> bool {
        self.is_sorted || self.delta_total_bits < self.total_bits
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::PhysicalEncoder;

    #[test]
    fn candidates_rle_excluded_when_short_runs() {
        // All-distinct stream -> avg_run_length == 1 -> no raw-RLE candidate.
        // But sequential data has constant deltas -> DeltaRle IS included.
        let data: Vec<u32> = (0..100).collect();
        let candidates = DataProfile::prune_candidates::<i32>(&data);
        insta::assert_debug_snapshot!(candidates, @"
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
                logical: DeltaRle,
                physical: VarInt,
            },
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
    }

    #[test]
    fn select_u32_sequential_picks_delta_rle() {
        let data: Vec<u32> = (0..1_000).collect();
        let candidates = DataProfile::prune_candidates::<i32>(&data);
        insta::assert_debug_snapshot!(candidates, @"
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
    }

    #[test]
    fn select_u64_sequential_picks_delta_rle() {
        let data: Vec<u64> = (0u64..500).collect();
        let candidates = DataProfile::prune_candidates::<i64>(&data);
        insta::assert_debug_snapshot!(candidates, @"
        [
            IntEncoder {
                logical: DeltaRle,
                physical: VarInt,
            },
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
    }

    #[test]
    fn select_u32_empty_fallback() {
        let enc = DataProfile::prune_candidates::<i32>(&[]);
        assert_eq!(enc, vec![IntEncoder::plain()]);
    }

    #[test]
    fn select_u64_empty_fallback() {
        let enc = DataProfile::prune_candidates::<i64>(&[]);
        assert_eq!(enc, vec![IntEncoder::plain()]);
    }
}
