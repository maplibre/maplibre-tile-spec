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
    /// Running total of decoded bytes charged so far.
    pub used_bytes: u32,
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
            used_bytes: 0,
        }
    }

    /// Reserve `size` bytes of budget.
    ///
    /// Returns `Err(MltError::MemoryLimitExceeded)` if adding `size` to
    /// `used_bytes` would exceed `max_bytes`.  On success `used_bytes` is
    /// increased by `size`.
    ///
    /// Prefer calling this *before* allocating.  Where the output size is
    /// only known *after* the allocation (e.g. FSST decompression), it is
    /// acceptable to allocate first and then call `consume` to record the
    /// actual size; a budget check will still prevent unbounded total usage
    /// as long as every allocation is eventually charged.
    pub fn consume(&mut self, size: u32) -> Result<(), MltError> {
        let new_used = self.used_bytes.saturating_add(size);
        if new_used > self.max_bytes {
            return Err(MltError::MemoryLimitExceeded {
                limit: self.max_bytes,
                used: self.used_bytes,
                requested: size,
            });
        }
        self.used_bytes = new_used;
        Ok(())
    }
}
