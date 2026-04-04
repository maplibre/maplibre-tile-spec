use std::io;

use integer_encoding::VarIntWriter as _;

use crate::encoder::{
    EncoderConfig, ExplicitEncoder, IdWidth, IntEncoder, StrEncoding, VertexBufferType,
};
use crate::{MltError, MltResult};

/// Stateful encoder that accumulates encoded layer bytes and provides
/// reusable scratch buffers to avoid repeated allocations during encoding.
///
/// Mirrors the [`Decoder`](crate::Decoder) pattern: the struct holds both the
/// output buffers and reusable intermediate buffers that grow to the required
/// size on first use and are then reused across streams without re-allocating.
///
/// # Buffer layout
///
/// The MLT layer wire format is:
///
/// ```text
/// [varint(body_len + 1)] [tag = 1]
/// [name: string] [extent: varint] [column_count: varint]   <- hdr
/// [col_type₁] [col_type₂] … [col_typeN]                    <- meta
/// [col₁ stream data] [col₂ stream data] … [colN stream data] <- data
/// ```
///
/// The three sections are accumulated into separate buffers so they can be
/// combined at the end *without* any in-place insertion or extra copies:
///
/// * [`hdr`] – layer header (name, extent, `column_count`).
/// * [`meta`] – column-type bytes (one byte + optional name per column).
/// * [`data`] – encoded stream data; also the target of [`impl Write`].
///
/// # Sort-strategy trialing
///
/// Create one `Encoder` per sort-strategy trial, encode the layer into it,
/// and keep the one whose `total_len()` is smallest:
///
/// ```rust,ignore
/// let mut best: Option<Encoder> = None;
/// for strategy in strategies {
///     let mut enc = Encoder::new(cfg);
///     layer.write_to(&mut enc)?;
///     if best.as_ref().is_none_or(|b| enc.total_len() < b.total_len()) {
///         best = Some(enc);
///     }
/// }
/// return best.unwrap().into_layer_bytes();
/// ```
///
/// # Stream-level encoding alternatives
///
/// Use [`start_alternatives`] / [`finish_alternative`] to try multiple
/// encodings for a single stream and keep the shortest, all within the
/// same `data` buffer without extra allocations:
///
/// ```rust,ignore
/// enc.start_alternatives();
/// write_stream_as_varint(data, &mut enc)?;
/// enc.finish_alternative();         // commits the VarInt candidate
/// write_stream_as_fastpfor(data, &mut enc)?;
/// enc.finish_alternative();         // keeps whichever was shorter
/// ```
///
/// [`hdr`]: Encoder::hdr
/// [`meta`]: Encoder::meta
/// [`data`]: Encoder::data
/// [`impl Write`]: Encoder#impl-Write
/// [`start_alternatives`]: Encoder::start_alternatives
/// [`finish_alternative`]: Encoder::end_alternative
#[derive(Debug, Default)]
pub struct Encoder {
    /// Encoding configuration: controls which optimisation strategies are tried
    /// (sort orders, compression algorithms, etc.).
    ///
    /// Set once at construction time via [`Encoder::new`]; propagated
    /// automatically to all sub-encoders so individual encode methods do not
    /// need a separate `cfg` argument.
    pub cfg: EncoderConfig,

    /// When [`Some`], property / ID / geometry encoders use [`ExplicitEncoder`]
    /// callbacks instead of trying candidate encodings. When [`None`], the
    /// automatic optimization path runs.
    pub explicit: Option<ExplicitEncoder>,

    /// Layer header bytes: `name`, `extent`, `column_count`.
    ///
    /// Written to `hdr` via [`Encoder::write_header`].  This section comes
    /// first in the wire format and is never subject to alternatives.
    pub hdr: Vec<u8>,

    /// Column-type metadata bytes.
    ///
    /// Each column contributes one type byte (plus a name string for property
    /// columns).  Written by the `write_columns_meta_to` methods, which write
    /// directly to `enc.meta`.  This section comes second in the wire format
    /// and is never subject to alternatives (column types are fixed).
    pub meta: Vec<u8>,

    /// Encoded stream data.
    ///
    /// All stream counts, per-stream encoding-metadata bytes, and encoded
    /// data bytes land here via [`impl Write`].  This section comes last in
    /// the wire format and is where stream-level alternatives compete.
    ///
    /// [`impl Write`]: Encoder#impl-Write
    pub data: Vec<u8>,

    /// Layer columns written so far (geometry, optional ID, property columns).
    ///
    /// Incremented by each column encoder when it writes its column-type byte to
    /// [`meta`](Encoder::meta). [`write_header`](Encoder::write_header) uses this
    /// as the wire-format `column_count`.
    pub layer_column_count: u32,

    /// Reusable scratch buffer for intermediate `u32` values.
    ///
    /// Used for the logical-encoding step (e.g. delta or RLE transform) before
    /// physical compression writes the final bytes to `data`.
    #[expect(dead_code, reason = "reserved for stream-level in-place encoding")]
    pub(crate) tmp_u32: Vec<u32>,

    /// Reusable scratch buffer for intermediate `u64` values.
    ///
    /// Same role as `tmp_u32` but for `u64` streams.
    #[expect(dead_code, reason = "reserved for stream-level in-place encoding")]
    pub(crate) tmp_u64: Vec<u64>,

    /// Reusable scratch buffer for intermediate `u8` bytes.
    ///
    /// Used for multi-step byte transforms before the final bytes land in `data`.
    #[expect(dead_code, reason = "reserved for stream-level in-place encoding")]
    pub(crate) tmp_u8: Vec<u8>,

    // -----------------------------------------------------------------------
    // Alternatives state — a stack that supports nested competitions.
    //
    // Each entry: (start_in_data, best_candidate_size_so_far).
    //
    // Invariant between candidates at any level:
    //   data.len() == entry.0 + entry.1.unwrap_or(0)
    //
    // Empty stack ↔ no competition in progress.
    // -----------------------------------------------------------------------
    /// Stack of active encoding competitions, innermost last.
    ///
    /// Each entry is `(start, best_size)` where:
    /// - `start` — position in [`data`] where this competition began.
    /// - `best_size` — byte count of the best candidate so far (`None` until
    ///   the first [`finish_alternative`] call at this level).
    ///
    /// Empty while no [`start_alternatives`] / `finish_alternatives` session
    /// is in progress.
    ///
    /// [`data`]: Encoder::data
    /// [`start_alternatives`]: Encoder::start_alternatives
    /// [`finish_alternative`]: Encoder::end_alternative
    alt_stack: Vec<(usize, Option<usize>)>,
}

impl Encoder {
    /// Create a new encoder with the given [`EncoderConfig`].
    ///
    /// Use [`Encoder::default()`] when the default configuration is sufficient.
    #[inline]
    #[must_use]
    pub fn new(cfg: EncoderConfig) -> Self {
        Self {
            cfg,
            ..Self::default()
        }
    }

    /// Like [`Self::new`] but with [`Self::explicit`] set for deterministic encoding
    /// (tests, synthetics). Use with `StagedLayer01::encode_explicit`.
    #[inline]
    #[must_use]
    pub fn with_explicit(cfg: EncoderConfig, explicit: ExplicitEncoder) -> Self {
        Self {
            cfg,
            explicit: Some(explicit),
            ..Self::default()
        }
    }

    /// Record one layer column (geometry, ID, or property) after writing its
    /// column-type metadata to [`meta`](Encoder::meta).
    #[inline]
    pub(crate) fn increment_column_count(&mut self) {
        self.layer_column_count = self.layer_column_count.saturating_add(1);
    }

    /// Write the layer header (`name`, `extent`, `column_count`) to [`hdr`].
    ///
    /// `column_count` is `layer_column_count` —
    /// each column encoder must call `push_layer_column`
    /// when it commits a column to the wire format.
    ///
    /// Must be called exactly once per layer, after all column meta and data.
    ///
    /// [`hdr`]: Encoder::hdr
    pub fn write_header(&mut self, name: &str, extent: u32) -> MltResult<()> {
        debug_assert!(
            self.alt_stack.is_empty(),
            "write_header called with an open alternatives session"
        );
        let name_len = u32::try_from(name.len())?;
        self.hdr.write_varint(name_len).map_err(MltError::from)?;
        self.hdr.extend_from_slice(name.as_bytes());
        self.hdr.write_varint(extent).map_err(MltError::from)?;
        self.hdr
            .write_varint(self.layer_column_count)
            .map_err(MltError::from)?;
        Ok(())
    }

    /// When [`Self::explicit`] is [`Some`], returns the callback-chosen [`IntEncoder`].
    /// [`None`] means run automatic candidate selection for that stream.
    #[inline]
    pub(crate) fn get_int_encoder(
        &self,
        kind: &str,
        name: &str,
        subname: Option<&str>,
    ) -> Option<IntEncoder> {
        self.explicit
            .as_ref()
            .map(|e| (e.get_int_encoder)(kind, name, subname))
    }

    /// When [`Self::explicit`] is [`Some`], returns the callback-chosen [`StrEncoding`].
    /// [`None`] means run automatic string / shared-dict corpus selection.
    #[inline]
    pub(crate) fn get_str_encoding(&self, kind: &str, name: &str) -> Option<StrEncoding> {
        self.explicit
            .as_ref()
            .map(|e| (e.get_str_encoding)(kind, name))
    }

    /// Whether the explicit encoder forces a presence stream for an all-present column
    /// (or similar), per [`ExplicitEncoder::override_presence`].
    #[inline]
    pub(crate) fn override_presence(&self, kind: &str, name: &str, subname: Option<&str>) -> bool {
        self.explicit
            .as_ref()
            .is_some_and(|e| (e.override_presence)(kind, name, subname))
    }

    /// Applies `ExplicitEncoder::override_id_width` when an explicit encoder is active;
    /// otherwise returns `auto` unchanged.
    #[inline]
    #[allow(clippy::unused_self)]
    pub(crate) fn override_id_width(&self, auto: IdWidth) -> IdWidth {
        self.explicit
            .as_ref()
            .map_or(auto, |e| (e.override_id_width)(auto))
    }

    /// Pinned vertex layout when an explicit encoder is active.
    #[inline]
    #[allow(clippy::unused_self)]
    pub(crate) fn override_vertex_buffer_type(&self) -> Option<VertexBufferType> {
        self.explicit.as_ref().map(|e| e.vertex_buffer_type)
    }

    /// Total encoded bytes across all three sections (`hdr + meta + data`).
    ///
    /// Used to compare sort-strategy trials without assembling the final bytes.
    #[inline]
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.hdr.len() + self.meta.len() + self.data.len()
    }

    /// Concatenate `hdr + meta + data` into a single buffer **without** a
    /// tag/size prefix.
    ///
    /// Use this when the caller expects a raw layer body
    /// (e.g. [`Layer01::from_bytes`]) rather than a framed wire record.
    ///
    /// [`Layer01::from_bytes`]: crate::decoder::Layer01
    #[must_use]
    pub fn into_raw_bytes(mut self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.hdr.len() + self.meta.len() + self.data.len());
        out.append(&mut self.hdr);
        out.append(&mut self.meta);
        out.append(&mut self.data);
        out
    }

    /// Assemble the complete Tag-01 layer record:
    /// `[varint(body_len + 1)][tag = 1][hdr][meta][data]`.
    ///
    /// Consumes the encoder.  Call this on the winning sort-strategy trial.
    pub fn into_layer_bytes(mut self) -> MltResult<Vec<u8>> {
        debug_assert!(
            self.alt_stack.is_empty(),
            "into_layer_bytes called with an open alternatives session"
        );
        let body_len = self.hdr.len() + self.meta.len() + self.data.len();
        let size = u32::try_from(body_len + 1)?; // +1 for the tag byte
        let mut out = Vec::with_capacity(5 + 1 + body_len);
        out.write_varint(size).map_err(MltError::from)?;
        out.push(1_u8); // tag = 1
        out.append(&mut self.hdr);
        out.append(&mut self.meta);
        out.append(&mut self.data);
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Stream-level alternatives
    // -----------------------------------------------------------------------

    /// Begin a new encoding competition.
    ///
    /// Call **once** before the candidates loop, then write each candidate
    /// to `data` and call [`finish_alternative`] after each one.  The
    /// competition keeps the shortest candidate seen so far.  Between
    /// candidates `data` always ends at `start + best_size`, so the next
    /// write is appended naturally.
    ///
    /// Nesting is allowed: calling `start_alternatives` while a competition
    /// is already active pushes a new independent level onto the stack.
    ///
    /// End the competition with [`finish_alternatives`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// enc.start_alternatives();
    /// write_stream_as_varint(data, &mut enc)?;
    /// enc.finish_alternative();              // commits the VarInt candidate
    /// write_stream_as_fastpfor(data, &mut enc)?;
    /// enc.finish_alternative();              // commits FastPFOR candidate
    /// enc.finish_alternatives();             // keeps whichever was shorter
    /// ```
    ///
    /// [`finish_alternative`]: Encoder::end_alternative
    /// [`finish_alternatives`]: Encoder::finish_alternatives
    pub fn start_alternatives(&mut self) {
        self.alt_stack.push((self.data.len(), None));
    }

    /// Commit the current candidate at the innermost competition level.
    ///
    /// The candidate bytes (everything written to `data` since the last
    /// [`finish_alternative`] or [`start_alternatives`] at this level) are
    /// compared against the current best.  The shorter one is kept; ties
    /// preserve the earlier candidate (strict `<`).
    ///
    /// After the call `data.len() == level_start + best_size`, so the next
    /// candidate starts at `data.len()`.
    ///
    /// # Panics
    ///
    /// Panics if called outside a [`start_alternatives`] / [`finish_alternatives`] pair.
    ///
    /// [`finish_alternative`]: Encoder::end_alternative
    /// [`start_alternatives`]: Encoder::start_alternatives
    /// [`finish_alternatives`]: Encoder::finish_alternatives
    pub fn end_alternative(&mut self) {
        assert!(
            !self.alt_stack.is_empty(),
            "finish_alternative called outside a start_alternatives / finish_alternatives pair"
        );
        let (data, stack) = (&mut self.data, &mut self.alt_stack);
        let level = stack.last_mut().unwrap();
        Self::close_candidate(data, level.0, &mut level.1);
    }

    /// Commit any pending candidate and end the innermost competition.
    ///
    /// If bytes were written since the last [`finish_alternative`] call they
    /// are evaluated as a final candidate before the level is popped.
    /// If every candidate was already committed via [`finish_alternative`]
    /// this is a cheap stack-pop with no `data` changes.
    ///
    /// # Panics
    ///
    /// Panics if called outside a [`start_alternatives`] / [`finish_alternatives`] pair.
    ///
    /// [`finish_alternative`]: Encoder::end_alternative
    /// [`start_alternatives`]: Encoder::start_alternatives
    /// [`finish_alternatives`]: Encoder::finish_alternatives
    pub fn finish_alternatives(&mut self) {
        assert!(
            !self.alt_stack.is_empty(),
            "finish_alternatives called outside a start_alternatives / finish_alternatives pair"
        );
        {
            let (data, stack) = (&mut self.data, &mut self.alt_stack);
            let level = stack.last_mut().unwrap();
            let (start, best_size) = *level;
            let pending = data.len() - (start + best_size.unwrap_or(0));
            // Only evaluate a pending candidate if bytes were actually written
            // since the last finish_alternative() call (or since
            // start_alternatives() if this is the sole candidate).
            // pending == 0 with an existing best means all candidates were
            // already committed; just pop.
            if pending > 0 || best_size.is_none() {
                Self::close_candidate(data, level.0, &mut level.1);
            }
        }
        self.alt_stack.pop();
    }

    /// Shared compare-and-keep logic used by both `finish_alternative` and
    /// `finish_alternatives`.
    ///
    /// Compares the bytes written since the last committed candidate against
    /// `best_size`.  Keeps the shorter one (ties keep the existing best).
    fn close_candidate(data: &mut Vec<u8>, start: usize, best_size: &mut Option<usize>) {
        let best_end = start + best_size.unwrap_or(0);
        let cand_size = data.len() - best_end;
        if best_size.is_none_or(|prev| cand_size < prev) {
            // New best: shift to start (no-op on the very first candidate).
            if best_size.is_some() {
                data.copy_within(best_end..best_end + cand_size, start);
            }
            data.truncate(start + cand_size);
            *best_size = Some(cand_size);
        } else {
            // Not an improvement: discard.
            data.truncate(best_end);
        }
    }
}

/// Writes bytes to [`Encoder::data`].
///
/// This blanket implementation makes `Encoder` compatible with all
/// `BinarySerializer`, `VarIntWriter`, and other `Write`-based utilities so that
/// stream-data methods do not need a separate code path.
impl io::Write for Encoder {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.data.write_all(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: directly extend `enc.data` with raw bytes (simulates a stream write).
    fn push(enc: &mut Encoder, bytes: &[u8]) {
        enc.data.extend_from_slice(bytes);
    }

    // ── basic single-level behaviour ──────────────────────────────────────

    /// The shortest candidate wins.
    #[test]
    fn alternatives_keeps_shortest() {
        let mut enc = Encoder::default();
        push(&mut enc, b"prefix");

        enc.start_alternatives();
        push(&mut enc, b"longer"); // 6 bytes
        enc.end_alternative();
        push(&mut enc, b"ab"); // 2 bytes — shortest
        enc.end_alternative();
        push(&mut enc, b"xyz"); // 3 bytes
        enc.end_alternative();
        enc.finish_alternatives();

        assert_eq!(enc.data, b"prefixab");
    }

    /// On a tie the first candidate is kept (strict `<`, not `<=`).
    #[test]
    fn alternatives_tie_keeps_first() {
        let mut enc = Encoder::default();

        enc.start_alternatives();
        push(&mut enc, b"aaa"); // 3 bytes
        enc.end_alternative();
        push(&mut enc, b"bbb"); // 3 bytes — equal, not strictly shorter
        enc.finish_alternatives();

        assert_eq!(enc.data, b"aaa");
    }

    /// A single candidate is unconditionally the winner.
    #[test]
    fn alternatives_single_candidate() {
        let mut enc = Encoder::default();

        enc.start_alternatives();
        push(&mut enc, b"only");
        enc.finish_alternatives(); // close only candidate + pop

        assert_eq!(enc.data, b"only");
    }

    /// Bytes written before `start_alternatives` are left intact throughout.
    #[test]
    fn prefix_bytes_are_preserved() {
        let mut enc = Encoder::default();
        push(&mut enc, b"HDR");

        enc.start_alternatives();
        push(&mut enc, b"long_encoding"); // 13 bytes
        enc.end_alternative();
        push(&mut enc, b"short"); // 5 bytes — winner
        enc.finish_alternatives();

        assert_eq!(&enc.data[..3], b"HDR");
        assert_eq!(&enc.data[3..], b"short");
    }

    /// `finish_alternatives` is safe to call after `finish_alternative` covered
    /// the last candidate — it just pops the level without touching `data`.
    #[test]
    fn finish_alternatives_after_finish_alternative_is_noop() {
        let mut enc = Encoder::default();

        enc.start_alternatives();
        push(&mut enc, b"best");
        enc.end_alternative();
        assert!(!enc.alt_stack.is_empty(), "level still on stack");

        enc.finish_alternatives(); // pop; data must be untouched
        assert!(
            enc.alt_stack.is_empty(),
            "stack empty after finish_alternatives"
        );
        assert_eq!(enc.data, b"best");
    }

    // ── nesting ───────────────────────────────────────────────────────────

    /// An inner competition is resolved before the outer candidate is committed.
    #[test]
    fn nested_alternatives() {
        let mut enc = Encoder::default();

        // Outer: try two formats — whichever total is shorter wins.
        enc.start_alternatives();

        // Outer candidate A: header + inner competition.
        push(&mut enc, b"A:");
        enc.start_alternatives(); // inner
        push(&mut enc, b"long_inner"); // 10 bytes
        enc.end_alternative();
        push(&mut enc, b"in"); // 2 bytes — inner winner
        enc.finish_alternatives(); // inner done; data = b"A:in"
        push(&mut enc, b"!");
        enc.end_alternative(); // outer candidate A = b"A:in!" (5 bytes)

        // Outer candidate B: shorter overall.
        push(&mut enc, b"B"); // 1 byte — outer winner
        enc.finish_alternatives(); // outer done

        assert_eq!(enc.data, b"B");
    }

    /// Stack depth reflects nesting level.
    #[test]
    fn nesting_depth_reflected_in_stack() {
        let mut enc = Encoder::default();
        assert_eq!(enc.alt_stack.len(), 0);
        enc.start_alternatives();
        assert_eq!(enc.alt_stack.len(), 1);
        enc.start_alternatives();
        assert_eq!(enc.alt_stack.len(), 2);
        push(&mut enc, b"x");
        enc.finish_alternatives();
        assert_eq!(enc.alt_stack.len(), 1);
        push(&mut enc, b"y");
        enc.finish_alternatives();
        assert_eq!(enc.alt_stack.len(), 0);
    }

    // ── misuse panics ─────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "finish_alternative called outside")]
    fn panics_finish_alternative_without_start() {
        let mut enc = Encoder::default();
        enc.end_alternative();
    }

    #[test]
    #[should_panic(expected = "finish_alternatives called outside")]
    fn panics_finish_alternatives_without_start() {
        let mut enc = Encoder::default();
        enc.finish_alternatives();
    }
}
