use std::mem;

use crate::analyse::{Analyze, StatType};
use crate::v01::StreamMeta;
use crate::{Decoder, MltError, MltResult};

pub trait Decode<Parsed>: Sized {
    fn decode(self, decoder: &mut Decoder) -> MltResult<Parsed>;
}

mod sealed {
    pub trait Sealed {}
}

/// Type-state marker for [`Layer01`](crate::v01::Layer01) and related column wrappers.
///
/// Implementors determine how `(Raw, Parsed)` column pairs are stored:
/// - [`Lazy`] stores an [`LazyParsed<Raw, Parsed>`] enum that can be in `Raw`, `Parsed`, or `ParsingFailed` state.
/// - [`Parsed`] stores only `Parsed`, giving zero-cost infallible field access.
pub trait DecodeState: sealed::Sealed {
    type LazyOrParsed<Raw, Parsed>;
}

/// Lazy state: individual columns may still be raw or already decoded.
///
/// This is the default state produced by [`Layer01::from_bytes`](crate::v01::Layer01::from_bytes).
/// Columns can be decoded in place (via `decode_id`, `decode_geometry`, etc.) or
/// all at once by calling [`Layer01::decode_all`](crate::v01::Layer01::decode_all), which
/// consumes `self` and returns a [`Layer01<Parsed>`](crate::v01::Layer01).
#[derive(Debug, Clone, PartialEq)]
pub struct Lazy;

/// Fully-decoded state: all columns hold their parsed values directly.
///
/// A `Layer01<Parsed>` is produced by [`Layer01::decode_all`](crate::v01::Layer01::decode_all).
/// Its fields (`id`, `geometry`, `properties`) are the parsed types themselves — no
/// wrapping enum, no `Result`, just plain field access.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed;

impl sealed::Sealed for Lazy {}
impl sealed::Sealed for Parsed {}

impl DecodeState for Lazy {
    type LazyOrParsed<Raw, Parsed> = LazyParsed<Raw, Parsed>;
}
impl DecodeState for Parsed {
    /// In the decoded state the column IS the parsed value — no enum wrapper.
    type LazyOrParsed<Raw, Parsed> = Parsed;
}

/// Shared wrapper for values that may still be in the original (raw) format or
/// already parsed (but still columnar).
/// Used by: `Id`, `Geometry`, `Property`, and eventually - `SharedDictItem`
#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum LazyParsed<Raw, Parsed> {
    Raw(Raw),
    Parsed(Parsed),
    ParsingFailed,
}

impl<Raw: Decode<Parsed>, Parsed> LazyParsed<Raw, Parsed> {
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

impl<Raw: Analyze, Parsed: Analyze> Analyze for LazyParsed<Raw, Parsed> {
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
