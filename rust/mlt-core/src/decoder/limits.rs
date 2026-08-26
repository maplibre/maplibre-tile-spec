//! Memory-limit enforcement shared by every layer format.

use crate::errors::AsMltError as _;
use crate::{Layer, MltError, MltResult, ParsedLayer};

/// Default memory budget: 20 MiB.
const DEFAULT_MAX_BYTES: u32 = 20 * 1024 * 1024;

/// Stateful decoder that enforces a per-tile memory budget during decoding.
///
/// Pass a `Decoder` to every `raw.decode()` / `into_tile()` call and to `from_bytes`-style parsers.
/// Each method charges the budget before performing heap allocations, so the total heap used never exceeds `max_bytes` (in bytes).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoder {
    /// Keep track of the memory used when decoding a tile: raw->parsed transition
    budget: MemBudget,
    /// Reusable scratch buffer for the physical u32 decode pass.
    /// Held here so its heap allocation is reused across streams without extra cost.
    pub(crate) buffer_u32: Vec<u32>,
    /// Reusable scratch buffer for the physical u64 decode pass.
    /// Held here so its heap allocation is reused across streams without extra cost.
    pub(crate) buffer_u64: Vec<u64>,
}

impl Decoder {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget::with_max_size(max_bytes),
            ..Default::default()
        }
    }

    pub fn decode_all<'a>(
        &mut self,
        layers: impl IntoIterator<Item = Layer<'a>>,
    ) -> MltResult<Vec<ParsedLayer<'a>>> {
        layers
            .into_iter()
            .map(|l| l.decode_all(self))
            .collect::<MltResult<_>>()
    }

    /// Allocate a `Vec<T>` with the given capacity, charging the decoder's budget for `capacity * size_of::<T>()` bytes.
    /// Use this instead of `Vec::with_capacity` in decode paths.
    #[inline]
    pub(crate) fn alloc<T>(&mut self, capacity: usize) -> MltResult<Vec<T>> {
        let bytes = capacity.checked_mul(size_of::<T>()).or_overflow()?;
        let bytes_u32 = u32::try_from(bytes).or_overflow()?;
        self.budget.consume(bytes_u32)?;
        Ok(Vec::with_capacity(capacity))
    }

    /// Charge the budget for `size` raw bytes.
    /// Prefer [`consume_items`][Self::consume_items] when charging for a known-type collection.
    #[inline]
    pub(crate) fn consume(&mut self, size: u32) -> MltResult<()> {
        self.budget.consume(size)
    }

    /// Charge the budget for `count` items of type `T` (`count * size_of::<T>()` bytes).
    #[inline]
    pub(crate) fn consume_items<T>(&mut self, count: usize) -> MltResult<()> {
        let bytes = count.checked_mul(size_of::<T>()).or_overflow()?;
        self.budget.consume(u32::try_from(bytes).or_overflow()?)
    }

    #[inline]
    pub(crate) fn adjust(&mut self, adjustment: u32) {
        self.budget.adjust(adjustment);
    }

    /// Return the unused portion of a pre-charged allocation budget.
    ///
    /// Call this after fully populating a `Vec<T>` that was pre-allocated with [`Decoder::alloc`],
    /// passing the same `alloc_size` that was given to `alloc`.
    ///
    /// Returns an error if the vector grew beyond `alloc_size` (malformed input caused more items
    /// than declared). Subtracts `(alloc_size - buf.len()) * size_of::<T>()` from the budget.
    #[inline]
    pub(crate) fn adjust_alloc<T>(&mut self, buf: &[T], alloc_size: usize) -> MltResult<()> {
        if buf.len() > alloc_size {
            return Err(MltError::InvalidDecodingStreamSize(buf.len(), alloc_size));
        }
        // Return the unused portion of the pre-charged budget.
        let unused = (alloc_size - buf.len()) * size_of::<T>();
        // unused fits in u32: it's at most alloc_size * size_of::<T>(), which was checked to fit
        // in u32 when alloc() was called. Using saturating_cast to avoid a fallible conversion.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "unused <= alloc_size * size_of::<T>() which was verified to fit in u32 by alloc()"
        )]
        self.budget.adjust(unused as u32);
        Ok(())
    }

    #[must_use]
    pub fn consumed(&self) -> u32 {
        self.budget.consumed()
    }

    /// Reset the memory budget to zero, keeping scratch buffers allocated.
    ///
    /// Call this between tiles when reusing a single `Decoder` for multiple
    /// decodes - the per-tile budget is enforced fresh, but the internal
    /// `buffer_u32` / `buffer_u64` scratch space is retained so it doesn't
    /// need to be re-allocated.
    ///
    /// # Safety / correctness precondition
    ///
    /// Only call this after dropping any decoded allocations returned from the
    /// previous tile. Resetting the budget while earlier decoded outputs are
    /// still alive makes the budget enforceable only per-tile and can bypass
    /// the stronger guarantee that total live heap tracked by this decoder
    /// never exceeds the configured maximum.
    pub fn reset_budget(&mut self) {
        self.budget.reset();
    }
}

/// Stateful parser that enforces a memory budget during parsing (binary -> raw structures).
///
/// The parse chain reserves memory before allocations so total heap stays within the limit.
///
/// ```
/// use mlt_core::Parser;
///
/// # let bytes: &[u8] = &[];
/// let mut parser = Parser::default();
/// let layers = parser.parse_layers(bytes).expect("parse");
///
/// // Or with a custom limit:
/// let mut parser = Parser::with_max_size(64 * 1024 * 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parser {
    budget: MemBudget,
}

impl Parser {
    /// Create a parser with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget::with_max_size(max_bytes),
        }
    }

    /// Parse a sequence of binary layers, reserving decoded memory against this parser's budget.
    pub fn parse_layers<'a>(&mut self, mut input: &'a [u8]) -> MltResult<Vec<Layer<'a>>> {
        let mut result = Vec::new();
        while !input.is_empty() {
            let layer;
            (input, layer) = Layer::from_bytes(input, self)?;
            result.push(layer);
        }
        Ok(result)
    }

    /// Reserve `size` bytes from the parse budget. Used internally by the parse chain.
    #[inline]
    pub(crate) fn reserve(&mut self, size: u32) -> MltResult<()> {
        self.budget.consume(size)
    }

    #[must_use]
    pub fn reserved(&self) -> u32 {
        self.budget.consumed()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemBudget {
    /// Hard ceiling: total decoded bytes may not exceed this value.
    pub max_bytes: u32,
    /// Running total of used bytes so far.
    pub bytes_used: u32,
}

impl Default for MemBudget {
    /// Create a decoder with the default 10 MiB memory budget.
    fn default() -> Self {
        Self::with_max_size(DEFAULT_MAX_BYTES)
    }
}

impl MemBudget {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    fn with_max_size(max_bytes: u32) -> Self {
        Self {
            max_bytes,
            bytes_used: 0,
        }
    }

    /// Adjust previous consumption by `- adjustment` bytes.  Will panic if used incorrectly.
    #[inline]
    fn adjust(&mut self, adjustment: u32) {
        self.bytes_used = self.bytes_used.checked_sub(adjustment).unwrap();
    }

    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    fn consume(&mut self, size: u32) -> MltResult<()> {
        let accumulator = &mut self.bytes_used;
        let max_bytes = self.max_bytes;
        if let Some(new_value) = accumulator.checked_add(size).filter(|&v| v <= max_bytes) {
            *accumulator = new_value;
            Ok(())
        } else {
            Err(MltError::MemoryLimitExceeded {
                limit: max_bytes,
                used: *accumulator,
                requested: size,
            })
        }
    }

    fn consumed(&self) -> u32 {
        self.bytes_used
    }

    /// Reset tracked usage for a new decode window.
    ///
    /// Callers must ensure that allocations accounted for by the previous
    /// window are no longer live before resetting.
    fn reset(&mut self) {
        self.bytes_used = 0;
    }
}
