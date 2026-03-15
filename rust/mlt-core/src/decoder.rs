use crate::MltError;

/// Default memory budget: 10 MiB.
const DEFAULT_MAX_BYTES: u32 = 10 * 1024 * 1024;

/// Stateful decoder that enforces a per-tile memory budget.
///
/// Pass a `Decoder` to every `raw.decode()` / `into_tile()` call. Each method
/// calls [`Decoder::consume`] before performing heap allocations, so the
/// total heap used never exceeds `max_bytes` (in bytes).
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoder {
    /// Keep track of the memory used when decoding a tile: raw->parsed transition
    budget: MemBudget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemBudget {
    /// Hard ceiling: total decoded bytes may not exceed this value.
    pub max_bytes: u32,
    /// Running total of used bytes so far.
    pub bytes_used: u32,
}

impl Default for Decoder {
    /// Create a decoder with the default 10 MiB memory budget.
    fn default() -> Self {
        Self::with_max_size(DEFAULT_MAX_BYTES)
    }
}

impl Decoder {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget {
                max_bytes,
                bytes_used: 0,
            },
        }
    }

    pub fn consume(&mut self, size: u32) -> Result<(), MltError> {
        self.budget.consume(size)
    }
}

impl MemBudget {
    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    pub fn consume(&mut self, size: u32) -> Result<(), MltError> {
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
