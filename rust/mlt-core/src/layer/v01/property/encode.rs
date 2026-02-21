use crate::v01::{LogicalCodec, PhysicalCodec, RleMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceSteamStrategy {
    Present,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalCodecStrategy {
    None,
    Delta,
    ComponentwiseDelta,
    Rle,
    // only implemented logical techniques
}
impl From<LogicalCodecStrategy> for LogicalCodec {
    fn from(value: LogicalCodecStrategy) -> Self {
        match value {
            LogicalCodecStrategy::None => Self::None,
            LogicalCodecStrategy::Delta => Self::Delta,
            LogicalCodecStrategy::ComponentwiseDelta => Self::ComponentwiseDelta,
            LogicalCodecStrategy::Rle => Self::Rle(RleMeta {
                runs: 0,
                num_rle_values: 0,
            }),
        }
    }
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
