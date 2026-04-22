use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

/// Per-column feature presence bitvector for decoded scalar properties.
///
/// Bit order matches the wire format: bit `i` is `(byte[i/8] >> (i%8)) & 1`,
/// which is exactly `bitvec`'s `Lsb0` ordering.
///
/// When the stream has no compression (`PhysicalEncoding::None`), the inner
/// `Cow` borrows directly from the tile bytes (`Cow::Borrowed`) without
/// copying.  Otherwise it holds an owned `BitVec` produced after RLE
/// decompression (`Cow::Owned`).
#[derive(Clone, PartialEq, Debug)]
pub enum Presence<'a> {
    /// No presence stream — every feature has a value.
    AllPresent,
    /// Per-feature packed bitvector: bit `i` is set iff feature `i` has a value.
    Bits(Cow<'a, BitSlice<u8, Lsb0>>),
}

impl Presence<'_> {
    /// Returns `true` if feature `idx` is present.
    ///
    /// Always returns `true` for [`Presence::AllPresent`].
    /// Returns `false` when `idx` is out of bounds for [`Presence::Bits`].
    #[inline]
    #[must_use]
    pub fn is_present(&self, idx: usize) -> bool {
        match self {
            Self::AllPresent => true,
            Self::Bits(bits) => bits.get(idx).as_deref().copied().unwrap_or(false),
        }
    }

    /// Returns the number of present features before `idx` (i.e. the dense offset).
    ///
    /// For `AllPresent` this is just `idx`.  For `Bits` it counts set bits in `0..idx`.
    #[inline]
    #[must_use]
    pub fn dense_offset(&self, idx: usize) -> usize {
        match self {
            Self::AllPresent => idx,
            Self::Bits(bits) => bits[..idx].count_ones(),
        }
    }

    /// Total number of features covered by this presence descriptor.
    ///
    /// For [`Presence::AllPresent`] this equals `dense_len` (the dense values length).
    /// Passing the wrong `dense_len` for the `AllPresent` case is a bug.
    #[inline]
    #[must_use]
    pub fn feature_count(&self, dense_len: usize) -> usize {
        match self {
            Self::AllPresent => dense_len,
            Self::Bits(bits) => bits.len(),
        }
    }
}
