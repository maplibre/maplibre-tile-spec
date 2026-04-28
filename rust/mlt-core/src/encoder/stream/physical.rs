use crate::decoder::PhysicalEncoding;
use crate::{MltError, MltResult};

impl PhysicalEncoding {
    pub fn parse(value: u8) -> MltResult<Self> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalEncoding(value)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PhysicalEncoder {
    None,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPFOR`.
    VarInt,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    ///
    /// Does not support u64/i64 integers
    FastPFOR,
}
