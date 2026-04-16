use crate::encoder::stream::logical::LogicalEncoder;
use crate::encoder::stream::physical::PhysicalEncoder;

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
}
