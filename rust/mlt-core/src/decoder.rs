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
    /// Hard ceiling: total decoded bytes may not exceed this value.
    pub max_bytes: u32,
    /// Running total of the bytes we expect will be used in the later pass
    pub bytes_reserved: u32,
    /// Running total of decoded bytes charged so far.
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
            max_bytes,
            bytes_reserved: 0,
            bytes_used: 0,
        }
    }

    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    pub fn consume(&mut self, size: u32) -> Result<(), MltError> {
        Self::add(&mut self.bytes_used, size, self.max_bytes)
    }

    /// Take `size` bytes from the reservation budget.
    /// Used when we know we will need to allocate in a later pass, but don't want to charge the budget yet.
    #[inline]
    pub fn reserve(&mut self, size: u32) -> Result<(), MltError> {
        Self::add(&mut self.bytes_reserved, size, self.max_bytes)
    }

    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    fn add(accumulator: &mut u32, size: u32, max_bytes: u32) -> Result<(), MltError> {
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
