use crate::v01::{DataProfile, LogicalEncoder, PhysicalEncoder};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IntegerEncoder {
    pub logical: LogicalEncoder,
    pub physical: PhysicalEncoder,
}

impl IntegerEncoder {
    #[must_use]
    pub const fn new(logical: LogicalEncoder, physical: PhysicalEncoder) -> Self {
        Self { logical, physical }
    }

    #[must_use]
    pub fn plain() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::None, PhysicalEncoder::None)
    }
    #[must_use]
    pub fn varint() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::None, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn rle_varint() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::Rle, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn delta_rle_varint() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::DeltaRle, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn fastpfor() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::None, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn rle_fastpfor() -> IntegerEncoder {
        IntegerEncoder::new(LogicalEncoder::Rle, PhysicalEncoder::FastPFOR)
    }

    /// Automatically select the best encoder for a `u32` stream.
    ///
    /// Uses the `BTRBlocks` strategy:
    /// - profile a small sample of the data to prune unsuitable candidates,
    /// - then encode the same sample with all survivors and
    /// - return the encoder that produces the smallest output.
    ///
    /// `FastPFOR` is always preferred over `VarInt` when sizes are equal.
    #[must_use]
    pub fn auto_u32(values: &[u32]) -> IntegerEncoder {
        let enc = DataProfile::prune_candidates::<i32>(values);
        DataProfile::min_size_encoding_u32s(&enc, values)
    }

    /// Automatically select the best encoder for a `u64` stream.
    #[must_use]
    pub fn auto_u64(values: &[u64]) -> IntegerEncoder {
        let enc = DataProfile::prune_candidates::<i64>(values);
        DataProfile::min_size_encoding_u64s(&enc, values)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct FsstStringEncoder {
    pub symbol_lengths: IntegerEncoder,
    pub dict_lengths: IntegerEncoder,
}
