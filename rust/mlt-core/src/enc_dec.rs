use crate::analyse::{Analyze, StatType};

/// Shared wrapper for values that may still be encoded or already decoded.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum EncDec<Encoded, Decoded> {
    Encoded(Encoded),
    Decoded(Decoded),
}

impl<Encoded, Decoded> From<Encoded> for EncDec<Encoded, Decoded> {
    fn from(encoded: Encoded) -> Self {
        Self::Encoded(encoded)
    }
}

impl<Encoded, Decoded> Analyze for EncDec<Encoded, Decoded>
where
    Encoded: Analyze,
    Decoded: Analyze,
{
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(encoded) => encoded.collect_statistic(stat),
            Self::Decoded(decoded) => decoded.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(crate::v01::StreamMeta)) {
        match self {
            Self::Encoded(encoded) => encoded.for_each_stream(cb),
            Self::Decoded(decoded) => decoded.for_each_stream(cb),
        }
    }
}
