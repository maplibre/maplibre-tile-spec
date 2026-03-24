use std::mem;

use crate::analyse::{Analyze, StatType};
use crate::v01::StreamMeta;
use crate::{Decoder, MltError, MltResult};

pub trait Decode<Parsed>: Sized {
    fn decode(self, decoder: &mut Decoder) -> MltResult<Parsed>;
}

/// Shared wrapper for values that may still be in the original (raw) format or
/// already parsed (but still columnar).
/// Used by: `Id`, `Geometry`, `Property`, and eventually - `SharedDictItem`
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum EncDec<Raw, Parsed> {
    Raw(Raw),
    Parsed(Parsed),
    ParsingFailed,
}

impl<Raw: Decode<Parsed>, Parsed> EncDec<Raw, Parsed> {
    /// Decode in place, replacing the raw value with the parsed result.
    pub fn decode(&mut self, decoder: &mut Decoder) -> MltResult<&mut Parsed> {
        match self {
            Self::Parsed(v) => Ok(v),
            Self::Raw(_) => {
                let Self::Raw(raw) = mem::replace(self, Self::ParsingFailed) else {
                    unreachable!();
                };
                *self = Self::Parsed(raw.decode(decoder)?);
                let Self::Parsed(v) = self else {
                    unreachable!()
                };
                Ok(v)
            }
            Self::ParsingFailed => Err(MltError::PriorParseFailure),
        }
    }

    /// Consume and return the parsed value, decoding if currently raw.
    pub fn into_parsed(self, decoder: &mut Decoder) -> MltResult<Parsed> {
        match self {
            Self::Parsed(v) => Ok(v),
            Self::Raw(raw) => raw.decode(decoder),
            Self::ParsingFailed => Err(MltError::PriorParseFailure),
        }
    }

    pub fn as_parsed(&self) -> MltResult<&Parsed> {
        match self {
            Self::Parsed(v) => Ok(v),
            Self::Raw(_) => Err(MltError::NotDecoded("enc_dec value")), // TODO: I wonder if the str can be of the type name?
            Self::ParsingFailed => Err(MltError::PriorParseFailure),
        }
    }
}

impl<Raw: Analyze, Parsed: Analyze> Analyze for EncDec<Raw, Parsed> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(encoded) => encoded.collect_statistic(stat),
            Self::Parsed(decoded) => decoded.collect_statistic(stat),
            Self::ParsingFailed => 0,
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Raw(encoded) => encoded.for_each_stream(cb),
            Self::Parsed(decoded) => decoded.for_each_stream(cb),
            Self::ParsingFailed => {}
        }
    }
}
