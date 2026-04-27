use std::borrow::Cow;

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
                dense_idx: 0,
            },
            Self::Bits { bits, values } => PresenceOptIter {
                bits: Some(bits),
                values,
                feat_idx: 0,
                dense_idx: 0,
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
    feat_idx: usize,
    dense_idx: usize,
}

impl<T: Copy> Iterator for PresenceOptIter<'_, T> {
    type Item = Option<T>;

    fn next(&mut self) -> Option<Option<T>> {
        match self.bits {
            None => {
                let v = self.values.get(self.feat_idx).copied()?;
                self.feat_idx += 1;
                Some(Some(v))
            }
            Some(bits) => {
                if self.feat_idx >= bits.len() {
                    return None;
                }
                let present = bits[self.feat_idx];
                self.feat_idx += 1;
                if present {
                    let v = self.values[self.dense_idx];
                    self.dense_idx += 1;
                    Some(Some(v))
                } else {
                    Some(None)
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = match self.bits {
            None => self.values.len().saturating_sub(self.feat_idx),
            Some(bits) => bits.len().saturating_sub(self.feat_idx),
        };
        (remaining, Some(remaining))
    }
}

impl<T: Copy> ExactSizeIterator for PresenceOptIter<'_, T> {}
