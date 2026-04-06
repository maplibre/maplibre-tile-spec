use std::io;

use integer_encoding::VarIntWriter as _;

use crate::encoder::model::{ExplicitEncoder, StrEncoding, StreamCtx};
use crate::encoder::{EncoderConfig, IdWidth, IntEncoder, VertexBufferType};
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
/// Use [`Encoder::try_alternatives`] to open a competition,
/// then submit each candidate via `AltSession::with`.  The guard's `Drop`
/// impl finalises the competition automatically:
///
/// ```rust,ignore
/// let mut alt = enc.try_alternatives();
/// alt.with(|enc| write_stream_as_varint(data, enc))?;
/// alt.with(|enc| write_stream_as_fastpfor(data, enc))?;
/// // alt drops → keeps whichever was shorter
/// ```
///
/// [`hdr`]: Encoder::hdr
/// [`meta`]: Encoder::meta
/// [`data`]: Encoder::data
/// [`impl Write`]: Encoder#impl-Write
#[derive(Debug, Default)]
pub struct Encoder {
    /// Encoding configuration: controls which optimization strategies are tried
    /// (sort orders, compression algorithms, etc.).
    ///
    /// Set once at construction time via [`Encoder::new`]; propagated
    /// automatically to all sub-encoders so individual encode methods do not
    /// need a separate `cfg` argument.
    pub cfg: EncoderConfig,

    /// When [`Some`], property / ID / geometry encoders use `ExplicitEncoder`
    /// callbacks instead of trying candidate encodings. When [`None`], the
    /// automatic optimization path runs.
    pub(crate) explicit: Option<ExplicitEncoder>,

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
    // Invariant between candidates at any level:
    //   data.len() == level.data_start + level.best_data_size.unwrap_or(0)
    //   meta.len() == level.meta_start + level.best_meta_size.unwrap_or(0)
    //
    // Empty stack ↔ no competition in progress.
    // -----------------------------------------------------------------------
    /// Stack of active encoding competitions, innermost last.
    ///
    /// Empty while no [`Encoder::try_alternatives`] session
    /// is in progress.
    alt_stack: Vec<AltLevel>,
}

/// State for one level of an encoding competition.
///
/// Tracks the starting position in both the [`data`](Encoder::data) and
/// [`meta`](Encoder::meta) buffers, and the byte count of the best candidate
/// committed so far.
///
/// Candidates are compared by **total** bytes (`data + meta`); the shorter one
/// wins, with ties resolved in favor of the earlier candidate.
#[derive(Debug, Default, Clone)]
struct AltLevel {
    data_start: usize,
    meta_start: usize,
    /// Byte count appended to `data` by the current best candidate.
    best_data: Option<usize>,
    /// Byte count appended to `meta` by the current best candidate.
    best_meta: Option<usize>,
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

    /// Like [`Self::new`] but with the explicit encoder set for deterministic encoding
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
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
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
    pub(crate) fn override_int_enc(&self, ctx: &StreamCtx<'_>) -> Option<IntEncoder> {
        self.explicit.as_ref().map(|e| (e.get_int_encoder)(ctx))
    }

    /// When [`Self::explicit`] is [`Some`], returns the callback-chosen [`StrEncoding`].
    /// [`None`] means run automatic string / shared-dict corpus selection.
    #[inline]
    pub(crate) fn override_str_enc(&self, name: &str) -> Option<StrEncoding> {
        self.explicit.as_ref().map(|e| (e.get_str_encoding)(name))
    }

    /// Whether the explicit encoder forces a presence stream for an all-present column
    /// (or similar), per [`ExplicitEncoder::override_presence`].
    #[inline]
    pub(crate) fn override_presence(&self, ctx: &StreamCtx<'_>) -> bool {
        self.explicit
            .as_ref()
            .is_some_and(|e| (e.override_presence)(ctx))
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

    /// Whether to force writing a geometry stream even when its data is empty.
    ///
    /// Delegates to [`ExplicitEncoder::force_stream`]; returns `false` when no explicit
    /// encoder is active (the default "skip empty streams" behavior).
    #[inline]
    pub(crate) fn force_stream(&self, ctx: &StreamCtx<'_>) -> bool {
        self.explicit
            .as_ref()
            .is_some_and(|e| (e.force_stream)(ctx))
    }

    /// Total encoded bytes across all three sections (`hdr + meta + data`).
    #[inline]
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.hdr.len() + self.meta.len() + self.data.len()
    }

    /// Concatenate `hdr + meta + data` into a single buffer **without** a
    /// tag/size prefix.
    ///
    /// Use this when the caller expects a raw layer body
    /// (e.g. [`Layer01::from_bytes`](crate::decoder::Layer01)) rather than a framed wire record.
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
    /// Returns an `AltSession` guard.  Submit each candidate via
    /// `AltSession::with`; the guard's `Drop` impl finalises
    /// the competition and retains the shortest candidate automatically.
    ///
    /// Nesting is supported: calling `try_alternatives` inside a
    /// `with` closure opens an inner competition on the same stack,
    /// resolved before the outer candidate is committed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut alt = enc.try_alternatives();
    /// for cand in candidates {
    ///     alt.with(|enc| write_candidate(cand, enc))?;
    /// }
    /// // alt drops → finalises the competition
    /// ```
    pub fn try_alternatives(&mut self) -> AltSession<'_> {
        self.alt_stack.push(AltLevel {
            data_start: self.data.len(),
            meta_start: self.meta.len(),
            best_data: None,
            best_meta: None,
        });
        AltSession { enc: self }
    }

    /// Commit the current candidate at the innermost competition level.
    ///
    /// Compares bytes written since the last commit against the running best
    /// by **total** (`data + meta`) size; keeps the shorter one.
    ///
    /// Called internally by `AltSession::with` on `Ok`.
    fn alt_commit(&mut self) {
        debug_assert!(
            !self.alt_stack.is_empty(),
            "alt_commit called outside an active AltSession"
        );
        let (data, meta, stack) = (&mut self.data, &mut self.meta, &mut self.alt_stack);
        let level = stack.last_mut().unwrap();
        Self::close_candidate(data, meta, level);
    }

    /// Finalize the innermost competition and pop it from the stack.
    ///
    /// Any bytes written since the last `alt_commit` are evaluated as a
    /// final candidate; if no pending bytes exist and a best is already
    /// recorded this is a cheap stack-pop.
    fn alt_pop(&mut self) {
        debug_assert!(
            !self.alt_stack.is_empty(),
            "alt_pop called outside an active AltSession"
        );
        {
            let (data, meta, stack) = (&mut self.data, &mut self.meta, &mut self.alt_stack);
            let level = stack.last_mut().unwrap();
            let data_pending = data.len() - (level.data_start + level.best_data.unwrap_or(0));
            let meta_pending = meta.len() - (level.meta_start + level.best_meta.unwrap_or(0));
            if data_pending > 0 || meta_pending > 0 || level.best_data.is_none() {
                Self::close_candidate(data, meta, level);
            }
        }
        self.alt_stack.pop();
    }

    /// Shared compare-and-keep logic used by both `alt_commit` and `alt_pop`.
    ///
    /// Compares the bytes written since the last committed candidate against
    /// the current best by **total** (`data + meta`) size.
    /// Keeps the shorter one; ties preserve the existing best.
    fn close_candidate(data: &mut Vec<u8>, meta: &mut Vec<u8>, level: &mut AltLevel) {
        let best_data_end = level.data_start + level.best_data.unwrap_or(0);
        let best_meta_end = level.meta_start + level.best_meta.unwrap_or(0);
        let cand_data = data.len() - best_data_end;
        let cand_meta = meta.len() - best_meta_end;
        let cand_total = cand_data + cand_meta;
        let best_total = level.best_data.unwrap_or(0) + level.best_meta.unwrap_or(0);
        if level.best_data.is_none_or(|_| cand_total < best_total) {
            // New best: shift data candidate bytes to data_start.
            if level.best_data.is_some() {
                data.copy_within(best_data_end..best_data_end + cand_data, level.data_start);
                meta.copy_within(best_meta_end..best_meta_end + cand_meta, level.meta_start);
            }
            data.truncate(level.data_start + cand_data);
            meta.truncate(level.meta_start + cand_meta);
            level.best_data = Some(cand_data);
            level.best_meta = Some(cand_meta);
        } else {
            // Not an improvement: discard.
            data.truncate(best_data_end);
            meta.truncate(best_meta_end);
        }
    }
}

/// RAII guard for a stream-encoding competition opened by [`Encoder::try_alternatives`].
///
/// Submit each candidate via [`with`](AltSession::with); on `Ok` the candidate is
/// committed (compared against the running best and kept if shorter); on `Err`
/// the partial write is rolled back and the error propagates.  The guard's
/// `Drop` impl finalises the competition automatically, so the [`Encoder`] is
/// always left in a consistent state even when an error exits the loop early.
///
/// Nesting is allowed: calling [`Encoder::try_alternatives`] inside a
/// `with` closure opens an inner competition that is fully
/// resolved before the outer candidate is committed.
#[must_use = "AltSession must be used; drop it to finalise the competition"]
pub struct AltSession<'a> {
    enc: &'a mut Encoder,
}

impl AltSession<'_> {
    /// Encode one candidate.
    ///
    /// - **`Ok`** — commits the candidate; replaces the running best if shorter.
    /// - **`Err`** — truncates the partial write back to the pre-call checkpoint
    ///   and returns the error.  The guard's `Drop` still finalises the
    ///   competition cleanly using whichever candidates succeeded so far.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn with<F>(&mut self, f: F) -> MltResult<()>
    where
        F: FnOnce(&mut Encoder) -> MltResult<()>,
    {
        let data_cp = self.enc.data.len();
        let meta_cp = self.enc.meta.len();
        match f(self.enc) {
            Ok(()) => {
                self.enc.alt_commit();
                Ok(())
            }
            Err(e) => {
                self.enc.data.truncate(data_cp);
                self.enc.meta.truncate(meta_cp);
                Err(e)
            }
        }
    }
}

impl Drop for AltSession<'_> {
    fn drop(&mut self) {
        self.enc.alt_pop();
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

    // ── basic single-level behavior ──────────────────────────────────────

    /// The shortest candidate wins.
    #[test]
    fn alternatives_keeps_shortest() {
        let mut enc = Encoder::default();
        push(&mut enc, b"prefix");

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"longer");
            Ok(())
        })
        .unwrap(); // 6 bytes
        alt.with(|enc| {
            push(enc, b"ab");
            Ok(())
        })
        .unwrap(); // 2 bytes — shortest
        alt.with(|enc| {
            push(enc, b"xyz");
            Ok(())
        })
        .unwrap(); // 3 bytes
        drop(alt);

        assert_eq!(enc.data, b"prefixab");
    }

    /// On a tie the first candidate is kept (strict `<`, not `<=`).
    #[test]
    fn alternatives_tie_keeps_first() {
        let mut enc = Encoder::default();

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"aaa");
            Ok(())
        })
        .unwrap(); // 3 bytes
        alt.with(|enc| {
            push(enc, b"bbb");
            Ok(())
        })
        .unwrap(); // 3 bytes — equal
        drop(alt);

        assert_eq!(enc.data, b"aaa");
    }

    /// A single candidate is unconditionally the winner.
    #[test]
    fn alternatives_single_candidate() {
        let mut enc = Encoder::default();

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"only");
            Ok(())
        })
        .unwrap();
        drop(alt);

        assert_eq!(enc.data, b"only");
    }

    /// Bytes written before `try_alternatives` are left intact throughout.
    #[test]
    fn prefix_bytes_are_preserved() {
        let mut enc = Encoder::default();
        push(&mut enc, b"HDR");

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"long_encoding");
            Ok(())
        })
        .unwrap(); // 13 bytes
        alt.with(|enc| {
            push(enc, b"short");
            Ok(())
        })
        .unwrap(); // 5 bytes — winner
        drop(alt);

        assert_eq!(&enc.data[..3], b"HDR");
        assert_eq!(&enc.data[3..], b"short");
    }

    /// Dropping the guard after all candidates are committed is a cheap stack-pop.
    #[test]
    fn drop_after_all_committed_is_noop() {
        let mut enc = Encoder::default();

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"best");
            Ok(())
        })
        .unwrap();
        drop(alt); // all candidates committed; drop just pops the stack

        assert!(enc.alt_stack.is_empty(), "stack empty after drop");
        assert_eq!(enc.data, b"best");
    }

    // ── nesting ───────────────────────────────────────────────────────────

    /// An inner competition is resolved before the outer candidate is committed.
    #[test]
    fn nested_alternatives() {
        let mut enc = Encoder::default();

        let mut outer = enc.try_alternatives();

        // Outer candidate A: header bytes + inner competition.
        outer
            .with(|enc| {
                push(enc, b"A:");
                let mut inner = enc.try_alternatives(); // inner level pushed
                inner.with(|enc| {
                    push(enc, b"long_inner");
                    Ok(())
                })?; // 10 bytes
                inner.with(|enc| {
                    push(enc, b"in");
                    Ok(())
                })?; // 2 bytes — inner winner
                drop(inner); // inner done; enc = b"A:in"
                push(enc, b"!");
                Ok(())
            })
            .unwrap(); // outer candidate A = b"A:in!" (5 bytes)

        // Outer candidate B: shorter overall.
        outer
            .with(|enc| {
                push(enc, b"B");
                Ok(())
            })
            .unwrap(); // 1 byte — winner
        drop(outer);

        assert_eq!(enc.data, b"B");
    }

    /// Stack depth tracks nesting level; inner guard drops before outer closure returns.
    #[test]
    fn nesting_depth_reflected_in_stack() {
        let mut enc = Encoder::default();

        assert_eq!(enc.alt_stack.len(), 0);
        let mut outer = enc.try_alternatives();

        outer
            .with(|enc| {
                assert_eq!(enc.alt_stack.len(), 1); // outer level on stack
                let mut inner = enc.try_alternatives();
                inner.with(|enc| {
                    assert_eq!(enc.alt_stack.len(), 2); // both levels on stack
                    push(enc, b"x");
                    Ok(())
                })?;
                drop(inner); // inner popped
                assert_eq!(enc.alt_stack.len(), 1);
                push(enc, b"y");
                Ok(())
            })
            .unwrap();

        drop(outer); // outer popped
        assert_eq!(enc.alt_stack.len(), 0);
    }

    // ── meta buffer tracking ──────────────────────────────────────────────

    /// Writes to both `data` and `meta` are rolled back for the losing
    /// candidate and kept for the winner, measured by total bytes.
    #[test]
    fn alternatives_tracks_meta_and_data() {
        let mut enc = Encoder::default();
        enc.data.extend_from_slice(b"D");
        enc.meta.extend_from_slice(b"M");

        let mut alt = enc.try_alternatives();
        // Candidate A: 4 data + 2 meta = 6 total
        alt.with(|enc| {
            push(enc, b"DDDD");
            enc.meta.extend_from_slice(b"mm");
            Ok(())
        })
        .unwrap();
        // Candidate B: 1 data + 1 meta = 2 total — winner
        alt.with(|enc| {
            push(enc, b"d");
            enc.meta.extend_from_slice(b"n");
            Ok(())
        })
        .unwrap();
        drop(alt);

        assert_eq!(enc.data, b"Dd");
        assert_eq!(enc.meta, b"Mn");
    }

    // ── error rollback ────────────────────────────────────────────────────

    /// A failing candidate is rolled back; prior best is preserved.
    #[test]
    fn error_candidate_is_rolled_back() {
        let mut enc = Encoder::default();

        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            push(enc, b"ok");
            Ok(())
        })
        .unwrap();
        let _ = alt.with(|enc| {
            push(enc, b"partial");
            Err(MltError::IntegerOverflow) // simulated failure
        });
        drop(alt);

        assert_eq!(enc.data, b"ok"); // "partial" was rolled back; "ok" kept
    }
}
