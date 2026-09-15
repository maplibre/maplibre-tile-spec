#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PhysicalEncoder {
    None,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPFOR`.
    VarInt,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    ///
    /// Does not support u64/i64 integers
    FastPFOR,
    /// Every value in the same `1..=32` bits, which a leading payload byte names.
    ///
    /// Only v2 can express it, so the v1 round-trip properties never generate it.
    #[cfg(feature = "unstable-v2")]
    #[cfg_attr(test, proptest(skip))]
    BitPacked,
}
