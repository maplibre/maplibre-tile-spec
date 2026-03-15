use crate::analyse::{Analyze, StatType};

/// Shared wrapper for values that may still be in the original (raw) format or
/// already parsed (but still columnar).
/// Used by: `Id`, `Geometry`, `Property`, and eventually - `SharedDictItem`
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum EncDec<Raw, Parsed> {
    Raw(Raw),       // Raw
    Parsed(Parsed), // Parsed
}

impl<Raw, Parsed> From<Raw> for EncDec<Raw, Parsed> {
    fn from(raw: Raw) -> Self {
        Self::Raw(raw)
    }
}

impl<Raw, Parsed> Analyze for EncDec<Raw, Parsed>
where
    Raw: Analyze,
    Parsed: Analyze,
{
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(encoded) => encoded.collect_statistic(stat),
            Self::Parsed(decoded) => decoded.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(crate::v01::StreamMeta)) {
        match self {
            Self::Raw(encoded) => encoded.for_each_stream(cb),
            Self::Parsed(decoded) => decoded.for_each_stream(cb),
        }
    }
}
