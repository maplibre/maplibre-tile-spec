use crate::MltError;
use crate::utils::AsUsize;

/// Default memory budget: 10 MiB.
const DEFAULT_MAX_BYTES: u32 = 10 * 1024 * 1024;

/// Stateful decoder that enforces a per-tile memory budget during decoding.
///
/// Pass a `Decoder` to every `raw.decode()` / `into_tile()` call and to
/// `from_bytes`-style parsers. Each method calls [`Decoder::consume`] before
/// performing heap allocations, so the total heap used never exceeds `max_bytes`
/// (in bytes).
///
/// ```
/// use mlt_core::Decoder;
///
/// // Default: 10 MiB budget.
/// let mut dec = Decoder::default();
///
/// // Custom budget.
/// let mut dec = Decoder::with_max_size(64 * 1024 * 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoder {
    /// Keep track of the memory used when decoding a tile: raw->parsed transition
    budget: MemBudget,
}

impl Decoder {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget::with_max_size(max_bytes),
        }
    }

    #[inline]
    pub fn alloc<T>(&mut self, size: u32) -> Result<Vec<T>, MltError> {
        self.budget.consume(size)?;
        Ok(Vec::with_capacity(size.as_usize()))
    }

    pub fn consume(&mut self, size: u32) -> Result<(), MltError> {
        self.budget.consume(size)
    }
}

/// Stateful parser that enforces a memory budget during parsing (binary → raw structures).
///
/// Pass a `Parser` to [`parse_layers`](crate::parse_layers) and to `from_bytes`-style APIs.
/// The parse chain calls [`Parser::reserve`] before allocations so total heap stays within the limit.
///
/// ```
/// use mlt_core::{Parser, parse_layers};
///
/// # let bytes: &[u8] = &[];
/// let mut parser = Parser::default();
/// let layers = parse_layers(bytes, &mut parser).expect("parse");
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

    /// Reserve `size` bytes from the parse budget. Used internally by the parse chain.
    #[inline]
    pub(crate) fn reserve(&mut self, size: u32) -> Result<(), MltError> {
        self.budget.consume(size)
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

    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    fn consume(&mut self, size: u32) -> Result<(), MltError> {
        let accumulator = &mut self.bytes_used;
        let max_bytes = self.max_bytes;
        if let Some(new_value) = accumulator
            .checked_add(size)
            .and_then(|v| if v > max_bytes { None } else { Some(v) })
        {
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
}
