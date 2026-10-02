//! rANS over componentwise vertex deltas, with tables picked by run start and by the dx class.
//!
//! ```text
//! [u8 depth]             mantissa bits a symbol keeps, 0-2
//! [tables]               one per context, bit-packed LSB-first, padded to a byte
//! [varint lane_len; 4]
//! [lane; 4]              u32 LE state, then u16 LE refill words
//! [raw bits]             LSB-first
//! ```

use integer_encoding::VarInt as _;

use crate::MltError::MalformedRans;
use crate::codecs::varint::parse_varint;
use crate::{Decoder, MltResult};

/// A state below this is refilled with the lane's next word.
const STATE_LOW: u32 = 1 << 16;
/// Largest table precision, in bits.
const MAX_PRECISION: u32 = 11;
/// Largest mantissa depth.
const MAX_DEPTH: u32 = 2;
/// Symbol `0` is a zero delta, the rest are two per (bit length, mantissa) code, up to `i32::MIN` at depth 2.
const SYMBOLS: usize = 257;
/// dx contexts: inside a run, at a run start.
const X_CONTEXTS: usize = 2;
/// dx classes that pick a dy context: the bit length of dx's magnitude, capped.
const CLASSES: usize = 6;
const CONTEXTS: usize = X_CONTEXTS + 2 * CLASSES;
/// The context array's length, a power of two so a masked index is in bounds.
const CONTEXT_SLOTS: usize = CONTEXTS.next_power_of_two();
const LANES: usize = 4;
/// Vertex pairs decoded to slots before their raw bits are read.
const BLOCK: usize = 128;
/// Slab sizes are powers of two, so a masked lookup is in bounds.
const MIN_SLAB: usize = 0x1000;
/// Room for every context at the largest precision plus the invalid slot.
const MAX_SLAB: usize = ((CONTEXTS << MAX_PRECISION) + 1).next_power_of_two();

/// `(symbol, raw bit count, raw bits)` of `delta` at mantissa depth `depth`.
fn symbolize(delta: i32, depth: u32) -> (usize, u32, u32) {
    if delta == 0 {
        return (0, 0, 0);
    }
    let mag = delta.unsigned_abs();
    let len = mag.bit_width();
    let kept = (len - 1).min(depth);
    let raw_bits = len - 1 - kept;
    let mantissa = (mag >> raw_bits) & low_mask(kept);
    let code = (len << depth) | (mantissa << (depth - kept));
    let symbol = 2 * code - u32::from(delta > 0);
    (symbol as usize, raw_bits, mag & low_mask(raw_bits))
}

#[inline]
#[expect(clippy::cast_possible_truncation, reason = "bits is at most 32")]
const fn low_mask(bits: u32) -> u32 {
    ((1_u64 << bits) - 1) as u32
}

/// The dy context class of a dx symbol.
const fn class(symbol: usize, depth: u32) -> usize {
    let class = symbol.div_ceil(2) >> depth;
    if class < CLASSES { class } else { CLASSES - 1 }
}

/// The slot fields `symbol` decodes to at `depth`, frequency and offset left `0`,
/// or [`None`] if no delta maps to it.
#[inline]
fn info(symbol: usize, depth: u32) -> Option<Slot> {
    let e = INFO[depth as usize][symbol];
    (e != NO_SLOT).then_some(e)
}

/// [`info`] for every symbol at every depth, [`NO_SLOT`] where no delta maps to it.
static INFO: [[Slot; SYMBOLS]; MAX_DEPTH as usize + 1] = {
    let mut table = [[NO_SLOT; SYMBOLS]; MAX_DEPTH as usize + 1];
    let mut depth = 0;
    while depth <= MAX_DEPTH {
        let mut symbol = 0;
        while symbol < SYMBOLS {
            table[depth as usize][symbol] = slot_fields(symbol, depth);
            symbol += 1;
        }
        depth += 1;
    }
    table
};

const NO_SLOT: Slot = Slot::MAX;

#[expect(
    clippy::cast_possible_truncation,
    reason = "fewer than SYMBOLS symbols"
)]
const fn slot_fields(symbol: usize, depth: u32) -> Slot {
    if symbol == 0 {
        return (X_CONTEXTS as u64) << 22;
    }
    let code = symbol.div_ceil(2) as u32;
    let len = code >> depth;
    if len < 1 || len > 32 {
        return NO_SLOT;
    }
    let kept = if len - 1 < depth { len - 1 } else { depth };
    let mantissa = code & low_mask(depth);
    if mantissa & low_mask(depth - kept) != 0 {
        return NO_SLOT;
    }
    let positive = symbol.is_multiple_of(2);
    // Only `i32::MIN` has a 32-bit magnitude, and it is negative with no mantissa bits.
    if len == 32 && (!positive || mantissa != 0) {
        return NO_SLOT;
    }
    let head = (1 << kept) | (mantissa >> (depth - kept));
    ((len - 1 - kept) as u64) << 16
        | (positive as u64) << 21
        | ((X_CONTEXTS + 2 * class(symbol, depth)) as u64) << 22
        | (head as u64) << 27
}

/// One flag per vertex, `1` at each of the run offsets `offsets`, or at every vertex without them.
fn run_starts(offsets: Option<&[u32]>, vertex_count: usize) -> MltResult<Vec<u8>> {
    let Some(offsets) = offsets else {
        return Ok(vec![1; vertex_count]);
    };
    if offsets.first().is_some_and(|&o| o != 0)
        || offsets.last().map_or(0, |&o| o as usize) != vertex_count
    {
        return Err(MalformedRans("runs do not cover the vertices"));
    }
    let mut first = vec![0; vertex_count];
    for &o in offsets {
        match first.get_mut(o as usize) {
            Some(flag) => *flag = 1,
            None if o as usize == vertex_count => {}
            None => return Err(MalformedRans("runs do not cover the vertices")),
        }
    }
    Ok(first)
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    bits: u32,
}

impl BitWriter {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "writes the accumulator's low byte"
    )]
    fn put(&mut self, value: u32, bits: u32) {
        self.acc |= u64::from(value) << self.bits;
        self.bits += bits;
        while self.bits >= 8 {
            self.bytes.push(self.acc as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }

    fn gamma(&mut self, value: u32) {
        let zeros = u32::BITS - 1 - value.leading_zeros();
        self.put(0, zeros);
        self.put(1, 1);
        self.put(value & low_mask(zeros), zeros);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "writes the accumulator's low byte"
    )]
    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.bytes.push(self.acc as u8);
        }
        self.bytes
    }
}

fn gamma_len(value: u32) -> u32 {
    2 * (u32::BITS - 1 - value.leading_zeros()) + 1
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    /// The 57 or more bits from the current position on, zero past the end.
    fn peek(&self) -> u64 {
        let byte = self.bit / 8;
        if let Some(&word) = self.bytes.get(byte..).and_then(<[u8]>::first_chunk) {
            return u64::from_le_bytes(word) >> (self.bit % 8);
        }
        let mut word = [0; 8];
        let tail = self.bytes.get(byte..).unwrap_or(&[]);
        let n = tail.len().min(8);
        word[..n].copy_from_slice(&tail[..n]);
        u64::from_le_bytes(word) >> (self.bit % 8)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "at most 32 bits are masked in"
    )]
    fn get(&mut self, bits: u32) -> u32 {
        let value = (self.peek() & u64::from(low_mask(bits))) as u32;
        self.bit += bits as usize;
        value
    }

    fn gamma(&mut self) -> MltResult<u32> {
        let zeros = self.peek().trailing_zeros();
        if zeros > MAX_PRECISION + 1 {
            return Err(MalformedRans("frequency out of range"));
        }
        self.bit += zeros as usize + 1;
        Ok((1 << zeros) | self.get(zeros))
    }

    fn overran(&self) -> bool {
        self.bit > self.bytes.len() * 8
    }
}

/// A context's normalised frequencies, summing to `1 << precision`.
struct Table {
    precision: u32,
    freq: Vec<u32>,
}

impl Table {
    fn write(&self, w: &mut BitWriter) {
        let max = self.freq.len() - 1;
        w.put(self.precision, 4);
        w.put(u32::try_from(max).expect("fewer than 512 symbols"), 9);
        if self.precision > 0 {
            for &f in &self.freq {
                w.gamma(f + 1);
            }
        }
    }

    fn read(r: &mut BitReader, depth: u32, dec: &mut Decoder) -> MltResult<Self> {
        let precision = r.get(4);
        let max = r.get(9) as usize;
        if precision > MAX_PRECISION || max >= SYMBOLS {
            return Err(MalformedRans("table out of range"));
        }
        let mut freq = dec.alloc(max + 1)?;
        freq.resize(max + 1, 0);
        if precision == 0 {
            freq[max] = 1;
        } else {
            for f in &mut freq {
                *f = r.gamma()? - 1;
            }
        }
        let names_invalid = freq
            .iter()
            .enumerate()
            .any(|(s, &f)| f > 0 && info(s, depth).is_none());
        if freq.iter().sum::<u32>() != 1 << precision || freq[max] == 0 || names_invalid {
            return Err(MalformedRans("table frequencies"));
        }
        Ok(Self { precision, freq })
    }

    /// Where each symbol's slots start.
    fn cumulative(&self) -> Vec<u32> {
        self.freq
            .iter()
            .scan(0, |total, &f| {
                let start = *total;
                *total += f;
                Some(start)
            })
            .collect()
    }
}

/// The table for `counts` that is cheapest in data plus table bits.
fn normalise(counts: &[u64]) -> Table {
    let max = counts
        .iter()
        .rposition(|&c| c > 0)
        .expect("a context with symbols");
    let counts = &counts[..=max];
    let total: u64 = counts.iter().sum();
    let named = counts.iter().filter(|&&c| c > 0).count();
    if named == 1 {
        let mut freq = vec![0; max + 1];
        freq[max] = 1;
        return Table { precision: 0, freq };
    }
    let lowest = (named - 1).bit_width();
    (lowest..=MAX_PRECISION)
        .map(|precision| {
            let freq = quantise(counts, total, precision);
            let scale = f64::from(1_u32 << precision);
            #[expect(clippy::cast_precision_loss, reason = "a cost estimate")]
            let data: f64 = counts
                .iter()
                .zip(&freq)
                .filter(|&(&c, _)| c > 0)
                .map(|(&c, &f)| -(c as f64) * (f64::from(f) / scale).log2())
                .sum();
            let table: u32 = freq.iter().map(|&f| gamma_len(f + 1)).sum();
            (data + f64::from(table), Table { precision, freq })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("at least one precision fits")
        .1
}

/// Scale `counts` to sum to `1 << precision`, keeping every named symbol at one or more.
fn quantise(counts: &[u64], total: u64, precision: u32) -> Vec<u32> {
    let target = 1_u64 << precision;
    let mut freq: Vec<u64> = counts
        .iter()
        .map(|&c| {
            if c == 0 {
                0
            } else {
                ((c * target + total / 2) / total).max(1)
            }
        })
        .collect();
    let largest = |freq: &[u64]| (0..freq.len()).max_by_key(|&i| freq[i]).expect("non-empty");
    let mut sum: u64 = freq.iter().sum();
    while sum > target {
        let i = largest(&freq);
        freq[i] -= 1;
        sum -= 1;
    }
    let i = largest(&freq);
    freq[i] += target - sum;
    freq.into_iter()
        .map(|f| u32::try_from(f).expect("at most 1 << precision"))
        .collect()
}

/// Code `vertices` (`[x0, y0, x1, y1, …]`) at the cheapest depth.
///
/// A run starts at each of `offsets`, which start at `0` and end at the vertex count,
/// or at every vertex when there are none.
pub fn encode_vertices(vertices: &[i32], offsets: Option<&[u32]>) -> MltResult<Vec<u8>> {
    if !vertices.len().is_multiple_of(2) {
        return Err(MalformedRans("odd vertex word count"));
    }
    let first = run_starts(offsets, vertices.len() / 2)?;
    Ok((0..=MAX_DEPTH)
        .map(|depth| encode_at(vertices, &first, depth))
        .min_by_key(Vec::len)
        .expect("three depths"))
}

fn encode_at(vertices: &[i32], first: &[u8], depth: u32) -> Vec<u8> {
    let mut raw = BitWriter::default();
    let mut counts = vec![[0_u64; SYMBOLS]; CONTEXTS];
    let mut symbols = Vec::with_capacity(vertices.len());
    let mut prev = [0_i32; 2];
    for (&[x, y], &start) in vertices.as_chunks::<2>().0.iter().zip(first) {
        let (sx, x_bits, x_raw) = symbolize(x.wrapping_sub(prev[0]), depth);
        let (sy, y_bits, y_raw) = symbolize(y.wrapping_sub(prev[1]), depth);
        prev = [x, y];
        raw.put(x_raw, x_bits);
        raw.put(y_raw, y_bits);
        let cx = usize::from(start);
        let cy = X_CONTEXTS + 2 * class(sx, depth) + usize::from(start);
        counts[cx][sx] += 1;
        counts[cy][sy] += 1;
        symbols.push((cx, sx));
        symbols.push((cy, sy));
    }

    let mut out = vec![u8::try_from(depth).expect("depth is at most 2")];
    let mut w = BitWriter::default();
    let tables: Vec<Option<(Table, Vec<u32>)>> = counts
        .iter()
        .map(|c| {
            let present = c.iter().any(|&n| n > 0);
            w.put(u32::from(present), 1);
            present.then(|| {
                let t = normalise(c);
                t.write(&mut w);
                let cum = t.cumulative();
                (t, cum)
            })
        })
        .collect();
    out.extend(w.finish());

    let mut states = [STATE_LOW; LANES];
    let mut words: [Vec<u16>; LANES] = Default::default();
    for (i, &(ctx, symbol)) in symbols.iter().enumerate().rev() {
        let lane = 2 * (i / 2 % 2) + i % 2;
        let (t, cum) = tables[ctx]
            .as_ref()
            .expect("a table for every used context");
        let (f, state) = (t.freq[symbol], &mut states[lane]);
        let limit = (u64::from(STATE_LOW >> t.precision) << 16) * u64::from(f);
        while u64::from(*state) >= limit {
            words[lane].push((*state & 0xffff) as u16);
            *state >>= 16;
        }
        *state = ((*state / f) << t.precision) + *state % f + cum[symbol];
    }
    for words in &words {
        out.extend((4 + 2 * words.len()).encode_var_vec());
    }
    for (words, state) in words.iter().zip(states) {
        out.extend(state.to_le_bytes());
        out.extend(words.iter().rev().flat_map(|w| w.to_le_bytes()));
    }
    out.extend(raw.finish());
    out
}

/// `off:12 | freq:12 | raw_bits:5 | negative:1 | class:3 | … | head:3 | … | invalid:1`.
type Slot = u64;
const INVALID: Slot = 1 << 47;

/// The dy context a dx slot picks.
#[inline]
fn dy_context(e: Slot, start: u8) -> usize {
    (e >> 22 & 15) as usize + usize::from(start)
}

/// Raw bits over a buffer padded with 8 zero bytes, so a read never runs off it.
struct RawBits<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl RawBits<'_> {
    /// The delta a slot names, completed from the raw bits.
    #[inline]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "bit fields"
    )]
    fn value(&mut self, e: Slot) -> i32 {
        let raw_bits = (e >> 16 & 31) as u32;
        let byte = self.bit / 8;
        let word = self.bytes.get(byte..byte + 8).map_or(0, |b| {
            u64::from_le_bytes(b.try_into().expect("an 8-byte slice"))
        });
        let low = (word >> (self.bit % 8)) as u32 & low_mask(raw_bits);
        self.bit += raw_bits as usize;
        let mag = ((e >> 27 & 7) as u32) << raw_bits | low;
        let sign = -(((e >> 21) & 1) as i32);
        ((mag as i32) ^ sign).wrapping_sub(sign)
    }
}

/// Bytes a block's raw bits can span, at most `4 * BLOCK * 31` bits from its first byte, rounded up to a power of two.
const WINDOW: usize = 2048;

/// One block's raw bits, read at a masked index so no read needs a bounds check.
struct Window<'a> {
    bytes: &'a [u8; WINDOW + 8],
    bit: usize,
}

impl RawBits<'_> {
    /// The window the next block reads from, or [`None`] once the bits run past the buffer.
    fn window(&self) -> Option<Window<'_>> {
        let byte = self.bit / 8;
        let bytes = self.bytes.get(byte..byte + WINDOW + 8)?.try_into().ok()?;
        Some(Window {
            bytes,
            bit: self.bit % 8,
        })
    }
}

impl Window<'_> {
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "measured on the decode hot path, and bit fields"
    )]
    fn value(&mut self, e: Slot) -> i32 {
        let raw_bits = (e >> 16 & 31) as u32;
        let byte = (self.bit / 8) % WINDOW;
        let word = u64::from_le_bytes(
            self.bytes[byte..byte + 8]
                .try_into()
                .expect("an 8-byte slice"),
        );
        let low = (word >> (self.bit % 8)) as u32 & low_mask(raw_bits);
        self.bit += raw_bits as usize;
        let mag = ((e >> 27 & 7) as u32) << raw_bits | low;
        let sign = -(((e >> 21) & 1) as i32);
        ((mag as i32) ^ sign).wrapping_sub(sign)
    }

    /// Complete `slots`, alternating `dx` and `dy`, into running coordinates, returning their `INVALID` bits.
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "measured on the decode hot path")]
    fn resolve(&mut self, slots: &[Slot], out: &mut [i32], x: &mut i32, y: &mut i32) -> Slot {
        let mut invalid = 0;
        for (o, &[ex, ey]) in out
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .zip(slots.as_chunks::<2>().0)
        {
            invalid |= ex | ey;
            *x = x.wrapping_add(self.value(ex));
            *y = y.wrapping_add(self.value(ey));
            *o = [*x, *y];
        }
        invalid
    }
}

struct Lane<'a> {
    bytes: &'a [u8],
    pos: usize,
    state: u32,
}

impl<'a> Lane<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        let &state = bytes
            .first_chunk::<4>()
            .expect("split checked the lane length");
        Self {
            bytes,
            pos: 4,
            state: u32::from_le_bytes(state),
        }
    }

    #[inline]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the offset is the low 16 bits"
    )]
    fn step<const N: usize>(&mut self, (start, precision): (u32, u32), slab: &[Slot; N]) -> Slot {
        let mask = (1 << precision) - 1;
        let e = slab[(start + (self.state & mask)) as usize % N];
        let (off, freq) = (u32::from(e as u16), (e >> 48) as u32);
        self.state = freq.wrapping_mul(self.state >> precision).wrapping_add(off);
        let word = self
            .bytes
            .get(self.pos..self.pos + 2)
            .map_or(0, |b| u32::from(b[0]) | u32::from(b[1]) << 8);
        let refill = u32::from(self.state < STATE_LOW);
        self.state = (self.state << (refill * 16)) | (word & refill.wrapping_neg());
        self.pos += 2 * refill as usize;
        e
    }

    /// The bytes the next block can refill from, copied into `pad` with zeros only near the lane's end.
    fn stage<'p>(&'p self, pad: &'p mut [u8; STAGE]) -> Staged<'p> {
        let rest = self.bytes.get(self.pos..).unwrap_or_default();
        let bytes = if let Some(window) = rest.first_chunk() {
            window
        } else {
            pad[..rest.len()].copy_from_slice(rest);
            pad[rest.len()..].fill(0);
            pad
        };
        Staged {
            bytes,
            state: self.state,
            rel: 0,
        }
    }

    fn unstage(&mut self, (state, rel): (u32, usize)) {
        self.state = state;
        self.pos += rel;
    }

    fn finished(&self) -> bool {
        self.state == STATE_LOW && self.pos == self.bytes.len()
    }
}

/// Bytes one block can refill, 16 bits per step at most, plus room for a masked read.
const STAGE: usize = 2 * BLOCK + 2;

/// A lane during one block, refilling from a copy of its next bytes.
struct Staged<'a> {
    bytes: &'a [u8; STAGE],
    state: u32,
    rel: usize,
}

impl Staged<'_> {
    fn done(self) -> (u32, usize) {
        (self.state, self.rel)
    }

    #[inline(always)]
    #[expect(
        clippy::inline_always,
        clippy::cast_possible_truncation,
        reason = "measured on the decode hot path, and the offset is the low 16 bits"
    )]
    fn step<const N: usize>(&mut self, (start, precision): (u32, u32), slab: &[Slot; N]) -> Slot {
        let mask = (1 << precision) - 1;
        let e = slab[(start + (self.state & mask)) as usize % N];
        let (off, freq) = (u32::from(e as u16), (e >> 48) as u32);
        self.state = freq.wrapping_mul(self.state >> precision).wrapping_add(off);
        let at = self.rel % (2 * BLOCK);
        let word = u32::from(self.bytes[at]) | u32::from(self.bytes[at + 1]) << 8;
        let refill = u32::from(self.state < STATE_LOW);
        self.state = (self.state << (refill * 16)) | (word & refill.wrapping_neg());
        self.rel += 2 * refill as usize;
        e
    }
}

/// A payload split into its parts, before any symbol is decoded.
pub(crate) struct Parts<'a> {
    pub depth: u32,
    tables: Vec<Option<Table>>,
    pub lanes: [&'a [u8]; LANES],
    pub raw: &'a [u8],
}

impl Parts<'_> {
    /// The precision of each context's table, [`None`] where the context has none.
    pub fn precisions(&self) -> impl Iterator<Item = Option<u32>> {
        self.tables.iter().map(|t| t.as_ref().map(|t| t.precision))
    }
}

/// Split `data` into its depth, tables, lanes and raw bits, charging `dec` for the tables.
pub(crate) fn split<'a>(data: &'a [u8], dec: &mut Decoder) -> MltResult<Parts<'a>> {
    let (&depth, rest) = data.split_first().ok_or(MalformedRans("empty"))?;
    let depth = u32::from(depth);
    if depth > MAX_DEPTH {
        return Err(MalformedRans("depth"));
    }

    let mut r = BitReader {
        bytes: rest,
        bit: 0,
    };
    let mut tables = dec.alloc(CONTEXTS)?;
    for _ in 0..CONTEXTS {
        tables.push(if r.get(1) == 1 {
            Some(Table::read(&mut r, depth, dec)?)
        } else {
            None
        });
        if r.overran() {
            return Err(MalformedRans("tables truncated"));
        }
    }

    let mut rest = &rest[r.bit.div_ceil(8)..];
    let mut lens = [0_usize; LANES];
    for len in &mut lens {
        let (after, l) = parse_varint::<u32>(rest)?;
        (rest, *len) = (after, l as usize);
    }
    let mut lanes = [&[][..]; LANES];
    for (lane, len) in lanes.iter_mut().zip(lens) {
        if len < 4 || !len.is_multiple_of(2) {
            return Err(MalformedRans("lane length"));
        }
        (*lane, rest) = rest
            .split_at_checked(len)
            .ok_or(MalformedRans("lanes truncated"))?;
    }
    Ok(Parts {
        depth,
        tables,
        lanes,
        raw: rest,
    })
}

/// Decode `num_values` vertex words whose runs start at `offsets`, as [`encode_vertices`] takes them.
pub fn decode_vertices(
    data: &[u8],
    offsets: Option<&[u32]>,
    num_values: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    if !num_values.is_multiple_of(2) {
        return Err(MalformedRans("odd vertex word count"));
    }
    let vertex_count = num_values as usize / 2;
    dec.consume_items::<u8>(vertex_count)?;
    let first = run_starts(offsets, vertex_count)?;
    let Parts {
        depth,
        tables,
        lanes,
        raw,
    } = split(data, dec)?;

    let used = 1 + tables
        .iter()
        .flatten()
        .map(|t| 1_usize << t.precision)
        .sum::<usize>();
    let size = used.next_power_of_two().max(MIN_SLAB);
    dec.consume_items::<Slot>(size)?;
    if dec.rans_slots.len() < size {
        dec.rans_slots.resize(size, 0);
    }
    let slab = &mut dec.rans_slots[..size];
    slab[0] = INVALID | 1 << 48;
    let mut contexts = [(0_u32, 0_u32); CONTEXT_SLOTS];
    let mut at = 1;
    for (ctx, t) in contexts.iter_mut().zip(&tables) {
        let Some(t) = t else { continue };
        *ctx = (
            u32::try_from(at).expect("at most MAX_SLAB slots"),
            t.precision,
        );
        for (symbol, &f) in t.freq.iter().enumerate() {
            if f == 0 {
                continue;
            }
            if let Some(e) = info(symbol, depth) {
                let e = e | u64::from(f) << 48;
                for (k, s) in (0..).zip(&mut slab[at..at + f as usize]) {
                    *s = e | k;
                }
                at += f as usize;
            }
        }
    }

    let [l0, l1, l2, l3] = lanes;
    let lanes = [Lane::new(l0), Lane::new(l1), Lane::new(l2), Lane::new(l3)];
    let mut padded = dec.alloc(raw.len() + WINDOW + 8)?;
    padded.extend_from_slice(raw);
    padded.resize(raw.len() + WINDOW + 8, 0);

    let mut out = dec.alloc::<i32>(2 * vertex_count)?;
    out.resize(2 * vertex_count, 0);
    let slab = &dec.rans_slots[..size];
    let s = Symbols {
        contexts: &contexts,
        first: &first,
        raw: RawBits {
            bytes: &padded,
            bit: 0,
        },
        lanes,
    };
    let raw_bits = match size {
        MIN_SLAB => s.decode::<MIN_SLAB>(slab, &mut out),
        0x2000 => s.decode::<0x2000>(slab, &mut out),
        0x4000 => s.decode::<0x4000>(slab, &mut out),
        // Every table at the largest precision still fits, so no other size is left.
        _ => s.decode::<MAX_SLAB>(slab, &mut out),
    }?;
    if raw_bits.div_ceil(8) != raw.len() {
        return Err(MalformedRans("raw bits length"));
    }
    Ok(out)
}

/// Everything the symbol loop reads, once the tables are in the slab.
struct Symbols<'a> {
    contexts: &'a [(u32, u32); CONTEXT_SLOTS],
    first: &'a [u8],
    raw: RawBits<'a>,
    lanes: [Lane<'a>; LANES],
}

impl Symbols<'_> {
    /// Decode every vertex into `out` over a slab of `N` slots, returning how many raw bits were read.
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "measured on the decode hot path")]
    fn decode<const N: usize>(self, slab: &[Slot], out: &mut [i32]) -> MltResult<usize> {
        let slab: &[Slot; N] = slab.try_into().expect("a slab of N slots");
        let Self {
            contexts,
            first,
            mut raw,
            lanes: [mut l0, mut l1, mut l2, mut l3],
        } = self;
        let ctx = |i: usize| contexts[i % CONTEXT_SLOTS];
        let (mut x, mut y) = (0_i32, 0_i32);
        let mut invalid = 0;
        let (quads, tail) = out.as_chunks_mut::<4>();
        let (starts, last) = first.as_chunks::<2>();
        let mut buf = [[0 as Slot; 4]; BLOCK];
        for (quads, starts) in quads.chunks_mut(BLOCK).zip(starts.chunks(BLOCK)) {
            let mut pads = [[0; STAGE]; LANES];
            let [p0, p1, p2, p3] = &mut pads;
            let [mut s0, mut s1, mut s2, mut s3] =
                [l0.stage(p0), l1.stage(p1), l2.stage(p2), l3.stage(p3)];
            for (b, &[f0, f1]) in buf.iter_mut().zip(starts) {
                let ex0 = s0.step(ctx(usize::from(f0)), slab);
                let ex1 = s2.step(ctx(usize::from(f1)), slab);
                let ey0 = s1.step(ctx(dy_context(ex0, f0)), slab);
                let ey1 = s3.step(ctx(dy_context(ex1, f1)), slab);
                *b = [ex0, ey0, ex1, ey1];
            }
            let [t0, t1, t2, t3] = [s0.done(), s1.done(), s2.done(), s3.done()];
            l0.unstage(t0);
            l1.unstage(t1);
            l2.unstage(t2);
            l3.unstage(t3);
            let mut w = raw.window().ok_or(MalformedRans("raw bits length"))?;
            let slots = buf[..quads.len()].as_flattened();
            let out = quads.as_flattened_mut();
            invalid |= w.resolve(slots, out, &mut x, &mut y);
            raw.bit = raw.bit / 8 * 8 + w.bit;
        }
        if let ([px, py], [f0]) = (tail, last) {
            let ex0 = l0.step(ctx(usize::from(*f0)), slab);
            let ey0 = l1.step(ctx(dy_context(ex0, *f0)), slab);
            invalid |= ex0 | ey0;
            *px = x.wrapping_add(raw.value(ex0));
            *py = y.wrapping_add(raw.value(ey0));
        }
        if invalid & INVALID != 0 {
            return Err(MalformedRans("symbol from a missing table"));
        }
        if ![l0, l1, l2, l3].iter().all(Lane::finished) {
            return Err(MalformedRans("lanes do not end in the initial state"));
        }
        Ok(raw.bit)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::__private::dec;

    fn words(vertices: &[i32]) -> u32 {
        u32::try_from(vertices.len()).unwrap()
    }

    fn round_trip(vertices: &[i32], offsets: Option<&[u32]>) -> Vec<i32> {
        let data = encode_vertices(vertices, offsets).unwrap();
        decode_vertices(&data, offsets, words(vertices), &mut dec()).unwrap()
    }

    #[rstest]
    #[case::empty(&[], Some(&[0][..]))]
    #[case::every_vertex_a_run(&[5, -7, 8, 1, 8, 1], None)]
    #[case::one_vertex(&[5, -7], Some(&[0, 1][..]))]
    #[case::extreme_deltas(&[i32::MIN, i32::MAX, i32::MAX, i32::MIN, 0, 0, i32::MIN, i32::MIN], Some(&[0, 2, 4][..]))]
    #[case::empty_runs(&[1, 2, 3, 4, 5, 6], Some(&[0, 0, 2, 2, 3, 3][..]))]
    #[case::repeated_vertex(&[9, 9, 9, 9, 9, 9], Some(&[0, 3][..]))]
    fn decodes_what_it_encodes(#[case] vertices: &[i32], #[case] offsets: Option<&[u32]>) {
        assert_eq!(round_trip(vertices, offsets), vertices);
    }

    #[test]
    fn encodes_the_spec_example() {
        let data = encode_vertices(&[100, 200, 105, 210, 102, 215], Some(&[0, 3])).unwrap();
        #[rustfmt::skip]
        let expected = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x04, 0x69,
        ];
        assert_eq!(data, expected);
    }

    #[test]
    fn decodes_the_spec_example() {
        #[rustfmt::skip]
        let data = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x04, 0x69,
        ];
        let vertices = decode_vertices(&data, Some(&[0, 3]), 6, &mut dec()).unwrap();
        assert_eq!(vertices, [100, 200, 105, 210, 102, 215]);
    }

    #[test]
    fn a_decoder_reuses_its_slab() {
        #[rustfmt::skip]
        let data = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x04, 0x69,
        ];
        let mut d = dec();
        decode_vertices(&data, Some(&[0, 3]), 6, &mut d).unwrap();
        let vertices = decode_vertices(&data, Some(&[0, 3]), 6, &mut d).unwrap();
        assert_eq!(vertices, [100, 200, 105, 210, 102, 215]);
        assert_eq!(d.rans_slots.len(), MIN_SLAB);
    }

    #[rstest]
    #[case::vertex_flags(0, "memory limit exceeded: limit=0, used=0, requested=3")]
    #[case::tables(3, "memory limit exceeded: limit=3, used=3, requested=448")]
    #[case::frequencies(451, "memory limit exceeded: limit=451, used=451, requested=48")]
    #[case::slab(867, "memory limit exceeded: limit=867, used=867, requested=32768")]
    #[case::raw_bits(
        33635,
        "memory limit exceeded: limit=33635, used=33635, requested=2058"
    )]
    #[case::output(35693, "memory limit exceeded: limit=35693, used=35693, requested=24")]
    fn every_decoded_allocation_is_charged(#[case] limit: u32, #[case] expected: &str) {
        #[rustfmt::skip]
        let data = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x04, 0x69,
        ];
        let mut d = Decoder::with_max_size(limit);
        let err = decode_vertices(&data, Some(&[0, 3]), 6, &mut d).unwrap_err();
        assert_eq!(err.to_string(), expected);
    }

    #[test]
    fn rejects_a_state_that_does_not_end_on_the_initial_state() {
        #[rustfmt::skip]
        let data = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x04, 0x69,
        ];
        let err = decode_vertices(&data, Some(&[0, 3]), 6, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: lanes do not end in the initial state");
    }

    #[test]
    fn rejects_an_unread_refill_word() {
        #[rustfmt::skip]
        let data = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x06,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x04, 0x69,
        ];
        let err = decode_vertices(&data, Some(&[0, 3]), 6, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: lanes do not end in the initial state");
    }

    #[rstest]
    #[case::a_trailing_byte(&[0x04, 0x69, 0x00])]
    #[case::a_missing_byte(&[0x04])]
    fn rejects_raw_bits_of_the_wrong_length(#[case] raw: &[u8]) {
        #[rustfmt::skip]
        let lanes = [
            0x01,
            0x63, 0xC1, 0xFF, 0x52, 0xE8, 0x00, 0x61, 0x81, 0xF0, 0x00, 0x42, 0x08,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
        ];
        let data = [&lanes[..], raw].concat();
        let err = decode_vertices(&data, Some(&[0, 3]), 6, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "rANS vertex stream is malformed: raw bits length"
        );
    }

    #[test]
    fn rejects_an_odd_word_count() {
        let err = decode_vertices(&[0x00], None, 5, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: odd vertex word count");
    }

    #[test]
    fn refuses_to_encode_an_odd_word_count() {
        let err = encode_vertices(&[1, 2, 3], None).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: odd vertex word count");
    }

    #[test]
    fn refuses_to_encode_runs_that_miss_the_vertices() {
        let err = encode_vertices(&[1, 2], Some(&[0, 2])).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: runs do not cover the vertices");
    }

    #[rstest]
    #[case::not_starting_at_zero(&[1, 3])]
    #[case::one_past_the_vertices(&[0, 5, 3])]
    fn rejects_runs_that_miss_the_vertices(#[case] runs: &[u32]) {
        let err = decode_vertices(&[0x00], Some(runs), 6, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "rANS vertex stream is malformed: runs do not cover the vertices"
        );
    }

    #[rstest]
    #[case::empty(&[], "rANS vertex stream is malformed: empty")]
    #[case::tables_cut_short(&[0], "rANS vertex stream is malformed: tables truncated")]
    #[case::no_lane_lengths(&[0, 0, 0], "buffer underflow: needed 1 bytes, but only 0 remain")]
    #[case::lanes_cut_short(&[&[0, 0, 0, 4, 4, 4, 4][..], &[0; 15]].concat(), "rANS vertex stream is malformed: lanes truncated")]
    #[case::a_lane_shorter_than_its_state(&[&[0, 0, 0, 2, 4, 4, 4][..], &[0; 14]].concat(), "rANS vertex stream is malformed: lane length")]
    #[case::a_lane_of_odd_length(&[&[0, 0, 0, 5, 4, 4, 4][..], &[0; 17]].concat(), "rANS vertex stream is malformed: lane length")]
    fn rejects_a_malformed_layout(#[case] data: &[u8], #[case] expected: &str) {
        let err = decode_vertices(data, None, 0, &mut dec()).unwrap_err();
        assert_eq!(err.to_string(), expected);
    }

    #[test]
    fn rejects_a_symbol_from_a_missing_table() {
        #[rustfmt::skip]
        let data = [
            0x00,
            0x00, 0x00,
            0x04, 0x04, 0x04, 0x04,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
        ];
        let err = decode_vertices(&data, None, 2, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: symbol from a missing table");
    }

    fn first_table(depth: u8, write: fn(&mut BitWriter)) -> Vec<u8> {
        let mut w = BitWriter::default();
        w.put(1, 1);
        write(&mut w);
        [vec![depth], w.finish()].concat()
    }

    #[rstest]
    #[case::precision_past_eleven(0, |w: &mut BitWriter| { w.put(12, 4); w.put(0, 9); }, "table out of range")]
    #[case::max_symbol_past_256(0, |w: &mut BitWriter| { w.put(0, 4); w.put(257, 9); }, "table out of range")]
    #[case::gamma_past_twelve_zeros(0, |w: &mut BitWriter| { w.put(1, 4); w.put(1, 9); w.put(0, 13); w.put(1, 1); }, "frequency out of range")]
    #[case::gamma_of_twelve_zeros(0, |w: &mut BitWriter| { w.put(1, 4); w.put(0, 9); w.gamma(4096); }, "table frequencies")]
    #[case::frequencies_off_the_precision(0, |w: &mut BitWriter| { w.put(1, 4); w.put(1, 9); w.gamma(2); w.gamma(3); }, "table frequencies")]
    #[case::max_symbol_without_slots(0, |w: &mut BitWriter| { w.put(1, 4); w.put(1, 9); w.gamma(3); w.gamma(1); }, "table frequencies")]
    #[case::symbol_below_the_depth(1, |w: &mut BitWriter| { w.put(0, 4); w.put(1, 9); }, "table frequencies")]
    #[case::positive_i32_min(0, |w: &mut BitWriter| { w.put(0, 4); w.put(63, 9); }, "table frequencies")]
    #[case::i32_min_with_a_mantissa_at_depth_1(1, |w: &mut BitWriter| { w.put(0, 4); w.put(130, 9); }, "table frequencies")]
    #[case::positive_i32_min_at_depth_2(2, |w: &mut BitWriter| { w.put(0, 4); w.put(255, 9); }, "table frequencies")]
    fn rejects_a_malformed_table(
        #[case] depth: u8,
        #[case] write: fn(&mut BitWriter),
        #[case] expected: &str,
    ) {
        let err = decode_vertices(&first_table(depth, write), None, 0, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("rANS vertex stream is malformed: {expected}")
        );
    }

    fn deltas_of_every_symbol(depth: u32) -> Vec<i32> {
        let mut deltas = vec![0];
        for len in 1..=32_u32 {
            let kept = (len - 1).min(depth);
            for mantissa in 0..1_u32 << kept {
                let mag = u64::from((1 << kept) | mantissa) << (len - 1 - kept);
                deltas.extend(i32::try_from(mag).ok());
                deltas.extend(i32::try_from(-i64::try_from(mag).unwrap()).ok());
            }
        }
        deltas
    }

    #[rstest]
    #[case::depth_0(0, 64)]
    #[case::depth_1(1, 128)]
    #[case::depth_2(2, 256)]
    fn a_table_may_name_exactly_the_symbols_of_some_delta(
        #[case] depth: u32,
        #[case] largest: usize,
    ) {
        let mut mapped: Vec<usize> = deltas_of_every_symbol(depth)
            .into_iter()
            .map(|d| symbolize(d, depth).0)
            .collect();
        mapped.sort_unstable();
        mapped.dedup();
        let named: Vec<usize> = (0..SYMBOLS).filter(|&s| info(s, depth).is_some()).collect();
        assert_eq!(named, mapped);
        assert_eq!(named.last(), Some(&largest));
    }

    #[test]
    fn i32_min_decodes_at_every_depth() {
        let vertices = [0, 0, i32::MIN, i32::MIN, 0, 0];
        let first = run_starts(None, 3).unwrap();
        for depth in 0..=MAX_DEPTH {
            let data = encode_at(&vertices, &first, depth);
            assert_eq!(
                decode_vertices(&data, None, 6, &mut dec()).unwrap(),
                vertices
            );
        }
    }

    #[test]
    fn every_depth_decodes() {
        let vertices: Vec<i32> = (0..400)
            .flat_map(|i| [i * 37 % 1000, (i * i) % 4093 - 2000])
            .collect();
        let offsets = [0, 100, 150, 400];
        let first = run_starts(Some(&offsets), 400).unwrap();
        for depth in 0..=MAX_DEPTH {
            let data = encode_at(&vertices, &first, depth);
            assert_eq!(
                decode_vertices(&data, Some(&offsets), 800, &mut dec()).unwrap(),
                vertices
            );
        }
    }

    #[rstest]
    #[case::min_slab(1_000, MIN_SLAB)]
    #[case::twice_min_slab(50_000, 2 * MIN_SLAB)]
    #[case::four_times_min_slab(100_000, 4 * MIN_SLAB)]
    #[case::max_slab(400_000, MAX_SLAB)]
    fn a_wide_spread_of_deltas_takes_a_larger_slab(#[case] n: u32, #[case] slab: usize) {
        let mut seed = 1_u32;
        let vertices: Vec<i32> = (0..2 * n)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                i32::try_from(seed >> 16).unwrap() >> (seed % 16)
            })
            .collect();
        let offsets: Vec<u32> = (0..=n / 5).map(|i| 5 * i).collect();
        let data = encode_vertices(&vertices, Some(&offsets)).unwrap();
        let mut d = dec();
        assert_eq!(
            decode_vertices(&data, Some(&offsets), 2 * n, &mut d).unwrap(),
            vertices
        );
        assert_eq!(d.rans_slots.len(), slab);
    }

    #[test]
    fn rejects_runs_short_of_the_vertices() {
        let data = encode_vertices(&[1, 2, 3, 4], Some(&[0, 2])).unwrap();
        let err = decode_vertices(&data, Some(&[0, 1]), 4, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: runs do not cover the vertices");
    }

    #[test]
    fn rejects_runs_past_the_vertices() {
        let data = encode_vertices(&[1, 2, 3, 4], Some(&[0, 2])).unwrap();
        let err = decode_vertices(&data, Some(&[0, 3]), 4, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: runs do not cover the vertices");
    }

    #[test]
    fn rejects_depth_above_two() {
        let mut data = encode_vertices(&[1, 2, 3, 4], Some(&[0, 2])).unwrap();
        data[0] = 3;
        let err = decode_vertices(&data, Some(&[0, 2]), 4, &mut dec()).unwrap_err();
        assert_snapshot!(err, @"rANS vertex stream is malformed: depth");
    }

    fn partition(n: usize, cuts: Vec<usize>) -> Vec<u32> {
        let mut offsets: Vec<u32> = std::iter::once(0)
            .chain(cuts.into_iter().map(|c| c % (n + 1)))
            .chain(std::iter::once(n))
            .map(|o| u32::try_from(o).unwrap())
            .collect();
        offsets.sort_unstable();
        offsets
    }

    proptest! {
        #[test]
        fn round_trips(
            vertices in prop::collection::vec(any::<(i16, i16)>(), 0..300),
            cuts in prop::collection::vec(0..300_usize, 0..20),
        ) {
            let flat: Vec<i32> = vertices.iter().flat_map(|&(x, y)| [i32::from(x), i32::from(y)]).collect();
            let offsets = partition(vertices.len(), cuts);
            prop_assert_eq!(round_trip(&flat, Some(&offsets)), flat);
        }

        #[test]
        fn truncation_is_an_error(
            vertices in prop::collection::vec(any::<(i32, i32)>(), 1..50),
            cut in any::<prop::sample::Index>(),
        ) {
            let flat: Vec<i32> = vertices.iter().flat_map(|&(x, y)| [x, y]).collect();
            let offsets = [0, u32::try_from(vertices.len()).unwrap()];
            let data = encode_vertices(&flat, Some(&offsets)).unwrap();
            let cut = cut.index(data.len());
            prop_assert!(decode_vertices(&data[..cut], Some(&offsets), words(&flat), &mut dec()).is_err());
        }

        #[test]
        fn arbitrary_bytes_do_not_panic(
            data in prop::collection::vec(any::<u8>(), 0..200),
            vertex_count in 0..40_u32,
        ) {
            let _ = decode_vertices(&data, Some(&[0, vertex_count]), 2 * vertex_count, &mut dec());
        }
    }
}
