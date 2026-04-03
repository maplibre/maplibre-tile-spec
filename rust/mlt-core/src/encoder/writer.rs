use std::io;

use integer_encoding::VarIntWriter as _;

use crate::encoder::EncoderConfig;
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
/// Use [`start_alternative`] / [`finish_alternatives`] to try multiple
/// encodings for a single stream and keep the shortest, all within the
/// same `data` buffer without extra allocations:
///
/// ```rust,ignore
/// enc.start_alternative();
/// write_stream_as_varint(data, &mut enc)?;
/// enc.start_alternative();          // ends the VarInt candidate; starts FastPFOR
/// write_stream_as_fastpfor(data, &mut enc)?;
/// enc.finish_alternatives();        // keeps whichever was shorter
/// ```
///
/// [`hdr`]: Encoder::hdr
/// [`meta`]: Encoder::meta
/// [`data`]: Encoder::data
/// [`impl Write`]: Encoder#impl-Write
/// [`start_alternative`]: Encoder::start_alternative
/// [`finish_alternatives`]: Encoder::finish_alternatives
#[derive(Debug, Default)]
pub struct Encoder {
    /// Encoding configuration: controls which optimisation strategies are tried
    /// (sort orders, compression algorithms, etc.).
    ///
    /// Set once at construction time via [`Encoder::new`]; propagated
    /// automatically to all sub-encoders so individual encode methods do not
    /// need a separate `cfg` argument.
    pub cfg: EncoderConfig,

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
    // Alternatives state (inlined from the former `AltState` struct).
    //
    // Invariant while active (`alt_active == true`):
    //   `data.len() == alt_start + alt_best_size.unwrap_or(0)`
    //
    // `alt_active == false` ↔ no alternatives session in progress.
    // -----------------------------------------------------------------------
    /// `true` while a [`start_alternative`] / [`Encoder::finish_alternatives`] session
    /// is in progress.
    ///
    /// [`start_alternative`]: Encoder::start_alternative
    alt_active: bool,

    /// Position in [`data`] where the current alternatives set began.
    ///
    /// The current best candidate always occupies
    /// `data[alt_start .. alt_start + alt_best_size.unwrap_or(0)]`.
    ///
    /// [`data`]: Encoder::data
    alt_start: usize,

    /// Size of the best candidate seen so far, or `None` if no candidate has
    /// been fully recorded yet (i.e. between the very first
    /// [`start_alternative`] call and the subsequent one).
    ///
    /// [`start_alternative`]: Encoder::start_alternative
    alt_best_size: Option<usize>,
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

    /// Write the layer header (`name`, `extent`, `column_count`) to [`hdr`].
    ///
    /// Must be called exactly once per layer, before any column meta or data.
    ///
    /// [`hdr`]: Encoder::hdr
    pub fn write_header(&mut self, name: &str, extent: u32, column_count: u32) -> MltResult<()> {
        let name_len = u32::try_from(name.len())?;
        self.hdr.write_varint(name_len).map_err(MltError::from)?;
        self.hdr.extend_from_slice(name.as_bytes());
        self.hdr.write_varint(extent).map_err(MltError::from)?;
        self.hdr
            .write_varint(column_count)
            .map_err(MltError::from)?;
        Ok(())
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

    /// Begin a new encoding alternative for the current stream position.
    ///
    /// Call this before writing *each* candidate encoding to `data` (including
    /// the very first one).  Subsequent calls implicitly close the previous
    /// candidate: if the just-written bytes are strictly shorter than the
    /// current best they shift into the best position; otherwise they are
    /// discarded.
    ///
    /// End the alternative set with [`finish_alternatives`].
    ///
    /// # Invariant
    ///
    /// After every `start_alternative()` call:
    /// `data.len() == alt_start + alt_best_size.unwrap_or(0)`.
    /// The next candidate is written starting at `data.len()`.
    ///
    /// [`finish_alternatives`]: Encoder::finish_alternatives
    pub fn start_alternative(&mut self) {
        if self.alt_active {
            // Close the previous candidate.
            let best_end = self.alt_start + self.alt_best_size.unwrap_or(0);
            let cand_size = self.data.len() - best_end;

            if self.alt_best_size.is_none_or(|prev| cand_size < prev) {
                // This candidate is the new best: shift it to `alt_start`.
                if self.alt_best_size.is_some() {
                    self.data
                        .copy_within(best_end..best_end + cand_size, self.alt_start);
                }
                self.data.truncate(self.alt_start + cand_size);
                self.alt_best_size = Some(cand_size);
            } else {
                // Not an improvement: discard.
                self.data.truncate(best_end);
            }
            // Invariant restored: data.len() == alt_start + alt_best_size
        } else {
            // First call: activate and record the start position.
            self.alt_active = true;
            self.alt_start = self.data.len();
            self.alt_best_size = None;
        }
    }

    /// End the alternative set, keeping the shortest encoding.
    ///
    /// Closes the last candidate (same comparison logic as
    /// [`start_alternative`]) and clears the alternative state.
    ///
    /// [`start_alternative`]: Encoder::start_alternative
    pub fn finish_alternatives(&mut self) {
        if self.alt_active {
            let best_end = self.alt_start + self.alt_best_size.unwrap_or(0);
            let cand_size = self.data.len() - best_end;

            if self.alt_best_size.is_none_or(|prev| cand_size < prev) {
                if self.alt_best_size.is_some() {
                    self.data
                        .copy_within(best_end..best_end + cand_size, self.alt_start);
                }
                self.data.truncate(self.alt_start + cand_size);
            } else {
                self.data.truncate(best_end);
            }

            self.alt_active = false;
            self.alt_best_size = None;
        }
    }

    /// Discard any partially-written bytes for the current candidate and
    /// restore `data` to the end of the previous best.
    ///
    /// Call this when a candidate encoding fails mid-write so the corrupted
    /// bytes are removed before continuing.
    pub fn abort_alternative(&mut self) {
        if self.alt_active {
            let clean_end = self.alt_start + self.alt_best_size.unwrap_or(0);
            self.data.truncate(clean_end);
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
