use super::optimizer::DataProfile;
use super::physical::PhysicalEncoder;
use crate::v01::LogicalEncoder;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct IntEncoder {
    pub(crate) logical: LogicalEncoder,
    pub(crate) physical: PhysicalEncoder,
}

impl IntEncoder {
    #[must_use]
    pub const fn new(logical: LogicalEncoder, physical: PhysicalEncoder) -> Self {
        Self { logical, physical }
    }

    #[must_use]
    pub fn delta_fastpfor() -> Self {
        Self::new(LogicalEncoder::Delta, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn delta_rle_fastpfor() -> Self {
        Self::new(LogicalEncoder::DeltaRle, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn delta_rle_varint() -> Self {
        Self::new(LogicalEncoder::DeltaRle, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn delta_varint() -> Self {
        Self::new(LogicalEncoder::Delta, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn fastpfor() -> Self {
        Self::new(LogicalEncoder::None, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn plain() -> Self {
        Self::new(LogicalEncoder::None, PhysicalEncoder::None)
    }
    #[must_use]
    pub fn rle_fastpfor() -> Self {
        Self::new(LogicalEncoder::Rle, PhysicalEncoder::FastPFOR)
    }
    #[must_use]
    pub fn rle_varint() -> Self {
        Self::new(LogicalEncoder::Rle, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn varint() -> Self {
        Self::new(LogicalEncoder::None, PhysicalEncoder::VarInt)
    }
    #[must_use]
    pub fn varint_with(logical: LogicalEncoder) -> Self {
        Self::new(logical, PhysicalEncoder::VarInt)
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
    pub fn auto_u32(values: &[u32]) -> Self {
        let enc = DataProfile::prune_candidates::<i32>(values);
        DataProfile::compete_u32(&enc, values)
    }

    /// Automatically select the best encoder for a `u64` stream.
    #[must_use]
    pub fn auto_u64(values: &[u64]) -> Self {
        let enc = DataProfile::prune_candidates::<i64>(values);
        DataProfile::compete_u64(&enc, values)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct FsstStrEncoder {
    pub(crate) symbol_lengths: IntEncoder,
    pub(crate) dict_lengths: IntEncoder,
}
