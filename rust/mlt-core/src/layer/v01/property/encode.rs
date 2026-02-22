use crate::v01::PhysicalCodec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceSteamStrategy {
    Present,
    Absent,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicalCodecStrategy {
    None,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    /// But currently limited to 32-bit integer.
    FastPFOR,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the codec is easier to implement compared to `FastPFOR`.
    VarInt,
    // only implemented logical techniques
}

impl From<PhysicalCodecStrategy> for PhysicalCodec {
    fn from(value: PhysicalCodecStrategy) -> Self {
        match value {
            PhysicalCodecStrategy::None => PhysicalCodec::None,
            PhysicalCodecStrategy::FastPFOR => PhysicalCodec::FastPFOR,
            PhysicalCodecStrategy::VarInt => PhysicalCodec::VarInt,
        }
    }
}
