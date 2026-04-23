use std::fmt::{Debug, Formatter};
use std::ops::Deref;

use crate::decoder::{RawPresence, RawStream};
use crate::utils::analyze::AnalyzeViaDeref;
use crate::utils::{OptSeqOpt, Presence};
use crate::{DecodeState, Lazy};

/// ID column representation, parameterized by decode state.
///
/// - `Id<'a>` / `Id<'a, Lazy>` — either raw bytes or decoded, in a [`crate::LazyParsed`] enum.
/// - `Id<'a, Parsed>` — decoded [`ParsedId`] directly (no enum wrapper).
pub type Id<'a, S = Lazy> = <S as DecodeState>::LazyOrParsed<RawId<'a>, ParsedId<'a>>;

/// Unparsed ID data as read directly from the tile (borrows from input bytes)
#[derive(Debug, PartialEq, Clone)]
pub struct RawId<'a> {
    pub(crate) presence: RawPresence<'a>,
    pub(crate) value: RawIdValue<'a>,
}

/// A sequence of raw ID values, either 32-bit or 64-bit unsigned integers
#[derive(Debug, PartialEq, Clone)]
pub enum RawIdValue<'a> {
    Id32(RawStream<'a>),
    Id64(RawStream<'a>),
}

/// Decoded ID column.
///
/// A transparent type over [`Presence<'a, u64>`]. All feature-access methods
/// (`get`, `feature_count`, `dense_values`, `materialize`, `is_present`) are
/// available via auto-deref.
///
/// The lifetime `'a` ties the inner bitvector to the source bytes for zero-copy
/// decoding; when the stream is RLE-decompressed the data is owned and `'a` is `'static`.
// TODO: consider converting ParsedId to an enum with u32 vs u64 for performance
#[derive(Clone, PartialEq)]
pub struct ParsedId<'a>(pub(crate) Presence<'a, u64>);

impl<'a> Deref for ParsedId<'a> {
    type Target = Presence<'a, u64>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AnalyzeViaDeref for ParsedId<'_> {}

impl Debug for ParsedId<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParsedId({:?})", &OptSeqOpt(Some(&self.materialize())))
    }
}
