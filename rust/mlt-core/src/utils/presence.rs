use std::borrow::Cow;
use std::iter::FusedIterator;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::{Analyze, StatType};

/// Per-column feature presence bitvector paired with its dense values.
///
/// Bit order matches the wire format (`bitvec`'s `Lsb0`): bit `i` corresponds to
/// `(byte[i/8] >> (i%8)) & 1`.
#[derive(Clone, PartialEq, Debug)]
pub enum Presence<'a, T: Copy> {
    /// No presence stream — every feature has a value.
    AllPresent(Vec<T>),
    /// Per-feature packed bitvector: bit `i` is set iff feature `i` has a value.
    /// `values` holds only the non-null (present) entries in dense order.
    Bits {
        bits: Cow<'a, BitSlice<u8, Lsb0>>,
        values: Vec<T>,
    },
}

impl<T: Copy> Presence<'_, T> {
    /// Returns `true` if feature `idx` is present, `false` if absent or out of bounds.
    #[inline]
    #[must_use]
    pub fn is_present(&self, idx: usize) -> bool {
        match self {
            Self::AllPresent(values) => idx < values.len(),
            Self::Bits { bits, .. } => bits.get(idx).as_deref().copied().unwrap_or(false),
        }
    }

    /// Total number of features (present and absent).
    #[inline]
    #[must_use]
    pub fn feature_count(&self) -> usize {
        match self {
            Self::AllPresent(values) => values.len(),
            Self::Bits { bits, .. } => bits.len(),
        }
    }

    /// Dense values slice (present entries only).
    #[inline]
    #[must_use]
    pub fn dense_values(&self) -> &[T] {
        match self {
            Self::AllPresent(values) | Self::Bits { values, .. } => values,
        }
    }

    /// Returns the value for feature `idx`, or `None` if absent or out of bounds.
    ///
    /// For sequential access over all features prefer [`Presence::iter_optional`],
    /// which is O(1) per step. This method recomputes `count_ones()` each call and
    /// is O(idx) for sparse (Bits) presence.
    #[inline]
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<T> {
        match self {
            Self::AllPresent(values) => values.get(idx).copied(),
            Self::Bits { bits, values } => {
                if *bits.get(idx)? {
                    Some(values[bits[..idx].count_ones()])
                } else {
                    None
                }
            }
        }
    }

    /// Expand into a `Vec<Option<T>>` with one entry per feature.
    ///
    /// Allocates; prefer [`Presence::get`] for single-feature access or
    /// [`Presence::iter_optional`] for sequential access without allocation.
    #[must_use]
    pub fn materialize(&self) -> Vec<Option<T>> {
        self.iter_optional().collect()
    }

    /// Iterate over all features in order, yielding `Option<T>` per feature in O(1) per step.
    ///
    /// Unlike repeated [`Presence::get`] calls (which are O(idx) for sparse columns),
    /// this iterator tracks `dense_idx` internally and advances in O(1) per step.
    #[must_use]
    pub fn iter_optional(&self) -> PresenceOptIter<'_, T> {
        match self {
            Self::AllPresent(values) => PresenceOptIter {
                bits: None,
                values,
                feat_idx: 0,
                end: values.len(),
                dense_idx: 0,
                back_dense: None,
            },
            Self::Bits { bits, values } => PresenceOptIter {
                bits: Some(bits),
                values,
                feat_idx: 0,
                end: bits.len(),
                dense_idx: 0,
                back_dense: None,
            },
        }
    }
}

impl<T: Analyze + Copy> Analyze for Presence<'_, T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        if stat == StatType::DecodedMetaSize {
            0
        } else {
            let bits_size = match self {
                Self::AllPresent(_) => 0,
                Self::Bits { bits, .. } => bits.len().div_ceil(8),
            };
            bits_size + self.dense_values().collect_statistic(stat)
        }
    }
}

/// O(1)-per-step iterator over all features of a [`Presence`], yielding `Option<T>`.
///
/// Returned by [`Presence::iter_optional`]. Prefer this over repeated [`Presence::get`]
/// calls when iterating in order: `get` is O(idx) for sparse columns (recomputes
/// `count_ones()`), while this iterator advances in O(1) per step by tracking
/// `dense_idx` internally.
pub struct PresenceOptIter<'p, T: Copy> {
    /// `None` for `AllPresent`, `Some(bits)` for `Bits`.
    bits: Option<&'p BitSlice<u8, Lsb0>>,
    values: &'p [T],
    /// Next feature index to yield from the front.
    feat_idx: usize,
    /// One past the last feature index to yield from the back.
    end: usize,
    /// Dense index of the value belonging to `feat_idx`.
    dense_idx: usize,
    /// Dense index just past the next back value, filled in on the first
    /// `next_back` call so forward-only iteration never pays for the `count_ones`.
    back_dense: Option<usize>,
}

impl<T: Copy> Iterator for PresenceOptIter<'_, T> {
    type Item = Option<T>;

    fn next(&mut self) -> Option<Option<T>> {
        if self.feat_idx >= self.end {
            return None;
        }
        let idx = self.feat_idx;
        self.feat_idx += 1;
        match self.bits {
            None => Some(Some(self.values[idx])),
            Some(bits) if bits[idx] => {
                let v = self.values[self.dense_idx];
                self.dense_idx += 1;
                Some(Some(v))
            }
            Some(_) => Some(None),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl<T: Copy> DoubleEndedIterator for PresenceOptIter<'_, T> {
    fn next_back(&mut self) -> Option<Option<T>> {
        if self.feat_idx >= self.end {
            return None;
        }
        self.end -= 1;
        let idx = self.end;
        match self.bits {
            None => Some(Some(self.values[idx])),
            Some(bits) => {
                // Invariant: `back_dense` counts the present features in `0..=idx`.
                let dense = self
                    .back_dense
                    .get_or_insert_with(|| bits[..=idx].count_ones());
                if bits[idx] {
                    *dense -= 1;
                    Some(Some(self.values[*dense]))
                } else {
                    Some(None)
                }
            }
        }
    }
}

impl<T: Copy> ExactSizeIterator for PresenceOptIter<'_, T> {
    fn len(&self) -> usize {
        self.end - self.feat_idx
    }
}

impl<T: Copy> FusedIterator for PresenceOptIter<'_, T> {}

#[cfg(test)]
mod tests {
    use bitvec::bitvec;
    use rstest::rstest;

    use super::*;
    use crate::test_helpers::assert_size_hint_exact;

    fn sparse(pattern: &[bool]) -> Presence<'static, u8> {
        let mut bits = bitvec![u8, Lsb0;];
        let mut values = Vec::new();
        for (idx, &present) in pattern.iter().enumerate() {
            bits.push(present);
            if present {
                values.push(u8::try_from(idx).unwrap());
            }
        }
        Presence::Bits {
            bits: Cow::Owned(bits),
            values,
        }
    }

    fn expected(pattern: &[bool]) -> Vec<Option<u8>> {
        pattern
            .iter()
            .enumerate()
            .map(|(idx, &p)| p.then(|| u8::try_from(idx).unwrap()))
            .collect()
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::single_present(&[true])]
    #[case::single_absent(&[false])]
    #[case::alternating(&[true, false, true, false, true])]
    #[case::leading_absent(&[false, false, true, true, false, true, false])]
    #[case::all_present(&[true; 9])]
    #[case::all_absent(&[false; 9])]
    #[case::spans_two_bytes(&[true, false, true, true, false, false, true, false, true, true, false])]
    fn iter_optional_runs_backwards(#[case] pattern: &[bool]) {
        let presence = sparse(pattern);
        let want = expected(pattern);

        assert_eq!(presence.iter_optional().collect::<Vec<_>>(), want);
        assert_size_hint_exact(|| presence.iter_optional(), &want);

        let mut back = presence.iter_optional().rev().collect::<Vec<_>>();
        back.reverse();
        assert_eq!(back, want);
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::single(&[10])]
    #[case::many(&[10, 20, 30, 40])]
    fn iter_optional_runs_backwards_all_present(#[case] values: &[u8]) {
        let presence = Presence::AllPresent(values.to_vec());
        let want: Vec<_> = values.iter().copied().map(Some).collect();

        assert_eq!(presence.iter_optional().collect::<Vec<_>>(), want);
        assert_size_hint_exact(|| presence.iter_optional(), &want);

        let mut back = presence.iter_optional().rev().collect::<Vec<_>>();
        back.reverse();
        assert_eq!(back, want);
        assert_eq!(presence.iter_optional().next_back(), want.last().copied());
    }

    #[rstest]
    #[case::sparse(&[true, false, true, false, true])]
    #[case::all_absent(&[false, false])]
    fn get_matches_iter_optional(#[case] pattern: &[bool]) {
        let dense: Vec<u8> = expected(pattern).into_iter().flatten().collect();
        for presence in [sparse(pattern), Presence::AllPresent(dense.clone())] {
            let want = match presence {
                Presence::AllPresent(_) => dense.iter().copied().map(Some).collect(),
                Presence::Bits { .. } => expected(pattern),
            };

            assert_eq!(presence.feature_count(), want.len());
            assert_eq!(presence.dense_values(), dense);
            assert_eq!(presence.materialize(), want);
            for (idx, &value) in want.iter().enumerate() {
                assert_eq!(presence.get(idx), value);
                assert_eq!(presence.is_present(idx), value.is_some());
            }
            assert_eq!(presence.get(want.len()), None);
            assert!(!presence.is_present(want.len()));
        }
    }
}
