use num_traits::{AsPrimitive as _, PrimInt as _, WrappingSub, Zero as _};
use zigzag::ZigZag;

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

    /// Profile a representative sample.
    #[must_use]
    pub(crate) fn from_values<T>(values: &[T::UInt]) -> Self
    where
        T: ZigZag,
        <T as ZigZag>::UInt: WrappingSub,
    {
        if values.is_empty() {
            return Self::default();
        }

        let target = sample_size(values.len());
        let sample = block_sample(values, target);
        Self::profile::<T>(sample)
    }

    /// Returns `true` if RLE is a sensible candidate based on this profile.
    ///
    /// An average run length above the threshold means values repeat frequently
    /// enough that the run-length and unique-value arrays will be compact.
    #[must_use]
    pub(crate) fn rle_is_viable(&self) -> bool {
        self.avg_run_length >= RLE_MIN_AVG_RUN_LENGTH
    }

    /// Returns `true` if RLE on the delta-transformed stream is viable.
    ///
    /// For sequential or constant-delta data the raw values are all distinct
    /// but the zigzag-delta values are identical, making `DeltaRle` optimal.
    #[must_use]
    pub(crate) fn delta_rle_is_viable(&self) -> bool {
        self.delta_avg_run_length >= RLE_MIN_AVG_RUN_LENGTH
    }

    /// Returns `true` if Delta encoding is expected to be beneficial.
    #[must_use]
    pub(crate) fn delta_is_beneficial(&self) -> bool {
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

    fn assert_profile_flags(profile: &DataProfile, delta: bool, rle: bool, delta_rle: bool) {
        assert_eq!(profile.delta_is_beneficial(), delta, "delta");
        assert_eq!(profile.rle_is_viable(), rle, "rle");
        assert_eq!(profile.delta_rle_is_viable(), delta_rle, "delta_rle");
    }

    #[test]
    fn profile_sequential_u32_uses_delta_rle_without_raw_rle() {
        // All-distinct stream -> avg_run_length == 1 -> no raw-RLE candidate.
        // But sequential data has constant deltas -> delta-RLE is viable.
        let data: Vec<u32> = (0..100).collect();
        let profile = DataProfile::from_values::<i32>(&data);
        assert!(profile.is_sorted);
        assert_profile_flags(&profile, true, false, true);
    }

    #[test]
    fn profile_constant_u32_uses_raw_and_delta_rle() {
        let data = vec![1234u32; 500];
        let profile = DataProfile::from_values::<i32>(&data);
        assert!(profile.is_sorted);
        assert_profile_flags(&profile, true, true, true);
    }

    #[test]
    fn profile_sequential_u64_uses_delta_rle_without_raw_rle() {
        let data: Vec<u64> = (0u64..500).collect();
        let profile = DataProfile::from_values::<i64>(&data);
        assert!(profile.is_sorted);
        assert_profile_flags(&profile, true, false, true);
    }

    #[test]
    fn profile_empty_has_no_optimizing_flags() {
        let profile = DataProfile::from_values::<i32>(&[]);
        assert!(!profile.is_sorted);
        assert_profile_flags(&profile, false, false, false);
    }
}
