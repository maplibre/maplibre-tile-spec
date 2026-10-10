use std::ops::Deref;

use crate::decoder::{ColumnDecl, RawPresence, RawStream};
use crate::utils::PresentValues;
use crate::utils::analyze::AnalyzeViaDeref;
use crate::{DecodeState, Lazy};

/// ID column representation, parameterized by decode state.
///
/// - `Id<'a>` / `Id<'a, Lazy>` - either raw bytes or decoded, in a [`crate::LazyParsed`] enum.
/// - `Id<'a, Parsed>` - decoded [`ParsedId`] directly (no enum wrapper).
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

impl RawId<'_> {
    /// What this column declares, the id width included.
    pub(crate) fn decl(&self) -> ColumnDecl {
        ColumnDecl::Id {
            wide: matches!(self.value, RawIdValue::Id64(_)),
            optional: self.presence.is_optional(),
        }
    }
}

/// Decoded ID column.
///
/// A transparent type over [`PresentValues<'a, u64>`]. All feature-access methods
/// (`get`, `feature_count`, `dense_values`, `materialize`, `is_present`) are
/// available via auto-deref.
///
/// The lifetime `'a` allows zero-copy decoding when the inner bitvector borrows
/// from the source bytes. When the presence stream is RLE-decompressed, the inner
/// `Cow` becomes owned and no longer borrows from the input.
// TODO: consider converting ParsedId to an enum with u32 vs u64 for performance
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedId<'a>(pub(crate) PresentValues<'a, u64>);

impl<'a> Deref for ParsedId<'a> {
    type Target = PresentValues<'a, u64>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AnalyzeViaDeref for ParsedId<'_> {}
