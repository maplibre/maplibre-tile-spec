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

impl<'a, T: Copy> Presence<'a, T> {
    /// Returns `true` if feature `idx` is present.
    ///
    /// Always `true` for [`Presence::AllPresent`]; `false` when out of bounds.
    #[inline]
    #[must_use]
    pub fn is_present(&self, idx: usize) -> bool {
        match self {
            Self::AllPresent(_) => true,
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
            Self::AllPresent(values) => values,
            Self::Bits { values, .. } => values,
        }
    }

    /// Returns the value for feature `idx`, or `None` if absent or out of bounds.
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
    /// Allocates; prefer [`Presence::get`] for single-feature access.
    #[must_use]
    pub fn materialize(&self) -> Vec<Option<T>> {
        match self {
            Self::AllPresent(values) => values.iter().copied().map(Some).collect(),
            Self::Bits { bits, values } => {
                let mut dense = values.iter().copied();
                bits.iter()
                    .by_vals()
                    .map(|present| if present { dense.next() } else { None })
                    .collect()
            }
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
