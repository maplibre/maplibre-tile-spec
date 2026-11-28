//! Predicted vertices: residuals against a guess from the decoded vertices, Rice coded with parameters per class.

use rustc_hash::FxHashMap;
use usize_cast::IntoUsize as _;

use crate::MltError::MalformedPredictedVertices as Malformed;
use crate::codecs::varint::parse_varint;
use crate::codecs::vertex_copies::{self, Copier};
use crate::errors::AsMltError as _;
use crate::{Decoder, MltResult};

/// A ring or line part, the only runs a guess looks back into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunKind {
    Line,
    Ring,
}

/// One ring or line part: `len` vertices from `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub start: u32,
    pub len: u32,
    pub kind: RunKind,
}

/// The guess for the first vertex of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartGuess {
    /// The previous vertex.
    Previous,
    /// The first vertex of the previous run.
    RunStart,
}

/// The guess for a line vertex from the third on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineGuess {
    /// The previous vertex.
    Previous,
    /// The last edge continued, in line parts of six or more.
    ContinuedFromSix,
    /// The last edge continued, in line parts of three or more.
    ContinuedFromThree,
}

/// The guesses a payload was written with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guesses {
    pub start: StartGuess,
    pub line: LineGuess,
}

/// Which vertices a payload takes from earlier vertices instead of from a residual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sharing {
    /// Runs of vertices copied from earlier vertices.
    pub copies: bool,
    /// Line part endpoints that name a vertex of an earlier line part.
    pub line_references: bool,
}

/// What a payload's header byte holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub guesses: Guesses,
    pub sharing: Sharing,
}

/// The class of a vertex, which picks its stream and its guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Start,
    Point,
    Ring,
    Corner,
    LineSecond,
    Line,
    ContinuedLine,
}

const CLASSES: usize = 6;
const QUAD: u32 = 4;
const COPIES: u8 = 0x08;
const LINE_REFERENCES: u8 = 0x10;
const RESERVED: u8 = 0xE0;
/// The z parameter byte flag of a class whose z residuals are all zero and have no codeword.
const Z_ZERO: u8 = 0x10;
const Z_RESERVED: u8 = 0xE0;

impl LineGuess {
    /// The shortest line part whose later vertices continue the last edge.
    fn min_len(self) -> Option<u32> {
        match self {
            Self::Previous => None,
            Self::ContinuedFromSix => Some(6),
            Self::ContinuedFromThree => Some(3),
        }
    }
}

impl Guesses {
    /// Every pair in ascending header byte order.
    const ALL: [Self; 6] = [
        Self::new(StartGuess::Previous, LineGuess::Previous),
        Self::new(StartGuess::RunStart, LineGuess::Previous),
        Self::new(StartGuess::Previous, LineGuess::ContinuedFromSix),
        Self::new(StartGuess::RunStart, LineGuess::ContinuedFromSix),
        Self::new(StartGuess::Previous, LineGuess::ContinuedFromThree),
        Self::new(StartGuess::RunStart, LineGuess::ContinuedFromThree),
    ];

    const fn new(start: StartGuess, line: LineGuess) -> Self {
        Self { start, line }
    }

    fn bits(self) -> u8 {
        let start = match self.start {
            StartGuess::Previous => 0,
            StartGuess::RunStart => 1,
        };
        let line = match self.line {
            LineGuess::Previous => 0,
            LineGuess::ContinuedFromSix => 1,
            LineGuess::ContinuedFromThree => 2,
        };
        start | (line << 1)
    }

    fn from_bits(byte: u8) -> MltResult<Self> {
        let start = if byte & 1 == 0 {
            StartGuess::Previous
        } else {
            StartGuess::RunStart
        };
        let line = match (byte >> 1) & 0b11 {
            0 => LineGuess::Previous,
            1 => LineGuess::ContinuedFromSix,
            2 => LineGuess::ContinuedFromThree,
            _ => return Err(Malformed("unknown line guess")),
        };
        Ok(Self { start, line })
    }

    /// Whether these guesses can write streams over `runs` that no lower header byte writes.
    fn useful(self, runs: &[Run]) -> bool {
        let lines = || runs.iter().filter(|r| r.kind == RunKind::Line);
        let starts = self.start == StartGuess::Previous || runs.iter().any(|r| r.len > 0);
        let continued = match self.line {
            LineGuess::Previous => true,
            LineGuess::ContinuedFromSix => lines().any(|r| r.len >= 6),
            LineGuess::ContinuedFromThree => lines().any(|r| (3..6).contains(&r.len)),
        };
        starts && continued
    }
}

impl Sharing {
    const NONE: Self = Self {
        copies: false,
        line_references: false,
    };

    fn any(self) -> bool {
        self.copies || self.line_references
    }
}

impl Header {
    fn byte(self) -> u8 {
        let copies = if self.sharing.copies { COPIES } else { 0 };
        let line_references = if self.sharing.line_references {
            LINE_REFERENCES
        } else {
            0
        };
        self.guesses.bits() | copies | line_references
    }

    fn from_byte(byte: u8) -> MltResult<Self> {
        if byte & RESERVED != 0 {
            return Err(Malformed("reserved header bit set"));
        }
        Ok(Self {
            guesses: Guesses::from_bits(byte)?,
            sharing: Sharing {
                copies: byte & COPIES != 0,
                line_references: byte & LINE_REFERENCES != 0,
            },
        })
    }
}

impl Tag {
    const fn stream(self) -> usize {
        match self {
            Self::Start => 0,
            Self::Point => 1,
            Self::Ring => 2,
            Self::Corner => 3,
            Self::LineSecond => 4,
            Self::Line | Self::ContinuedLine => 5,
        }
    }
}

/// The vertices a payload shares instead of storing their residuals, and the ops and bits that say which.
#[derive(Clone)]
struct Plan {
    sharing: Sharing,
    shared: Vec<bool>,
    ops: Vec<u8>,
    bits: Vec<u8>,
}

impl Plan {
    fn unshared(vertex_count: usize) -> Self {
        Self {
            sharing: Sharing::NONE,
            shared: vec![false; vertex_count],
            ops: Vec::new(),
            bits: Vec::new(),
        }
    }

    /// The copies that save more of the zigzagged `residuals` than their ops take.
    fn copies<const D: usize>(vertices: &[i32], runs: &[Run], residuals: &[u32]) -> Self {
        let saved: Vec<usize> = residuals
            .as_chunks::<D>()
            .0
            .iter()
            .map(|v| {
                v.iter()
                    .map(|&r| vertex_copies::varint_len(r.into_usize()))
                    .sum()
            })
            .collect();
        let copies = vertex_copies::plan::<D>(vertices, runs, &saved);
        let mut ops = Vec::new();
        vertex_copies::write_ops(&copies, &mut ops);
        Self {
            sharing: Sharing {
                copies: true,
                line_references: false,
            },
            shared: vertex_copies::mask(&copies, saved.len()),
            ops,
            bits: Vec::new(),
        }
    }

    /// This plan with a reference for every line part endpoint it does not share yet and an earlier line part holds.
    fn with_line_references<const D: usize>(&self, vertices: &[i32], runs: &[Run]) -> Self {
        let pos = |i: usize| vertices.as_chunks::<D>().0[i];
        let mut shared = self.shared.clone();
        let mut bits = BitWriter::default();
        let mut slots = 0_usize;
        let mut first_slot: FxHashMap<[i32; D], usize> = FxHashMap::default();
        for run in runs.iter().filter(|r| r.kind == RunKind::Line && r.len > 0) {
            let start = run.start.into_usize();
            let end = start + run.len.into_usize();
            if slots > 0 {
                for i in endpoints(start, end) {
                    if shared[i] {
                        continue;
                    }
                    let hit = first_slot.get(&pos(i)).copied();
                    bits.put(usize::from(hit.is_some()), 1);
                    if let Some(slot) = hit {
                        bits.put(slot, index_bits(slots));
                        shared[i] = true;
                    }
                }
            }
            for i in start..end {
                first_slot.entry(pos(i)).or_insert(slots + i - start);
            }
            slots += end - start;
        }
        Self {
            sharing: Sharing {
                line_references: true,
                ..self.sharing
            },
            shared,
            ops: self.ops.clone(),
            bits: bits.bytes,
        }
    }

    /// The plan, or `None` when it shares no vertex.
    fn useful(self) -> Option<Self> {
        self.shared.contains(&true).then_some(self)
    }
}

/// The first and, when it differs, the last vertex of the line part `start..end`.
fn endpoints(start: usize, end: usize) -> impl Iterator<Item = usize> {
    std::iter::once(start).chain((end - 1 > start).then_some(end - 1))
}

/// The bits an index into `count` slots takes.
fn index_bits(count: usize) -> u32 {
    if count <= 1 {
        0
    } else {
        (count - 1).ilog2() + 1
    }
}

/// Writes bits most significant first.
struct BitWriter {
    bytes: Vec<u8>,
    used: u32,
}

impl Default for BitWriter {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            used: 8,
        }
    }
}

impl BitWriter {
    fn put(&mut self, value: usize, bits: u32) {
        for bit in (0..bits).rev() {
            if self.used == 8 {
                self.bytes.push(0);
                self.used = 0;
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= u8::from((value >> bit) & 1 == 1) << (7 - self.used);
            self.used += 1;
        }
    }
}

/// Reads bits most significant first.
struct BitReader<'a> {
    data: &'a [u8],
    at: usize,
}

impl BitReader<'_> {
    fn take(&mut self, bits: u32) -> MltResult<usize> {
        let mut value = 0_usize;
        for _ in 0..bits {
            let Some(byte) = self.data.get(self.at / 8) else {
                return Err(Malformed("reference bits are cut short"));
            };
            value = (value << 1) | usize::from((byte >> (7 - self.at % 8)) & 1);
            self.at += 1;
        }
        Ok(value)
    }

    /// Checks that only zero padding follows the last bit read.
    fn finish(&self) -> MltResult<()> {
        let padding_is_zero = self
            .data
            .get(self.at / 8)
            .is_none_or(|&byte| byte & (0xFF >> (self.at % 8)) == 0);
        if self.at.div_ceil(8) == self.data.len() && padding_is_zero {
            Ok(())
        } else {
            Err(Malformed("bytes after the last reference bit"))
        }
    }
}

/// Reads line part endpoint references while the vertices are decoded in order.
struct LineReferences<'a> {
    bits: BitReader<'a>,
    /// The first slot and the first vertex of every completed line part.
    parts: Vec<[usize; 2]>,
    /// The vertices of every completed line part.
    slots: usize,
}

impl<'a> LineReferences<'a> {
    fn new(bits: &'a [u8], runs: &[Run], dec: &mut Decoder) -> MltResult<Self> {
        let lines = runs
            .iter()
            .filter(|r| r.kind == RunKind::Line && r.len > 0)
            .count();
        Ok(Self {
            bits: BitReader { data: bits, at: 0 },
            parts: dec.alloc::<[usize; 2]>(lines)?,
            slots: 0,
        })
    }

    /// Whether a line part endpoint has an earlier line part to refer to.
    fn active(&self) -> bool {
        self.slots > 0
    }

    /// The vertex that a line part endpoint refers to, when its flag is set.
    fn reference<const D: usize>(&mut self, vertices: &[[i32; D]]) -> MltResult<Option<[i32; D]>> {
        if self.bits.take(1)? == 0 {
            return Ok(None);
        }
        let slot = self.bits.take(index_bits(self.slots))?;
        if slot >= self.slots {
            return Err(Malformed("reference index past the slots"));
        }
        let part = self.parts.partition_point(|&[first, _]| first <= slot) - 1;
        let [first, start] = self.parts[part];
        Ok(Some(vertices[start + slot - first]))
    }

    /// Takes the `len` vertices of the line part at `start` into the slots.
    fn completed(&mut self, start: usize, len: usize) {
        self.parts.push([self.slots, start]);
        self.slots += len;
    }
}

/// The header of a payload.
pub fn header(data: &[u8]) -> MltResult<Header> {
    let (&byte, _) = data.split_first().ok_or(Malformed("no header byte"))?;
    Header::from_byte(byte)
}

/// What the residuals of one class cost.
#[derive(Clone, Copy)]
struct ClassCost<const D: usize> {
    axes: [RiceCosts; D],
    vertices: usize,
}

/// The parameters a class is written with.
#[derive(Clone, Copy)]
struct ClassParams<const D: usize> {
    ks: [u32; D],
    /// Every z residual is zero and has no codeword.
    z_zero: bool,
}

impl<const D: usize> ClassCost<D> {
    const NONE: Self = Self {
        axes: [RiceCosts::NONE; D],
        vertices: 0,
    };

    /// The parameter bytes of a class that holds a residual: one for `x` and `y`, and one for `z` on triples.
    const PARAM_BYTES: usize = D - 1;

    fn add(&mut self, residual: &[u32; D]) {
        self.vertices += 1;
        for (axis, &r) in self.axes.iter_mut().zip(residual) {
            axis.add(r);
        }
    }

    fn sub(&mut self, other: &Self) {
        for (axis, other) in self.axes.iter_mut().zip(&other.axes) {
            axis.sub(other);
        }
        self.vertices -= other.vertices;
    }

    /// The parameters of the class, and the codeword bits it takes with them.
    fn params(&self) -> (ClassParams<D>, u64) {
        let mut ks = [0; D];
        let mut bits = 0;
        for (k, axis) in ks.iter_mut().zip(&self.axes) {
            let (best, cost) = axis.best();
            *k = best;
            bits += cost;
        }
        let z_zero = D == 3 && self.axes[D - 1].all_zero();
        if z_zero {
            bits -= self.axes[D - 1].best().1;
        }
        (ClassParams { ks, z_zero }, bits)
    }
}

/// The cost of the vertices `keep` selects, per class.
fn class_costs<const D: usize>(
    residuals: &[u32],
    tags: &[Tag],
    keep: impl Fn(usize) -> bool,
) -> [ClassCost<D>; CLASSES] {
    let mut costs = [ClassCost::NONE; CLASSES];
    for (i, (&tag, residual)) in tags.iter().zip(residuals.as_chunks::<D>().0).enumerate() {
        if keep(i) {
            costs[tag.stream()].add(residual);
        }
    }
    costs
}

/// The length `write` gives a payload, from the class costs of the vertices it stores.
fn payload_len<const D: usize>(
    header: Header,
    plan: &Plan,
    totals: &[ClassCost<D>; CLASSES],
    stored: &[ClassCost<D>; CLASSES],
) -> usize {
    let mut len = 1 + plan.ops.len() + plan.bits.len();
    if header.sharing.any() {
        for (total, stored) in totals.iter().zip(stored) {
            if total.vertices > 0 {
                len += vertex_copies::varint_len(total.vertices - stored.vertices);
            }
        }
    }
    if header.sharing.copies && header.sharing.line_references {
        len += vertex_copies::varint_len(plan.ops.len());
    }
    let written = || stored.iter().filter(|c| c.vertices > 0);
    let bits: u64 = written().map(|c| c.params().1).sum();
    let bytes = usize::try_from(bits.div_ceil(8)).unwrap_or(usize::MAX);
    if header.sharing.any() {
        len += vertex_copies::varint_len(bytes);
    }
    len + written().count() * ClassCost::<D>::PARAM_BYTES + bytes
}

/// The shortest payload so far.
struct Best {
    len: usize,
    header: Header,
    tags: Vec<Tag>,
    residuals: Vec<u32>,
    plan: Plan,
}

/// The shortest payload for interleaved `(x, y)` pairs (`D = 2`) or `(x, y, z)` triples (`D = 3`) over `runs`, the lowest header byte on a tie.
pub fn encode<const D: usize>(vertices: &[i32], runs: &[Run]) -> MltResult<Vec<u8>> {
    let n = vertex_count::<D>(vertices)?;
    classify(runs, n, LineGuess::Previous)?;
    let unshared = Plan::unshared(n);
    let line_references = unshared.with_line_references::<D>(vertices, runs).useful();
    let mut best: Option<Best> = None;
    for guesses in Guesses::ALL.into_iter().filter(|g| g.useful(runs)) {
        let tags = classify(runs, n, guesses.line)?;
        let residuals = residuals::<D>(vertices, &tags, guesses.start);
        let copies = Plan::copies::<D>(vertices, runs, &residuals).useful();
        let both = copies
            .as_ref()
            .map(|c| c.with_line_references::<D>(vertices, runs));
        let plans = [
            Some(&unshared),
            copies.as_ref(),
            line_references.as_ref(),
            both.as_ref(),
        ];
        let totals = class_costs::<D>(&residuals, &tags, |_| true);
        for plan in plans.into_iter().flatten() {
            let mut stored = totals;
            if plan.sharing.any() {
                let shared = class_costs::<D>(&residuals, &tags, |i| plan.shared[i]);
                for (stored, shared) in stored.iter_mut().zip(&shared) {
                    stored.sub(shared);
                }
            }
            let header = Header {
                guesses,
                sharing: plan.sharing,
            };
            let len = payload_len(header, plan, &totals, &stored);
            if best
                .as_ref()
                .is_none_or(|b| (len, header.byte()) < (b.len, b.header.byte()))
            {
                best = Some(Best {
                    len,
                    header,
                    tags: tags.clone(),
                    residuals: residuals.clone(),
                    plan: plan.clone(),
                });
            }
        }
    }
    let best = best.expect("the previous-vertex guesses are always useful");
    let payload = write::<D>(&best.residuals, &best.tags, best.header, &best.plan);
    debug_assert_eq!(payload.len(), best.len);
    Ok(payload)
}

/// The guesses [`encode_fast`] writes with.
const FAST_GUESSES: Guesses = Guesses::new(StartGuess::RunStart, LineGuess::ContinuedFromSix);

/// The payload for interleaved `(x, y)` pairs (`D = 2`) or `(x, y, z)` triples (`D = 3`) over `runs`, with one fixed pair of guesses and no shared vertices.
pub fn encode_fast<const D: usize>(vertices: &[i32], runs: &[Run]) -> MltResult<Vec<u8>> {
    let n = vertex_count::<D>(vertices)?;
    let header = Header {
        guesses: FAST_GUESSES,
        sharing: Sharing::NONE,
    };
    let tags = classify(runs, n, header.guesses.line)?;
    let residuals = residuals::<D>(vertices, &tags, header.guesses.start);
    Ok(write::<D>(&residuals, &tags, header, &Plan::unshared(n)))
}

/// The payload for interleaved `D`-component `vertices` over `runs` with `header`.
#[cfg(test)]
fn encode_as<const D: usize>(vertices: &[i32], runs: &[Run], header: Header) -> MltResult<Vec<u8>> {
    let n = vertex_count::<D>(vertices)?;
    let tags = classify(runs, n, header.guesses.line)?;
    let residuals = residuals::<D>(vertices, &tags, header.guesses.start);
    let mut plan = if header.sharing.copies {
        Plan::copies::<D>(vertices, runs, &residuals)
    } else {
        Plan::unshared(n)
    };
    if header.sharing.line_references {
        plan = plan.with_line_references::<D>(vertices, runs);
    }
    Ok(write::<D>(&residuals, &tags, header, &plan))
}

fn vertex_count<const D: usize>(vertices: &[i32]) -> MltResult<usize> {
    const { assert!(D == 2 || D == 3, "a vertex is a pair or a triple") };
    if vertices.len().is_multiple_of(D) {
        Ok(vertices.len() / D)
    } else if D == 2 {
        Err(Malformed("odd vertex word count"))
    } else {
        Err(Malformed("vertex word count is not a multiple of three"))
    }
}

/// The zigzagged residual of every vertex, interleaved.
fn residuals<const D: usize>(vertices: &[i32], tags: &[Tag], start: StartGuess) -> Vec<u32> {
    let vertices = vertices.as_chunks::<D>().0;
    let mut out = Vec::with_capacity(D * vertices.len());
    let mut run_start = [0; D];
    for (i, (&tag, v)) in tags.iter().zip(vertices).enumerate() {
        let g = guess(vertices, i, tag, start, run_start);
        out.extend(v.iter().zip(g).map(|(&v, g)| zigzag(v.wrapping_sub(g))));
        if tag == Tag::Start {
            run_start = *v;
        }
    }
    out
}

fn write<const D: usize>(residuals: &[u32], tags: &[Tag], header: Header, plan: &Plan) -> Vec<u8> {
    let mut stored = [ClassCost::<D>::NONE; CLASSES];
    let mut totals = [0_usize; CLASSES];
    let mut shared = [0_usize; CLASSES];
    let chunks = residuals.as_chunks::<D>().0;
    for (i, (&tag, residual)) in tags.iter().zip(chunks).enumerate() {
        let stream = tag.stream();
        totals[stream] += 1;
        if plan.shared[i] {
            shared[stream] += 1;
        } else {
            stored[stream].add(residual);
        }
    }
    let params = stored.map(|c| c.params().0);
    let mut out = Vec::with_capacity(residuals.len() + plan.ops.len() + plan.bits.len() + 8);
    out.push(header.byte());
    if header.sharing.any() {
        for (&total, &count) in totals.iter().zip(&shared) {
            if total > 0 {
                write_varint(count, &mut out);
            }
        }
    }
    if header.sharing.copies && header.sharing.line_references {
        write_varint(plan.ops.len(), &mut out);
    }
    for (class, p) in stored.iter().zip(&params) {
        if class.vertices > 0 {
            out.push(p.ks[0].to_le_bytes()[0] | p.ks[1].to_le_bytes()[0] << 4);
            if D == 3 {
                out.push(if p.z_zero {
                    Z_ZERO
                } else {
                    p.ks[2].to_le_bytes()[0]
                });
            }
        }
    }
    let mut bits = LsbBitWriter::default();
    for (i, (&tag, residual)) in tags.iter().zip(chunks).enumerate() {
        if !plan.shared[i] {
            let p = &params[tag.stream()];
            let coded = if p.z_zero { 2 } else { D };
            for (&v, &k) in residual.iter().zip(&p.ks).take(coded) {
                put_rice(v, k, &mut bits);
            }
        }
    }
    let bits = bits.finish();
    if header.sharing.any() {
        write_varint(bits.len(), &mut out);
    }
    out.extend_from_slice(&bits);
    out.extend_from_slice(&plan.ops);
    out.extend_from_slice(&plan.bits);
    out
}

/// Decode `vertex_count` interleaved `(x, y)` pairs (`D = 2`) or `(x, y, z)` triples (`D = 3`) over `runs`.
pub fn decode<const D: usize>(
    data: &[u8],
    runs: &[Run],
    vertex_count: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    const { assert!(D == 2 || D == 3, "a vertex is a pair or a triple") };
    let (&byte, mut rest) = data.split_first().ok_or(Malformed("no header byte"))?;
    let Header { guesses, sharing } = Header::from_byte(byte)?;
    let n = vertex_count.into_usize();
    let totals = class_lengths(runs, n)?;
    let mut lengths = totals;
    if sharing.any() {
        for (length, &total) in lengths.iter_mut().zip(&totals).filter(|(_, t)| **t > 0) {
            let (tail, count) = parse_varint::<u32>(rest)?;
            rest = tail;
            *length = total
                .checked_sub(count.into_usize())
                .ok_or(Malformed("more shared vertices than vertices in a stream"))?;
        }
    }
    let ops_len = if sharing.copies && sharing.line_references {
        let (tail, len) = parse_varint::<u32>(rest)?;
        rest = tail;
        Some(len.into_usize())
    } else {
        None
    };
    let (mut walk, rest) = Walk::<D>::new(rest, &lengths, guesses, sharing.any())?;
    let (ops, bits) = match (sharing.copies, ops_len) {
        (false, _) => (&[][..], rest),
        (true, None) => (rest, &[][..]),
        (true, Some(len)) => rest
            .split_at_checked(len)
            .ok_or(Malformed("copy ops are cut short"))?,
    };
    let mut copier = Copier::new(ops, runs, n, dec)?;
    let mut references = if sharing.line_references {
        Some(LineReferences::new(bits, runs, dec)?)
    } else {
        None
    };

    let words = n.checked_mul(D).or_overflow()?;
    let mut out = dec.alloc::<i32>(words)?;
    out.resize(words, 0);
    let vertices = out.as_chunks_mut::<D>().0;
    let mut end = 0;
    for run in runs.iter().filter(|r| r.len > 0) {
        let start = run.start.into_usize();
        if start > end {
            walk.points(&mut vertices[..start], end, &mut copier)?;
        }
        let len = run.len.into_usize();
        end = start + len;
        let line_references = match &mut references {
            Some(references) if run.kind == RunKind::Line && references.active() => {
                Some(references)
            }
            _ => None,
        };
        if line_references.is_none() && !copier.copies_before(end) {
            walk.residuals(&mut vertices[..end], start, [start, end], run.kind);
        } else {
            walk.shared_run(
                &mut vertices[..end],
                start,
                run.kind,
                &mut copier,
                line_references,
            )?;
        }
        if let Some(references) = &mut references
            && run.kind == RunKind::Line
        {
            references.completed(start, len);
        }
    }
    walk.points(vertices, end, &mut copier)?;
    walk.finish()?;
    if let Some(references) = &references {
        references.bits.finish()?;
    }
    Ok(out)
}

/// The vertex count of every class, which only the topology decides.
fn class_lengths(runs: &[Run], vertex_count: usize) -> MltResult<[usize; CLASSES]> {
    let mut lengths = [0_usize; CLASSES];
    let mut covered = 0_usize;
    let mut end = 0_usize;
    for run in runs {
        let start = run.start.into_usize();
        let len = run.len.into_usize();
        if start < end {
            return Err(Malformed("runs overlap or are out of order"));
        }
        if len == 0 {
            continue;
        }
        let Some(run_end) = start.checked_add(len).filter(|&e| e <= vertex_count) else {
            return Err(Malformed("a run ends past the last vertex"));
        };
        end = run_end;
        covered += len;
        lengths[Tag::Start.stream()] += 1;
        match run.kind {
            RunKind::Ring if run.len == QUAD => {
                lengths[Tag::Ring.stream()] += 2;
                lengths[Tag::Corner.stream()] += 1;
            }
            RunKind::Ring => lengths[Tag::Ring.stream()] += len - 1,
            RunKind::Line => {
                lengths[Tag::LineSecond.stream()] += usize::from(len >= 2);
                lengths[Tag::Line.stream()] += len.saturating_sub(2);
            }
        }
    }
    lengths[Tag::Point.stream()] = vertex_count - covered;
    Ok(lengths)
}

/// The decoder state between runs: the parameters of each class over the one bit data.
struct Walk<'a, const D: usize> {
    bits: Bits<'a>,
    params: [Params; CLASSES],
    start_guess: StartGuess,
    /// The shortest line part whose later vertices continue the last edge.
    continued_from: usize,
    /// The first vertex of the latest run.
    run_start: [i32; D],
}

impl<'a, const D: usize> Walk<'a, D> {
    /// Reads the parameters of the classes with `lengths` vertices off `data`, then locates the bit data.
    ///
    /// With `delimited`, a `byte_len` varint precedes the bit data, which otherwise runs to the end.
    fn new(
        data: &'a [u8],
        lengths: &[usize; CLASSES],
        guesses: Guesses,
        delimited: bool,
    ) -> MltResult<(Self, &'a [u8])> {
        let mut params = [Params::default(); CLASSES];
        let mut rest = data;
        for (p, &len) in params.iter_mut().zip(lengths) {
            if len > 0 {
                let Some((bytes, tail)) = rest.split_at_checked(D - 1) else {
                    return Err(Malformed("rice parameters are cut short"));
                };
                *p = Params::new(bytes[0], len);
                if D == 3 {
                    p.set_z(bytes[1])?;
                }
                rest = tail;
            }
        }
        let data;
        if delimited {
            let (tail, byte_len) = parse_varint::<u32>(rest)?;
            (data, rest) = tail
                .split_at_checked(byte_len.into_usize())
                .ok_or(Malformed("rice bits are cut short"))?;
        } else {
            (data, rest) = (rest, &[][..]);
        }
        let walk = Self {
            bits: Bits {
                data,
                ..Bits::default()
            },
            params,
            start_guess: guesses.start,
            continued_from: guesses.line.min_len().map_or(usize::MAX, u32::into_usize),
            run_start: [0; D],
        };
        Ok((walk, rest))
    }

    /// The reader of class `c`.
    #[expect(clippy::inline_always, reason = "keeps the bit buffer in registers")]
    #[inline(always)]
    fn rice(&mut self, c: usize) -> Rice<'_, 'a, D> {
        Rice {
            bits: &mut self.bits,
            p: &mut self.params[c],
        }
    }

    /// Decodes the points from `from` to the end of `vertices`, from copies or residuals.
    fn points(
        &mut self,
        vertices: &mut [[i32; D]],
        from: usize,
        copier: &mut Copier<'_>,
    ) -> MltResult<()> {
        let mut i = from;
        while i < vertices.len() {
            let shared = copier.copies(i, vertices)?;
            if shared > 0 {
                i += shared;
                continue;
            }
            let stop = copier.next().min(vertices.len());
            let mut p = previous(vertices, i);
            self.rice(POINT).fill(&mut vertices[i..stop], |r| {
                p = add(p, r);
                p
            });
            i = stop;
        }
        Ok(())
    }

    /// Decodes the run from `start` to the end of `vertices`, where copies or line `references` may share vertices.
    fn shared_run(
        &mut self,
        vertices: &mut [[i32; D]],
        start: usize,
        kind: RunKind,
        copier: &mut Copier<'_>,
        mut references: Option<&mut LineReferences<'_>>,
    ) -> MltResult<()> {
        let end = vertices.len();
        let last = end - 1;
        let mut i = start;
        while i < end {
            let shared = copier.copies(i, vertices)?;
            if shared > 0 {
                i += shared;
                continue;
            }
            if let Some(references) = references.as_deref_mut()
                && (i == start || i == last)
                && let Some(p) = references.reference(vertices)?
            {
                vertices[i] = p;
                i += 1;
                continue;
            }
            let mut stop = copier.next().min(end);
            if references.is_some() && i < last {
                stop = stop.min(last);
            }
            self.residuals(vertices, start, [i, stop], kind);
            i = stop;
        }
        self.run_start = vertices[start];
        Ok(())
    }

    /// Decodes the vertices `from..to` of the run from `start` to the end of `vertices` from their residuals.
    fn residuals(
        &mut self,
        vertices: &mut [[i32; D]],
        start: usize,
        [from, to]: [usize; 2],
        kind: RunKind,
    ) {
        let len = vertices.len() - start;
        let mut k = from - start;
        if k == 0 {
            let guess = match self.start_guess {
                StartGuess::Previous => previous(vertices, start),
                StartGuess::RunStart => self.run_start,
            };
            self.run_start = add(guess, self.rice(START).residual());
            vertices[start] = self.run_start;
            k = 1;
        }
        let run = &mut vertices[start..to];
        if k >= run.len() {
            return;
        }
        match kind {
            RunKind::Ring if len == QUAD.into_usize() && k == 1 && run.len() == len => {
                let mut ring = self.rice(RING);
                run[1] = add(run[0], ring.residual());
                run[2] = add(run[1], ring.residual());
                run[3] = add(corner(run), self.rice(CORNER).residual());
            }
            RunKind::Ring if len == QUAD.into_usize() => {
                for k in k..run.len() {
                    if k == 3 {
                        run[3] = add(corner(run), self.rice(CORNER).residual());
                    } else {
                        run[k] = add(run[k - 1], self.rice(RING).residual());
                    }
                }
            }
            RunKind::Ring => self.rice(RING).deltas(&mut run[k - 1..]),
            RunKind::Line => {
                if k == 1 {
                    run[1] = add(run[0], self.rice(LINE_SECOND).residual());
                    k = 2;
                }
                if k < run.len() {
                    if len >= self.continued_from {
                        self.rice(LINE).continued(&mut run[k - 2..]);
                    } else {
                        self.rice(LINE).deltas(&mut run[k - 1..]);
                    }
                }
            }
        }
    }

    /// Checks that the vertices took every residual of every class and only padding is left.
    fn finish(&self) -> MltResult<()> {
        for p in &self.params {
            p.finish()?;
        }
        self.bits.finish()
    }
}

const START: usize = Tag::Start.stream();
const POINT: usize = Tag::Point.stream();
const RING: usize = Tag::Ring.stream();
const CORNER: usize = Tag::Corner.stream();
const LINE_SECOND: usize = Tag::LineSecond.stream();
const LINE: usize = Tag::Line.stream();

/// The guess for the fourth vertex of a ring of four, `p[0] + p[2] - p[1]`.
fn corner<const D: usize>(run: &[[i32; D]]) -> [i32; D] {
    std::array::from_fn(|axis| {
        run[0][axis]
            .wrapping_add(run[2][axis])
            .wrapping_sub(run[1][axis])
    })
}

/// The vertex before `i`, or the origin for the first.
fn previous<const D: usize>(vertices: &[[i32; D]], i: usize) -> [i32; D] {
    i.checked_sub(1).map_or([0; D], |p| vertices[p])
}

fn add<const D: usize>(a: [i32; D], b: [i32; D]) -> [i32; D] {
    std::array::from_fn(|axis| a[axis].wrapping_add(b[axis]))
}

/// The Rice bit data, read least significant bit first through a 64 bit buffer.
#[derive(Clone, Copy, Default)]
struct Bits<'a> {
    data: &'a [u8],
    /// The next byte of `data` to buffer.
    at: usize,
    buf: u64,
    /// The bits of `buf` that hold data.
    filled: u32,
}

impl Bits<'_> {
    #[inline]
    fn refill(&mut self) {
        if let Some(word) = self.data.get(self.at..).and_then(<[u8]>::first_chunk::<8>) {
            self.buf |= u64::from_le_bytes(*word) << self.filled;
            self.at += usize::try_from((63 - self.filled) / 8).expect("at most 7");
            self.filled |= 0x38;
            return;
        }
        while self.filled <= 56 {
            let Some(&byte) = self.data.get(self.at) else {
                return;
            };
            self.buf |= u64::from(byte) << self.filled;
            self.filled += 8;
            self.at += 1;
        }
    }

    /// Checks that only zero padding follows the last codeword.
    fn finish(&self) -> MltResult<()> {
        if self.at != self.data.len() || self.filled >= 8 {
            return Err(Malformed("bytes after the last rice value"));
        }
        if self.buf != 0 {
            return Err(Malformed("rice padding bit is set"));
        }
        Ok(())
    }
}

/// The parameters of one class and the vertices it has left.
#[derive(Clone, Copy, Default)]
struct Params {
    ks: [u32; 2],
    /// The low `k` bits of each axis.
    masks: [u32; 2],
    /// The bits a pair takes at most without an escape.
    pair_bits: u32,
    /// The bits of a pair that do not depend on its quotients: both terminators and both remainders.
    fixed_bits: u32,
    kz: u32,
    z_mask: u32,
    /// The bits a z takes at most without an escape.
    z_bits: u32,
    /// Every z residual is zero and has no codeword.
    z_zero: bool,
    /// The vertices left to read.
    remaining: usize,
    /// The first malformed condition met, zero while the class is sound.
    bad: u8,
}

/// Failure codes, one more than the index into `REASONS`.
const CUT_SHORT: u8 = 1;
const WIDE_ESCAPE: u8 = 2;
const PAST_U32: u8 = 3;
const DISAGREE: u8 = 4;
const REASONS: [&str; 4] = [
    "rice bits are cut short",
    "rice escape wider than 32 bits",
    "rice value exceeds 32 bits",
    "shared vertices disagree with the stream lengths",
];

impl Params {
    /// The parameters an `(x, y)` byte holds, for a class of `vertices`.
    fn new(byte: u8, vertices: usize) -> Self {
        let ks = [u32::from(byte & 0x0F), u32::from(byte >> 4)];
        Self {
            ks,
            masks: ks.map(|k| (1_u32 << k) - 1),
            fixed_bits: 2 + ks[0] + ks[1],
            pair_bits: 2 * RICE_ESCAPE_Q + ks[0] + ks[1],
            remaining: vertices,
            ..Self::default()
        }
    }

    /// Takes the parameter of `z` from its byte.
    fn set_z(&mut self, byte: u8) -> MltResult<()> {
        if byte & Z_RESERVED != 0 {
            return Err(Malformed("reserved z parameter bit set"));
        }
        let kz = u32::from(byte & 0x0F);
        self.z_zero = byte & Z_ZERO != 0;
        if self.z_zero && kz != 0 {
            return Err(Malformed("z parameter with the zero flag has a nonzero k"));
        }
        self.kz = kz;
        self.z_mask = (1_u32 << kz) - 1;
        self.z_bits = RICE_ESCAPE_Q + kz;
        Ok(())
    }

    /// Checks that every vertex of the class was read and none was malformed.
    fn finish(&self) -> MltResult<()> {
        if self.bad != 0 {
            return Err(Malformed(REASONS[usize::from(self.bad - 1)]));
        }
        if self.remaining > 0 {
            return Err(Malformed(
                "shared vertices disagree with the stream lengths",
            ));
        }
        Ok(())
    }
}

/// The reader of one class: its parameters over the shared bit data.
struct Rice<'r, 'a, const D: usize> {
    bits: &'r mut Bits<'a>,
    p: &'r mut Params,
}

impl<const D: usize> Rice<'_, '_, D> {
    /// Bits an escaped value takes at most, the largest read between two refills.
    const MAX_BITS: u32 = RICE_ESCAPE_Q + 6 + 32;

    /// Decodes each vertex of `run` after the first as the one before it plus its residual.
    fn deltas(&mut self, run: &mut [[i32; D]]) {
        let Some((first, later)) = run.split_first_mut() else {
            return;
        };
        let mut p = *first;
        self.fill(later, |r| {
            p = add(p, r);
            p
        });
    }

    /// Decodes each vertex of `run` after the second as the last edge continued plus its residual.
    fn continued(&mut self, run: &mut [[i32; D]]) {
        let [mut a, mut b, ..] = *run else {
            return;
        };
        self.fill(&mut run[2..], |r| {
            let guess = std::array::from_fn(|axis| b[axis].wrapping_mul(2).wrapping_sub(a[axis]));
            (a, b) = (b, add(guess, r));
            b
        });
    }

    /// The value of the next residual, with parameter `k`.
    #[inline]
    fn value(&mut self, k: u32) -> Result<u32, u8> {
        let b = &mut *self.bits;
        if b.filled < Self::MAX_BITS {
            b.refill();
        }
        let zeros = b.buf.trailing_zeros();
        if zeros < RICE_ESCAPE_Q {
            let used = zeros + 1 + k;
            if used > b.filled {
                return Err(CUT_SHORT);
            }
            let low = (b.buf >> (zeros + 1)) & ((1_u64 << k) - 1);
            b.buf >>= used;
            b.filled -= used;
            return Ok(zeros << k | u32::try_from(low).expect("below 2^15"));
        }
        if b.filled < RICE_ESCAPE_Q + 6 {
            return Err(CUT_SHORT);
        }
        let n = u32::try_from((b.buf >> RICE_ESCAPE_Q) & 0x3F).expect("six bits");
        if n > 32 {
            return Err(WIDE_ESCAPE);
        }
        let used = RICE_ESCAPE_Q + 6 + n;
        if used > b.filled {
            return Err(CUT_SHORT);
        }
        let e = (b.buf >> (RICE_ESCAPE_Q + 6)) & ((1_u64 << n) - 1);
        let value = (u64::from(RICE_ESCAPE_Q) << k) + e;
        let Ok(value) = u32::try_from(value) else {
            return Err(PAST_U32);
        };
        b.buf >>= used;
        b.filled -= used;
        Ok(value)
    }

    /// The residual of the next vertex.
    #[expect(
        clippy::inline_always,
        reason = "measured 34M fewer instructions than #[inline] on the omt tiles"
    )]
    #[inline(always)]
    fn residual(&mut self) -> [i32; D] {
        if D == 3 && !self.p.z_zero {
            self.residual_with::<true>()
        } else {
            self.residual_with::<false>()
        }
    }

    /// The residual of the next vertex, with a `z` codeword when `Z`.
    #[expect(clippy::inline_always, reason = "the body of `residual`")]
    #[inline(always)]
    fn residual_with<const Z: bool>(&mut self) -> [i32; D] {
        if !self.reserve(1) {
            return [0; D];
        }
        if self.bits.filled < self.p.pair_bits {
            self.bits.refill();
        }
        let (mut buf, mut filled) = (self.bits.buf, self.bits.filled);
        if filled >= self.p.pair_bits
            && let Some(pair) = fast_pair(self.p, &mut buf, &mut filled)
        {
            (self.bits.buf, self.bits.filled) = (buf, filled);
            let mut r = [0; D];
            r[..2].copy_from_slice(&pair);
            if Z {
                r[2] = self.z();
            }
            return r;
        }
        self.vertex_exact::<Z>()
    }

    /// Claims `vertices` residuals of the class, false when it has fewer left.
    #[inline]
    fn reserve(&mut self, vertices: usize) -> bool {
        let Some(remaining) = self.p.remaining.checked_sub(vertices) else {
            self.fail(DISAGREE);
            return false;
        };
        self.p.remaining = remaining;
        true
    }

    /// Records the first malformed condition, which `finish` reports.
    #[cold]
    #[inline(never)]
    fn fail(&mut self, reason: u8) {
        if self.p.bad == 0 {
            self.p.bad = reason;
        }
    }

    /// Sets each vertex of `out` to `step` of the next residual.
    #[expect(
        clippy::inline_always,
        reason = "measured 34M fewer instructions than #[inline] on the omt tiles"
    )]
    #[inline(always)]
    fn fill(&mut self, out: &mut [[i32; D]], step: impl FnMut([i32; D]) -> [i32; D]) {
        if D == 3 && !self.p.z_zero {
            self.fill_with::<true>(out, step);
        } else {
            self.fill_with::<false>(out, step);
        }
    }

    /// The body of `fill`, with a `z` codeword per vertex when `Z`, keeping the bit buffer and the parameters in registers.
    #[expect(clippy::inline_always, reason = "the body of `fill`")]
    #[inline(always)]
    fn fill_with<const Z: bool>(
        &mut self,
        out: &mut [[i32; D]],
        mut step: impl FnMut([i32; D]) -> [i32; D],
    ) {
        if !self.reserve(out.len()) {
            return;
        }
        let p = *self.p;
        let (mut buf, mut filled) = (self.bits.buf, self.bits.filled);
        for v in out.iter_mut() {
            if filled < p.pair_bits {
                (self.bits.buf, self.bits.filled) = (buf, filled);
                self.bits.refill();
                (buf, filled) = (self.bits.buf, self.bits.filled);
            }
            if filled >= p.pair_bits
                && let Some(pair) = fast_pair(&p, &mut buf, &mut filled)
            {
                let mut r = [0; D];
                r[..2].copy_from_slice(&pair);
                if Z {
                    if filled < p.z_bits {
                        (self.bits.buf, self.bits.filled) = (buf, filled);
                        self.bits.refill();
                        (buf, filled) = (self.bits.buf, self.bits.filled);
                    }
                    let fast = if filled >= p.z_bits {
                        fast_z(&p, &mut buf, &mut filled)
                    } else {
                        None
                    };
                    r[2] = if let Some(z) = fast {
                        z
                    } else {
                        (self.bits.buf, self.bits.filled) = (buf, filled);
                        let z = self.z_exact();
                        (buf, filled) = (self.bits.buf, self.bits.filled);
                        z
                    };
                }
                *v = step(r);
                continue;
            }
            (self.bits.buf, self.bits.filled) = (buf, filled);
            *v = step(self.vertex_exact::<Z>());
            (buf, filled) = (self.bits.buf, self.bits.filled);
        }
        (self.bits.buf, self.bits.filled) = (buf, filled);
    }

    /// The `z` residual of the next vertex after its `x` and `y`.
    #[inline]
    fn z(&mut self) -> i32 {
        if self.bits.filled < self.p.z_bits {
            self.bits.refill();
        }
        let (mut buf, mut filled) = (self.bits.buf, self.bits.filled);
        if filled >= self.p.z_bits
            && let Some(z) = fast_z(self.p, &mut buf, &mut filled)
        {
            (self.bits.buf, self.bits.filled) = (buf, filled);
            return z;
        }
        self.z_exact()
    }

    /// The `z` residual of the next vertex after its `x` and `y`, with every cut-short check.
    #[inline(never)]
    fn z_exact(&mut self) -> i32 {
        match self.value(self.p.kz) {
            Ok(z) => unzigzag(z),
            Err(reason) => {
                self.fail(reason);
                0
            }
        }
    }

    /// The residual of the next vertex, with a `z` codeword when `Z` and every cut-short check.
    #[inline(never)]
    fn vertex_exact<const Z: bool>(&mut self) -> [i32; D] {
        let x = self.value(self.p.ks[0]);
        let y = self.value(self.p.ks[1]);
        let z = if Z { self.value(self.p.kz) } else { Ok(0) };
        match (x, y, z) {
            (Ok(x), Ok(y), Ok(z)) => {
                let mut r = [0; D];
                r[..2].copy_from_slice(&[unzigzag(x), unzigzag(y)]);
                if Z {
                    r[2] = unzigzag(z);
                }
                r
            }
            (Err(reason), _, _) | (_, Err(reason), _) | (_, _, Err(reason)) => {
                self.fail(reason);
                [0; D]
            }
        }
    }
}

/// The `(x, y)` residual of the next vertex when it takes no escape, given that `buf` holds `pair_bits` bits.
#[expect(
    clippy::inline_always,
    reason = "the bulk loop and the single reader share one body"
)]
#[inline(always)]
fn fast_pair(p: &Params, buf: &mut u64, filled: &mut u32) -> Option<[i32; 2]> {
    let [kx, ky] = p.ks;
    let [mx, my] = p.masks;
    let zx = buf.trailing_zeros();
    if zx >= RICE_ESCAPE_Q {
        return None;
    }
    let rest = *buf >> (zx + 1);
    let lx = u32::try_from(rest & 0xFFFF_FFFF).expect("masked") & mx;
    let rest = rest >> kx;
    let zy = rest.trailing_zeros();
    if zy >= RICE_ESCAPE_Q {
        return None;
    }
    let ly = u32::try_from((rest >> (zy + 1)) & 0xFFFF_FFFF).expect("masked") & my;
    *buf = rest >> (zy + 1 + ky);
    *filled -= zx + zy + p.fixed_bits;
    Some([unzigzag(zx << kx | lx), unzigzag(zy << ky | ly)])
}

/// The `z` residual of the next vertex when it takes no escape, given that `buf` holds `z_bits` bits.
#[expect(
    clippy::inline_always,
    reason = "the bulk loop and the single reader share one body"
)]
#[inline(always)]
fn fast_z(p: &Params, buf: &mut u64, filled: &mut u32) -> Option<i32> {
    let zz = buf.trailing_zeros();
    if zz >= RICE_ESCAPE_Q {
        return None;
    }
    let rest = *buf >> (zz + 1);
    let lz = u32::try_from(rest & 0xFFFF_FFFF).expect("masked") & p.z_mask;
    *buf = rest >> p.kz;
    *filled -= zz + 1 + p.kz;
    Some(unzigzag(zz << p.kz | lz))
}

/// The unary quotient from which a Rice value escapes.
const RICE_ESCAPE_Q: u32 = 16;

/// The bits `v` takes with parameter `k`.
fn rice_cost(v: u32, k: u32) -> u64 {
    let q = v >> k;
    if q < RICE_ESCAPE_Q {
        u64::from(q) + 1 + u64::from(k)
    } else {
        let e = v - (RICE_ESCAPE_Q << k);
        u64::from(RICE_ESCAPE_Q) + 6 + u64::from(e.bit_width())
    }
}

/// The Rice cost of values for every parameter, gathered in one pass and additive over values.
#[derive(Clone, Copy)]
struct RiceCosts {
    /// The bits of the values wider than `k` bits, per `k`.
    wide: [u64; 16],
    /// The count of values per bit width.
    widths: [u64; 33],
}

impl RiceCosts {
    const NONE: Self = Self {
        wide: [0; 16],
        widths: [0; 33],
    };

    fn add(&mut self, v: u32) {
        let width = v.bit_width() as usize;
        self.widths[width] += 1;
        for (k, cost) in (0..).zip(&mut self.wide[..width.min(16)]) {
            *cost += rice_cost(v, k);
        }
    }

    fn sub(&mut self, other: &Self) {
        for (a, b) in self.wide.iter_mut().zip(&other.wide) {
            *a -= b;
        }
        for (a, b) in self.widths.iter_mut().zip(&other.widths) {
            *a -= b;
        }
    }

    /// Whether every value is zero.
    fn all_zero(&self) -> bool {
        self.widths[1..].iter().all(|&n| n == 0)
    }

    /// The bits all values take per parameter.
    fn bits(&self) -> [u64; 16] {
        let mut out = self.wide;
        let mut narrow = 0_u64;
        for (k, cost) in out.iter_mut().enumerate() {
            narrow += self.widths[k];
            *cost += narrow * (k as u64 + 1);
        }
        out
    }

    /// The parameter in `0..=15` that takes the fewest bits, and those bits.
    fn best(&self) -> (u32, u64) {
        (0..)
            .zip(self.bits())
            .min_by_key(|&(_, c)| c)
            .unwrap_or((0, 0))
    }
}

/// Writes bits least significant first.
#[derive(Default)]
struct LsbBitWriter {
    bytes: Vec<u8>,
    acc: u64,
    held: u32,
}

impl LsbBitWriter {
    /// Appends the low `bits` (at most 32) of `value`.
    fn put(&mut self, value: u64, bits: u32) {
        self.acc |= (value & ((1_u64 << bits) - 1)) << self.held;
        self.held += bits;
        while self.held >= 8 {
            self.bytes.push(self.acc.to_le_bytes()[0]);
            self.acc >>= 8;
            self.held -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.held > 0 {
            self.bytes.push(self.acc.to_le_bytes()[0]);
        }
        self.bytes
    }
}

/// Appends the codeword of `v` with parameter `k`.
fn put_rice(value: u32, k: u32, bits: &mut LsbBitWriter) {
    let quotient = value >> k;
    if quotient < RICE_ESCAPE_Q {
        bits.put(0, quotient);
        bits.put(1, 1);
        bits.put(u64::from(value), k);
    } else {
        let excess = value - (RICE_ESCAPE_Q << k);
        let width = excess.bit_width();
        bits.put(0, RICE_ESCAPE_Q);
        bits.put(u64::from(width), 6);
        bits.put(u64::from(excess), width);
    }
}

/// The tag of every vertex, which only the topology and the line guess decide.
fn classify(runs: &[Run], vertex_count: usize, line: LineGuess) -> MltResult<Vec<Tag>> {
    let mut tags = vec![Tag::Point; vertex_count];
    let mut end = 0_usize;
    for run in runs {
        let start = run.start.into_usize();
        let len = run.len.into_usize();
        if start < end {
            return Err(Malformed("runs overlap or are out of order"));
        }
        if len == 0 {
            continue;
        }
        end = start
            .checked_add(len)
            .filter(|&e| e <= vertex_count)
            .ok_or(Malformed("a run ends past the last vertex"))?;
        let tag = &mut tags[start..end];
        tag[0] = Tag::Start;
        match run.kind {
            RunKind::Ring => {
                tag[1..].fill(Tag::Ring);
                if run.len == QUAD {
                    tag[3] = Tag::Corner;
                }
            }
            RunKind::Line => {
                if let Some(second) = tag.get_mut(1) {
                    *second = Tag::LineSecond;
                }
                if let Some(later) = tag.get_mut(2..) {
                    let continued = line.min_len().is_some_and(|m| run.len >= m);
                    later.fill(if continued {
                        Tag::ContinuedLine
                    } else {
                        Tag::Line
                    });
                }
            }
        }
    }
    Ok(tags)
}

/// The guess for vertex `i`, every component from the same vertices.
fn guess<const D: usize>(
    vertices: &[[i32; D]],
    i: usize,
    tag: Tag,
    start: StartGuess,
    run_start: [i32; D],
) -> [i32; D] {
    match (tag, start) {
        (Tag::Start, StartGuess::RunStart) => run_start,
        (Tag::Corner, _) => corner(&vertices[i - 3..i]),
        (Tag::ContinuedLine, _) => std::array::from_fn(|axis| {
            vertices[i - 1][axis]
                .wrapping_mul(2)
                .wrapping_sub(vertices[i - 2][axis])
        }),
        _ => previous(vertices, i),
    }
}

fn zigzag(v: i32) -> u32 {
    (v.cast_unsigned() << 1) ^ (v >> 31).cast_unsigned()
}

fn unzigzag(v: u32) -> i32 {
    (v >> 1).cast_signed() ^ -((v & 1).cast_signed())
}

pub(super) fn write_varint(mut v: usize, out: &mut Vec<u8>) {
    loop {
        #[expect(clippy::cast_possible_truncation, reason = "seven bits")]
        let low = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(low);
            return;
        }
        out.push(low | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::__private::dec;

    fn ring(start: u32, len: u32) -> Run {
        Run {
            start,
            len,
            kind: RunKind::Ring,
        }
    }

    fn line(start: u32, len: u32) -> Run {
        Run {
            start,
            len,
            kind: RunKind::Line,
        }
    }

    const SHARINGS: [Sharing; 4] = [
        Sharing {
            copies: false,
            line_references: false,
        },
        Sharing {
            copies: true,
            line_references: false,
        },
        Sharing {
            copies: false,
            line_references: true,
        },
        Sharing {
            copies: true,
            line_references: true,
        },
    ];

    #[test]
    fn a_ring_and_a_line_encode_to_the_exact_payload() {
        let vertices = [
            100, 200, 130, 200, 130, 240, 100, 240, 105, 205, 115, 205, 125, 207,
        ];
        let runs = [ring(0, 4), line(4, 3)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::ContinuedFromThree,
            },
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
        assert_eq!(
            payload,
            [
                0x05, 0x76, 0x54, 0x00, 0x03, 0x10, 0x88, 0x20, 0x04, 0x39, 0x08, 0x84, 0x57, 0x2A,
                0xC8, 0x09
            ]
        );
        assert_eq!(
            decode::<2>(&payload, &runs, 7, &mut dec()).unwrap(),
            vertices
        );
    }

    #[test]
    fn a_copied_wall_and_a_line_junction_encode_to_the_exact_payload() {
        let vertices = [
            0, 0, 100, 0, 100, 100, 0, 100, 100, 100, 100, 0, 200, 0, 200, 100, 200, 100, 300, 100,
            300, 100, 300, 200,
        ];
        let runs = [ring(0, 4), ring(4, 4), line(8, 2), line(10, 2)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::Previous,
            },
            sharing: Sharing {
                copies: true,
                line_references: true,
            },
        };
        let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
        assert_eq!(
            payload,
            [
                0x19, 0x02, 0x01, 0x00, 0x00, 0x03, 0x06, 0x57, 0x00, 0x66, 0x0E, 0x81, 0x22, 0x83,
                0x00, 0x20, 0x5A, 0x64, 0x30, 0x22, 0x11, 0x09, 0x04, 0x10, 0x01, 0x04, 0x05, 0x01,
                0xC0
            ]
        );
        assert_eq!(
            decode::<2>(&payload, &runs, 12, &mut dec()).unwrap(),
            vertices
        );
    }

    #[rstest]
    fn every_header_round_trips_mixed_runs(
        #[values(StartGuess::Previous, StartGuess::RunStart)] start: StartGuess,
        #[values(
            LineGuess::Previous,
            LineGuess::ContinuedFromSix,
            LineGuess::ContinuedFromThree
        )]
        line_guess: LineGuess,
        #[values(SHARINGS[0], SHARINGS[1], SHARINGS[2], SHARINGS[3])] sharing: Sharing,
    ) {
        let vertices = [
            5,
            5,
            100,
            200,
            130,
            200,
            130,
            240,
            100,
            240,
            0,
            0,
            10,
            2,
            21,
            4,
            31,
            7,
            40,
            10,
            50,
            12,
            70,
            20,
            130,
            240,
            130,
            200,
            0,
            0,
            -1,
            1,
            300,
            -70_000,
            0,
            0,
            10,
            2,
            21,
            4,
            40,
            10,
            300,
            -70_000,
            i32::MIN,
            i32::MAX,
            i32::MAX,
            i32::MIN,
        ];
        let runs = [
            ring(1, 4),
            line(5, 7),
            ring(12, 4),
            line(16, 2),
            line(18, 4),
            ring(22, 2),
        ];
        let header = Header {
            guesses: Guesses {
                start,
                line: line_guess,
            },
            sharing,
        };
        let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
        assert_eq!(super::header(&payload).unwrap(), header);
        assert_eq!(
            decode::<2>(&payload, &runs, 24, &mut dec()).unwrap(),
            vertices
        );
    }

    fn rice_points_payload(bits: &[u8]) -> Vec<u8> {
        [&[0x00, 0x00], bits].concat()
    }

    fn rice_error(payload: &[u8], vertex_count: u32) -> String {
        decode::<2>(payload, &[], vertex_count, &mut dec())
            .unwrap_err()
            .to_string()
    }

    #[rstest]
    #[case::no_sharing(SHARINGS[0])]
    #[case::copies(SHARINGS[1])]
    #[case::line_references(SHARINGS[2])]
    #[case::copies_and_line_references(SHARINGS[3])]
    fn a_rice_layer_of_rings_lines_points_and_shared_vertices_round_trips(
        #[case] sharing: Sharing,
    ) {
        let vertices = [
            0, 0, 100, 0, 100, 100, 0, 100, 100, 100, 100, 0, 200, 0, 200, 100, 200, 100, 300, 100,
            300, 100, 300, 200, 300, 200, 400, 250, 450, 255, 7, 7, 9, 9,
        ];
        let runs = [ring(0, 4), ring(4, 4), line(8, 2), line(10, 2), line(12, 3)];
        for guesses in Guesses::ALL {
            let header = Header { guesses, sharing };
            let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
            assert_eq!(super::header(&payload).unwrap(), header);
            assert_eq!(
                decode::<2>(&payload, &runs, 17, &mut dec()).unwrap(),
                vertices
            );
        }
    }

    #[test]
    fn a_rice_layer_with_escaped_values_round_trips() {
        let vertices = [0, 0, i32::MAX, i32::MIN, 0, 0, 1_000_000, -1_000_000, 3, 3];
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &[], header).unwrap();
        assert_eq!(decode::<2>(&payload, &[], 5, &mut dec()).unwrap(), vertices);
    }

    #[test]
    fn k_zero_rice_values_are_unary_bits_least_significant_first() {
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&[0, 0, 0, 0], &[], header).unwrap();
        assert_eq!(payload, [0x00, 0x00, 0x0F]);
    }

    #[test]
    fn a_quotient_of_sixteen_escapes_to_the_width_and_bits_of_v_less_sixteen_shifted_by_k() {
        let vertices = [-1, 0, -2, 0, -3, 0, -4, 0, -5, 0, -6, 0, -7, 0, 33, 0];
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &[], header).unwrap();
        assert_eq!(
            payload,
            [0x00, 0x01, 0xFF, 0xFF, 0x1F, 0x00, 0xC0, 0x80, 0x03]
        );
    }

    #[test]
    fn k_fifteen_rice_values_round_trip_with_both_nibbles_set() {
        let vertices = [250_000, -262_144, 450_000, -37_144];
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &[], header).unwrap();
        assert_eq!(payload[1], 0xFF);
        assert_eq!(decode::<2>(&payload, &[], 2, &mut dec()).unwrap(), vertices);
    }

    #[test]
    fn rice_bit_data_cut_short_is_rejected() {
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&[1, 1, 2, 2], &[], header).unwrap();
        let (&last, shorter) = payload.split_last().unwrap();
        assert_ne!(last, 0);
        assert_eq!(
            rice_error(shorter, 2),
            "predicted vertex stream is malformed: rice bits are cut short"
        );
    }

    #[test]
    fn a_rice_byte_after_the_last_value_is_rejected() {
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let mut payload = encode_as::<2>(&[1, 1, 2, 2], &[], header).unwrap();
        payload.push(0);
        assert_eq!(
            rice_error(&payload, 2),
            "predicted vertex stream is malformed: bytes after the last rice value"
        );
    }

    #[test]
    fn a_set_rice_padding_bit_is_rejected() {
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let mut payload = encode_as::<2>(&[1, 1, 2, 2], &[], header).unwrap();
        *payload.last_mut().unwrap() |= 0x80;
        assert_eq!(
            rice_error(&payload, 2),
            "predicted vertex stream is malformed: rice padding bit is set"
        );
    }

    #[test]
    fn a_rice_escape_wider_than_32_bits_is_rejected() {
        let mut bits = LsbBitWriter::default();
        bits.put(0, 16);
        bits.put(33, 6);
        bits.put(0, 32);
        bits.put(0, 32);
        assert_eq!(
            rice_error(&rice_points_payload(&bits.finish()), 1),
            "predicted vertex stream is malformed: rice escape wider than 32 bits"
        );
    }

    #[test]
    fn a_rice_escape_past_u32_is_rejected() {
        let mut bits = LsbBitWriter::default();
        bits.put(0, 16);
        bits.put(32, 6);
        bits.put(u64::from(u32::MAX), 32);
        bits.put(0, 32);
        let mut payload = rice_points_payload(&bits.finish());
        payload[1] = 0x0F;
        assert_eq!(
            rice_error(&payload, 1),
            "predicted vertex stream is malformed: rice value exceeds 32 bits"
        );
    }

    #[test]
    fn a_rice_stream_with_every_prefix_cut_never_panics() {
        let vertices = [0, 0, i32::MAX, i32::MIN, 5, 5, -300, 4000];
        let header = Header {
            guesses: Guesses::ALL[0],
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &[], header).unwrap();
        for cut in 0..payload.len() {
            assert!(decode::<2>(&payload[..cut], &[], 4, &mut dec()).is_err());
        }
    }

    #[test]
    fn an_index_takes_the_ceiling_of_log2_bits_and_none_for_one_slot() {
        assert_eq!(
            [0, 1, 2, 3, 4, 5, 8, 9].map(index_bits),
            [0, 0, 1, 2, 2, 3, 3, 4]
        );
    }

    #[test]
    fn the_race_keeps_the_lowest_header_byte_on_a_tie() {
        let payload = encode::<2>(&[1, 2, 3, 4], &[line(0, 2)]).unwrap();
        assert_eq!(payload, [0x00, 0x10, 0x11, 0x24, 0x22]);
    }

    #[test]
    fn the_race_keeps_the_run_start_guess_for_parallel_lines() {
        let vertices = [0, 0, 1000, 0, 1, 1, 1001, 1];
        let payload = encode::<2>(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().guesses,
            Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::Previous,
            }
        );
    }

    #[test]
    fn the_race_continues_a_straight_line_of_three() {
        let vertices = [0, 0, 500, 500, 1000, 1000];
        let payload = encode::<2>(&vertices, &[line(0, 3)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().guesses,
            Guesses {
                start: StartGuess::Previous,
                line: LineGuess::ContinuedFromThree,
            }
        );
    }

    #[test]
    fn the_race_copies_a_shared_wall() {
        let vertices = [
            0, 0, 1_000_000, 0, 1_000_000, 1_000_000, 0, 1_000_000, 1_000_000, 1_000_000,
            1_000_000, 0, 2_000_000, 0, 2_000_000, 1_000_000,
        ];
        let payload = encode::<2>(&vertices, &[ring(0, 4), ring(4, 4)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().sharing,
            Sharing {
                copies: true,
                line_references: false,
            }
        );
    }

    #[test]
    fn the_race_refers_to_a_line_junction() {
        let vertices = [
            0, 0, 1_000_000, 1_000_000, 5_000_000, 0, 1_000_000, 1_000_000,
        ];
        let payload = encode::<2>(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().sharing,
            Sharing {
                copies: false,
                line_references: true,
            }
        );
    }

    #[test]
    fn a_line_at_one_height_encodes_to_the_exact_payload_with_the_zero_flag_after_its_start() {
        let vertices = [100, 200, 250, 110, 205, 250, 120, 210, 250, 135, 212, 250];
        let runs = [line(0, 4)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::Previous,
                line: LineGuess::Previous,
            },
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<3>(&vertices, &runs, header).unwrap();
        assert_eq!(
            payload,
            [
                0x00, 0x87, 0x08, 0x23, 0x10, 0x24, 0x10, 0x22, 0x85, 0x94, 0x9E, 0xA4, 0x12, 0xD5,
                0x05
            ]
        );
        assert_eq!(
            decode::<3>(&payload, &runs, 4, &mut dec()).unwrap(),
            vertices
        );
    }

    #[test]
    fn a_sloped_ring_of_four_encodes_to_the_exact_payload_with_its_corner_exact_in_z() {
        let vertices = [0, 0, 10, 100, 0, 12, 100, 100, 15, 0, 100, 13];
        let runs = [ring(0, 4)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::Previous,
                line: LineGuess::Previous,
            },
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<3>(&vertices, &runs, header).unwrap();
        assert_eq!(
            payload,
            [
                0x00, 0x00, 0x03, 0x66, 0x02, 0x00, 0x10, 0x93, 0x88, 0x04, 0x24, 0x80, 0x88, 0x0E
            ]
        );
        assert_eq!(
            decode::<3>(&payload, &runs, 4, &mut dec()).unwrap(),
            vertices
        );
    }

    #[rstest]
    #[case::coded_z(&[0x00, 0x00, 0x00, 0x07], [0, 0, 0])]
    #[case::zero_flag(&[0x00, 0x00, 0x10, 0x03], [0, 0, 0])]
    #[case::coded_z_with_k_one(&[0x00, 0x00, 0x01, 0x0F], [0, 0, -1])]
    fn a_triple_point_reads_z_after_x_and_y_unless_its_class_has_the_zero_flag(
        #[case] data: &[u8],
        #[case] expected: [i32; 3],
    ) {
        assert_eq!(decode::<3>(data, &[], 1, &mut dec()).unwrap(), expected);
    }

    #[rstest]
    #[case::no_z_parameter_byte(&[0x00, 0x00], "rice parameters are cut short")]
    #[case::reserved_z_bit_5(&[0x00, 0x00, 0x20, 0x07], "reserved z parameter bit set")]
    #[case::reserved_z_bit_6(&[0x00, 0x00, 0x40, 0x07], "reserved z parameter bit set")]
    #[case::reserved_z_bit_7(&[0x00, 0x00, 0x80, 0x07], "reserved z parameter bit set")]
    #[case::zero_flag_with_k_one(&[0x00, 0x00, 0x11, 0x03], "z parameter with the zero flag has a nonzero k")]
    #[case::zero_flag_with_k_fifteen(&[0x00, 0x00, 0x1F, 0x03], "z parameter with the zero flag has a nonzero k")]
    #[case::z_codeword_cut_short(&[0x00, 0x00, 0x00, 0x03], "rice bits are cut short")]
    #[case::bits_after_a_zero_flag_class(&[0x00, 0x00, 0x10, 0x07], "rice padding bit is set")]
    fn malformed_triple_point_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode::<3>(data, &[], 1, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn the_race_refers_to_a_line_junction_at_the_same_height() {
        let vertices = [
            0, 0, 0, 1_000_000, 1_000_000, 40, 5_000_000, 0, 0, 1_000_000, 1_000_000, 40,
        ];
        let payload = encode::<3>(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().sharing,
            Sharing {
                copies: false,
                line_references: true,
            }
        );
    }

    #[test]
    fn the_race_shares_nothing_at_a_line_junction_of_two_heights() {
        let vertices = [
            0, 0, 0, 1_000_000, 1_000_000, 40, 5_000_000, 0, 0, 1_000_000, 1_000_000, 41,
        ];
        let payload = encode::<3>(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(header(&payload).unwrap().sharing, Sharing::NONE);
    }

    #[test]
    fn the_race_copies_a_shared_wall_of_triples() {
        let vertices = [
            0, 0, 5, 1_000_000, 0, 5, 1_000_000, 1_000_000, 900_000, 0, 1_000_000, 900_000,
            1_000_000, 1_000_000, 900_000, 1_000_000, 0, 5, 2_000_000, 0, 5, 2_000_000, 1_000_000,
            900_000,
        ];
        let payload = encode::<3>(&vertices, &[ring(0, 4), ring(4, 4)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().sharing,
            Sharing {
                copies: true,
                line_references: false,
            }
        );
        assert_eq!(
            decode::<3>(&payload, &[ring(0, 4), ring(4, 4)], 8, &mut dec()).unwrap(),
            vertices
        );
    }

    #[rstest]
    #[case::no_header(&[], "no header byte")]
    #[case::line_guess_three(&[0x06, 0x00, 0x03], "unknown line guess")]
    #[case::run_start_line_guess_three(&[0x07, 0x00, 0x03], "unknown line guess")]
    #[case::reserved_bit_5(&[0x20, 0x00, 0x03], "reserved header bit set")]
    #[case::reserved_bit_6(&[0x40, 0x00, 0x03], "reserved header bit set")]
    #[case::reserved_bit_7(&[0x80, 0x00, 0x03], "reserved header bit set")]
    #[case::trailing_bytes(&[0x00, 0x00, 0x03, 0x00], "bytes after the last rice value")]
    #[case::no_parameter_byte(&[0x00], "rice parameters are cut short")]
    #[case::no_bit_data(&[0x00, 0x00], "rice bits are cut short")]
    #[case::bit_data_shorter_than_its_length(&[0x08, 0x00, 0x00, 0x02, 0x03], "rice bits are cut short")]
    #[case::more_shared_than_vertices(&[0x08, 0x02], "more shared vertices than vertices in a stream")]
    #[case::shared_without_a_copy(&[0x08, 0x01, 0x00], "shared vertices disagree with the stream lengths")]
    #[case::ops_cut_short(&[0x18, 0x00, 0x01, 0x00, 0x01, 0x03], "copy ops are cut short")]
    #[case::bits_after_the_reference_flags(&[0x10, 0x00, 0x00, 0x01, 0x03, 0x00], "bytes after the last reference bit")]
    fn malformed_point_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode::<2>(data, &[], 1, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[rstest]
    #[case::no_ring_parameters(&[0x00, 0x00], "rice parameters are cut short")]
    #[case::ring_bits_cut_short(&[0x00, 0x00, 0x00, 0x03], "rice bits are cut short")]
    #[case::copy_without_a_count(&[0x08, 0x00, 0x00, 0x00, 0x00, 0x01, 0x0F, 0x01, 0x02, 0x00], "shared vertices disagree with the stream lengths")]
    #[case::count_without_a_copy(&[0x08, 0x00, 0x01, 0x00, 0x01, 0x03], "shared vertices disagree with the stream lengths")]
    fn malformed_ring_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode::<2>(data, &[ring(0, 2)], 2, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn a_shared_count_without_a_copy_on_a_class_with_no_residual_is_rejected() {
        let data = [
            0x08, 0x00, 0x01, 0xCC, 0x08, 0x00, 0x80, 0x00, 0x00, 0x00, 0x02, 0x00, 0x0A,
        ];
        let err = decode::<2>(&data, &[line(0, 2)], 2, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "predicted vertex stream is malformed: shared vertices disagree with the stream lengths"
        );
    }

    #[rstest]
    #[case::flag_cut_short(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF], "reference bits are cut short")]
    #[case::index_past_the_slots(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF, 0xE0], "reference index past the slots")]
    #[case::padding_set(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF, 0x88], "bytes after the last reference bit")]
    #[case::byte_after_the_bits(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF, 0x80, 0x00], "bytes after the last reference bit")]
    fn malformed_line_references_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let runs = [line(0, 3), line(3, 2)];
        let err = decode::<2>(data, &runs, 5, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn a_truncated_payload_is_rejected() {
        let vertices = [0, 0, 300, 0, 300, 300, 0, 300, 10, 10, 20, 20];
        let runs = [ring(0, 4), line(4, 2)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::Previous,
                line: LineGuess::Previous,
            },
            sharing: Sharing::NONE,
        };
        let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
        let err = decode::<2>(&payload[..payload.len() - 1], &runs, 6, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "predicted vertex stream is malformed: rice bits are cut short"
        );
    }

    #[rstest]
    #[case::overlapping(&[ring(0, 4), line(2, 6)], "runs overlap or are out of order")]
    #[case::past_the_end(&[line(0, 9)], "a run ends past the last vertex")]
    fn bad_runs_are_rejected(#[case] runs: &[Run], #[case] reason: &str) {
        let err = decode::<2>(&[0x00], runs, 8, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[rstest]
    #[case::overlapping(&[ring(0, 4), line(2, 6)], "runs overlap or are out of order")]
    #[case::past_the_end(&[line(0, 9)], "a run ends past the last vertex")]
    fn bad_runs_are_rejected_by_the_encoder(#[case] runs: &[Run], #[case] reason: &str) {
        let err = encode::<2>(&[0; 16], runs).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn empty_runs_are_skipped() {
        let vertices = [100, 200, 130, 200, 130, 240, 100, 240];
        let runs = [line(0, 0), ring(0, 4), ring(4, 0)];
        let header = Header {
            guesses: Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::ContinuedFromThree,
            },
            sharing: Sharing {
                copies: true,
                line_references: true,
            },
        };
        let payload = encode_as::<2>(&vertices, &runs, header).unwrap();
        assert_eq!(
            decode::<2>(&payload, &runs, 4, &mut dec()).unwrap(),
            vertices
        );
    }

    fn arb_layer(components: usize) -> impl Strategy<Value = (Vec<i32>, Vec<Run>)> {
        let shapes = prop::collection::vec((0_u32..12, any::<bool>(), 0_u32..3), 0..12);
        (shapes, any::<bool>()).prop_flat_map(move |(shapes, few_positions)| {
            let mut runs = Vec::new();
            let mut start = 0;
            for (len, ring, gap) in shapes {
                start += gap;
                let kind = if ring { RunKind::Ring } else { RunKind::Line };
                runs.push(Run { start, len, kind });
                start += len;
            }
            let words = components * start.into_usize();
            let coords = if few_positions {
                (0..3_i32).boxed()
            } else {
                any::<i32>().boxed()
            };
            (prop::collection::vec(coords, words), Just(runs))
        })
    }

    proptest! {
        #[test]
        fn rice_costs_match_the_cost_of_every_value_per_parameter(
            values in proptest::collection::vec(
                prop_oneof![0_u32..64, 0_u32..70_000, any::<u32>()],
                0..40,
            ),
        ) {
            let mut costs = RiceCosts::NONE;
            for &v in &values {
                costs.add(v);
            }
            let expected: Vec<u64> = (0..16)
                .map(|k| values.iter().map(|&v| rice_cost(v, k)).sum())
                .collect();
            prop_assert_eq!(costs.bits().to_vec(), expected);
        }

        #[test]
        fn any_layer_round_trips_every_header((vertices, runs) in arb_layer(2)) {
            let count = u32::try_from(vertices.len() / 2).unwrap();
            for guesses in Guesses::ALL {
                for sharing in SHARINGS {
                    let payload = encode_as::<2>(&vertices, &runs, Header { guesses, sharing }).unwrap();
                    prop_assert_eq!(decode::<2>(&payload, &runs, count, &mut dec()).unwrap(), vertices.clone());
                }
            }
        }

        #[test]
        fn any_layer_round_trips_the_race((vertices, runs) in arb_layer(2)) {
            let payload = encode::<2>(&vertices, &runs).unwrap();
            let count = u32::try_from(vertices.len() / 2).unwrap();
            prop_assert_eq!(decode::<2>(&payload, &runs, count, &mut dec()).unwrap(), vertices);
        }

        #[test]
        fn the_race_is_never_longer_than_any_unshared_guess_pair((vertices, runs) in arb_layer(2)) {
            let raced = encode::<2>(&vertices, &runs).unwrap();
            for guesses in Guesses::ALL {
                let unshared = Header { guesses, sharing: Sharing::NONE };
                prop_assert!(raced.len() <= encode_as::<2>(&vertices, &runs, unshared).unwrap().len());
            }
        }

        #[test]
        fn the_fast_encoder_writes_its_fixed_guesses_unshared((vertices, runs) in arb_layer(2)) {
            let unshared = Header { guesses: FAST_GUESSES, sharing: Sharing::NONE };
            prop_assert_eq!(encode_fast::<2>(&vertices, &runs).unwrap(), encode_as::<2>(&vertices, &runs, unshared).unwrap());
        }

        #[test]
        fn any_triple_layer_round_trips_every_header((vertices, runs) in arb_layer(3)) {
            let count = u32::try_from(vertices.len() / 3).unwrap();
            for guesses in Guesses::ALL {
                for sharing in SHARINGS {
                    let payload = encode_as::<3>(&vertices, &runs, Header { guesses, sharing }).unwrap();
                    prop_assert_eq!(decode::<3>(&payload, &runs, count, &mut dec()).unwrap(), vertices.clone());
                }
            }
        }

        #[test]
        fn any_triple_layer_round_trips_the_race((vertices, runs) in arb_layer(3)) {
            let payload = encode::<3>(&vertices, &runs).unwrap();
            let count = u32::try_from(vertices.len() / 3).unwrap();
            prop_assert_eq!(decode::<3>(&payload, &runs, count, &mut dec()).unwrap(), vertices);
        }

        #[test]
        fn the_fast_encoder_writes_its_fixed_guesses_unshared_on_triples((vertices, runs) in arb_layer(3)) {
            let unshared = Header { guesses: FAST_GUESSES, sharing: Sharing::NONE };
            prop_assert_eq!(encode_fast::<3>(&vertices, &runs).unwrap(), encode_as::<3>(&vertices, &runs, unshared).unwrap());
        }

        #[test]
        fn the_triple_race_is_never_longer_than_any_unshared_guess_pair((vertices, runs) in arb_layer(3)) {
            let raced = encode::<3>(&vertices, &runs).unwrap();
            for guesses in Guesses::ALL {
                let unshared = Header { guesses, sharing: Sharing::NONE };
                prop_assert!(raced.len() <= encode_as::<3>(&vertices, &runs, unshared).unwrap().len());
            }
        }
    }
}
