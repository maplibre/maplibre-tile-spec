//! What the encoder needs to know about a float type.

/// A float the encoders understand, identified by its bit pattern rather than its value.
/// `-0.0` must stay a distinct dictionary entry from `0.0`, and a NaN is equal to no float at all, itself included.
#[cfg_attr(
    not(feature = "unstable-v2"),
    allow(dead_code, reason = "only the tag 0x02 float encodings use these")
)]
pub trait FloatValue: Copy {
    /// Dictionary key type: this float's bits.
    type Bits: Copy + Eq + std::hash::Hash;

    /// Largest power of ten worth scaling by, past which the type has no more digits to recover.
    const MAX_EXPONENT: u8;

    fn key(self) -> Self::Bits;

    /// Widen to `f64`, so scaling arithmetic happens at full precision.
    fn widen(self) -> f64;

    fn narrow(value: f64) -> Self;

    fn same_bits(self, other: Self) -> bool;
}

impl FloatValue for f32 {
    type Bits = u32;

    const MAX_EXPONENT: u8 = 10;

    fn key(self) -> u32 {
        self.to_bits()
    }

    fn widen(self) -> f64 {
        f64::from(self)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "narrowing is the point; the caller verifies the value returns bit-for-bit"
    )]
    fn narrow(value: f64) -> Self {
        value as Self
    }

    fn same_bits(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl FloatValue for f64 {
    type Bits = u64;

    const MAX_EXPONENT: u8 = 18;

    fn key(self) -> u64 {
        self.to_bits()
    }

    fn widen(self) -> Self {
        self
    }

    fn narrow(value: f64) -> Self {
        value
    }

    fn same_bits(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}
