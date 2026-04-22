use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

/// Per-column feature presence bitvector for decoded scalar properties.
///
/// Bit order matches the wire format (`bitvec`'s `Lsb0`): bit `i` corresponds to
/// `(byte[i/8] >> (i%8)) & 1`.
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
    /// Always `true` for [`Presence::AllPresent`]; `false` when out of bounds.
    #[inline]
    #[must_use]
    pub fn is_present(&self, idx: usize) -> bool {
        match self {
            Self::AllPresent => true,
            Self::Bits(bits) => bits.get(idx).as_deref().copied().unwrap_or(false),
        }
    }
}
