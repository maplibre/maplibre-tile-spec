//! Predicted vertices: residuals against a guess from the decoded vertices, in six class streams.

use rustc_hash::FxHashMap;
use usize_cast::IntoUsize as _;

use crate::MltError::MalformedPredictedVertices as Malformed;
use crate::codecs::varint::parse_varint;
use crate::codecs::vertex_copies::{self, Copier};
use crate::{Decoder, MltResult};

/// A vertex as `[x, y]`.
pub type Pos = [i32; 2];

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
/// The leading class streams written as split varints, the others as two planes.
const SPLIT_VARINT_CLASSES: usize = 2;
const QUAD: u32 = 4;
const ESCAPE: u8 = 0xFF;
const COPIES: u8 = 0x08;
const LINE_REFERENCES: u8 = 0x10;
const RESERVED: u8 = 0xE0;

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
    fn stream(self) -> usize {
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
    fn copies(vertices: &[i32], runs: &[Run], residuals: &[u32]) -> Self {
        let saved: Vec<usize> = residuals
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[x, y]| {
                vertex_copies::varint_len(x.into_usize())
                    + vertex_copies::varint_len(y.into_usize())
            })
            .collect();
        let copies = vertex_copies::plan(vertices, runs, &saved);
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
    fn with_line_references(&self, vertices: &[i32], runs: &[Run]) -> Self {
        let pos = |i: usize| -> Pos { [vertices[2 * i], vertices[2 * i + 1]] };
        let mut shared = self.shared.clone();
        let mut bits = BitWriter::default();
        let mut slots = 0_usize;
        let mut first_slot: FxHashMap<Pos, usize> = FxHashMap::default();
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
    fn reference(&mut self, vertices: &[Pos]) -> MltResult<Option<Pos>> {
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

/// The shortest payload for interleaved `(x, y)` `vertices` over `runs`, the lowest header byte on a tie.
pub fn encode(vertices: &[i32], runs: &[Run]) -> MltResult<Vec<u8>> {
    let n = vertex_count(vertices)?;
    let unshared = Plan::unshared(n);
    let line_references = unshared.with_line_references(vertices, runs).useful();
    let mut best: Option<Vec<u8>> = None;
    for guesses in Guesses::ALL.into_iter().filter(|g| g.useful(runs)) {
        let tags = classify(runs, n, guesses.line)?;
        let residuals = residuals(vertices, &tags, guesses.start);
        let copies = Plan::copies(vertices, runs, &residuals).useful();
        let both = copies
            .as_ref()
            .map(|c| c.with_line_references(vertices, runs));
        let plans = [
            Some(&unshared),
            copies.as_ref(),
            line_references.as_ref(),
            both.as_ref(),
        ];
        for plan in plans.into_iter().flatten() {
            let header = Header {
                guesses,
                sharing: plan.sharing,
            };
            let payload = write(&residuals, &tags, header, plan);
            if best
                .as_ref()
                .is_none_or(|b| (payload.len(), payload[0]) < (b.len(), b[0]))
            {
                best = Some(payload);
            }
        }
    }
    Ok(best.expect("the previous-vertex guesses are always useful"))
}

/// The payload for interleaved `(x, y)` `vertices` over `runs` with `header`.
#[cfg(test)]
fn encode_as(vertices: &[i32], runs: &[Run], header: Header) -> MltResult<Vec<u8>> {
    let n = vertex_count(vertices)?;
    let tags = classify(runs, n, header.guesses.line)?;
    let residuals = residuals(vertices, &tags, header.guesses.start);
    let mut plan = if header.sharing.copies {
        Plan::copies(vertices, runs, &residuals)
    } else {
        Plan::unshared(n)
    };
    if header.sharing.line_references {
        plan = plan.with_line_references(vertices, runs);
    }
    Ok(write(&residuals, &tags, header, &plan))
}

fn vertex_count(vertices: &[i32]) -> MltResult<usize> {
    if vertices.len().is_multiple_of(2) {
        Ok(vertices.len() / 2)
    } else {
        Err(Malformed("odd vertex word count"))
    }
}

/// The zigzagged `(x, y)` residual of every vertex, interleaved.
fn residuals(vertices: &[i32], tags: &[Tag], start: StartGuess) -> Vec<u32> {
    let mut out = Vec::with_capacity(vertices.len());
    let mut previous = (0_i32, 0_i32);
    let mut run_start = (0_i32, 0_i32);
    for (i, (&tag, v)) in tags.iter().zip(vertices.as_chunks::<2>().0).enumerate() {
        let (gx, gy) = guess(vertices, i, tag, start, previous, run_start);
        out.push(zigzag(v[0].wrapping_sub(gx)));
        out.push(zigzag(v[1].wrapping_sub(gy)));
        if tag == Tag::Start {
            run_start = (v[0], v[1]);
        }
        previous = (v[0], v[1]);
    }
    out
}

fn write(residuals: &[u32], tags: &[Tag], header: Header, plan: &Plan) -> Vec<u8> {
    let mut streams: [Vec<u32>; CLASSES] = Default::default();
    let mut totals = [0_usize; CLASSES];
    let mut shared = [0_usize; CLASSES];
    for (i, (&tag, pair)) in tags.iter().zip(residuals.as_chunks::<2>().0).enumerate() {
        let stream = tag.stream();
        totals[stream] += 1;
        if plan.shared[i] {
            shared[stream] += 1;
        } else {
            streams[stream].extend_from_slice(pair);
        }
    }
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
    for (class, stream) in streams.iter().enumerate() {
        if stream.is_empty() {
            continue;
        }
        if class < SPLIT_VARINT_CLASSES {
            write_split_varint(stream, &mut out);
        } else {
            write_two_plane(stream, &mut out);
        }
    }
    out.extend_from_slice(&plan.ops);
    out.extend_from_slice(&plan.bits);
    out
}

/// Decode `vertex_count` interleaved `(x, y)` vertices over `runs`.
pub fn decode(
    data: &[u8],
    runs: &[Run],
    vertex_count: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
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
    let (mut walk, rest) = Walk::new(rest, &lengths, guesses)?;
    let (ops, bits) = match (sharing.copies, ops_len) {
        (false, _) => (&[][..], rest),
        (true, None) => (rest, &[][..]),
        (true, Some(len)) => rest
            .split_at_checked(len)
            .ok_or(Malformed("copy ops are cut short"))?,
    };
    if !sharing.line_references && !bits.is_empty() {
        return Err(Malformed("bytes after the last stream"));
    }
    let mut copier = Copier::new(ops, runs, n, dec)?;
    let mut references = if sharing.line_references {
        Some(LineReferences::new(bits, runs, dec)?)
    } else {
        None
    };

    let mut out = dec.alloc::<i32>(2 * n)?;
    out.resize(2 * n, 0);
    let vertices = out.as_chunks_mut::<2>().0;
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
            walk.residuals(&mut vertices[..end], start, [start, end], run.kind)?;
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

/// The decoder state between runs, with a reader per class stream.
struct Walk<'a> {
    start: Values<'a>,
    point: Values<'a>,
    ring: Values<'a>,
    corner: Values<'a>,
    line_second: Values<'a>,
    line: Values<'a>,
    start_guess: StartGuess,
    /// The shortest line part whose later vertices continue the last edge.
    continued_from: usize,
    /// The first vertex of the latest run.
    run_start: Pos,
}

impl<'a> Walk<'a> {
    /// Locates the class streams of `lengths` vertices at the front of `data`.
    fn new(
        data: &'a [u8],
        lengths: &[usize; CLASSES],
        guesses: Guesses,
    ) -> MltResult<(Self, &'a [u8])> {
        let mut streams = [Values::default(); CLASSES];
        let mut rest = data;
        for (class, (stream, &len)) in streams.iter_mut().zip(lengths).enumerate() {
            if len > 0 {
                (*stream, rest) = Values::locate(rest, 2 * len, class < SPLIT_VARINT_CLASSES)?;
            }
        }
        let [start, point, ring, corner, line_second, line] = streams;
        let walk = Self {
            start,
            point,
            ring,
            corner,
            line_second,
            line,
            start_guess: guesses.start,
            continued_from: guesses.line.min_len().map_or(usize::MAX, u32::into_usize),
            run_start: [0, 0],
        };
        Ok((walk, rest))
    }

    /// Decodes the points from `from` to the end of `vertices`, from copies or residuals.
    fn points(
        &mut self,
        vertices: &mut [Pos],
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
            let points = &mut vertices[i..stop];
            let pairs = self.point.pairs(points.len())?;
            for (v, &heads) in points.iter_mut().zip(pairs) {
                p = add(p, self.point.split_varint(heads)?);
                *v = p;
            }
            i = stop;
        }
        Ok(())
    }

    /// Decodes the run from `start` to the end of `vertices`, where copies or line `references` may share vertices.
    fn shared_run(
        &mut self,
        vertices: &mut [Pos],
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
            self.residuals(vertices, start, [i, stop], kind)?;
            i = stop;
        }
        self.run_start = vertices[start];
        Ok(())
    }

    /// Decodes the vertices `from..to` of the run from `start` to the end of `vertices` from their residuals.
    fn residuals(
        &mut self,
        vertices: &mut [Pos],
        start: usize,
        [from, to]: [usize; 2],
        kind: RunKind,
    ) -> MltResult<()> {
        let len = vertices.len() - start;
        let mut k = from - start;
        if k == 0 {
            let guess = match self.start_guess {
                StartGuess::Previous => previous(vertices, start),
                StartGuess::RunStart => self.run_start,
            };
            let heads = self.start.pair()?;
            self.run_start = add(guess, self.start.split_varint(heads)?);
            vertices[start] = self.run_start;
            k = 1;
        }
        let run = &mut vertices[start..to];
        if k >= run.len() {
            return Ok(());
        }
        match kind {
            RunKind::Ring if len == QUAD.into_usize() => {
                for k in k..run.len() {
                    if k == 3 {
                        let corner = [0, 1].map(|axis| {
                            run[0][axis]
                                .wrapping_add(run[2][axis])
                                .wrapping_sub(run[1][axis])
                        });
                        let low = self.corner.pair()?;
                        run[3] = add(corner, self.corner.two_plane(low)?);
                    } else {
                        let low = self.ring.pair()?;
                        run[k] = add(run[k - 1], self.ring.two_plane(low)?);
                    }
                }
            }
            RunKind::Ring => self.ring.deltas(&mut run[k - 1..])?,
            RunKind::Line => {
                if k == 1 {
                    let low = self.line_second.pair()?;
                    run[1] = add(run[0], self.line_second.two_plane(low)?);
                    k = 2;
                }
                if k < run.len() {
                    if len >= self.continued_from {
                        self.line.continued(&mut run[k - 2..])?;
                    } else {
                        self.line.deltas(&mut run[k - 1..])?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Checks that the vertices took every residual of every class stream.
    fn finish(&self) -> MltResult<()> {
        let streams = [
            &self.start,
            &self.point,
            &self.ring,
            &self.corner,
            &self.line_second,
            &self.line,
        ];
        if streams.iter().all(|s| s.heads.is_empty()) {
            Ok(())
        } else {
            Err(Malformed(
                "shared vertices disagree with the stream lengths",
            ))
        }
    }
}

/// The vertex before `i`, or the origin for the first.
fn previous(vertices: &[Pos], i: usize) -> Pos {
    i.checked_sub(1).map_or([0, 0], |p| vertices[p])
}

fn add(a: Pos, b: Pos) -> Pos {
    [a[0].wrapping_add(b[0]), a[1].wrapping_add(b[1])]
}

/// One class stream, read in order: a head byte per value, then the wider values.
#[derive(Clone, Copy, Default)]
struct Values<'a> {
    heads: &'a [u8],
    tail: &'a [u8],
}

impl<'a> Values<'a> {
    /// Splits a stream of `count` values off `data`, as split varints or as two planes.
    fn locate(data: &'a [u8], count: usize, split_varint: bool) -> MltResult<(Self, &'a [u8])> {
        let (cut_heads, cut_tail) = if split_varint {
            (
                "split varint heads are cut short",
                "split varint tail is cut short",
            )
        } else {
            (
                "two-plane low bytes are cut short",
                "two-plane varint is cut short",
            )
        };
        let Some((heads, rest)) = data.split_at_checked(count) else {
            return Err(Malformed(cut_heads));
        };
        #[expect(
            clippy::naive_bytecount,
            reason = "one count per stream needs no bytecount dependency"
        )]
        let wide = if split_varint {
            heads.iter().filter(|&&h| h & 0x80 != 0).count()
        } else {
            heads.iter().filter(|&&h| h == ESCAPE).count()
        };
        let Some(tail_len) = varints_len(rest, wide) else {
            return Err(Malformed(cut_tail));
        };
        let (tail, rest) = rest.split_at(tail_len);
        Ok((Self { heads, tail }, rest))
    }

    /// The head bytes of the next `count` vertices.
    fn pairs(&mut self, count: usize) -> MltResult<&'a [[u8; 2]]> {
        let Some((pairs, rest)) = self.heads.split_at_checked(2 * count) else {
            return Err(Malformed(
                "shared vertices disagree with the stream lengths",
            ));
        };
        self.heads = rest;
        Ok(pairs.as_chunks::<2>().0)
    }

    /// The head bytes of the next vertex.
    fn pair(&mut self) -> MltResult<[u8; 2]> {
        let Some((&pair, rest)) = self.heads.split_first_chunk() else {
            return Err(Malformed(
                "shared vertices disagree with the stream lengths",
            ));
        };
        self.heads = rest;
        Ok(pair)
    }

    /// The residual of a vertex with split varint `heads`.
    #[inline]
    fn split_varint(&mut self, heads: [u8; 2]) -> MltResult<Pos> {
        if (heads[0] | heads[1]) & 0x80 == 0 {
            return Ok(heads.map(|h| unzigzag(u32::from(h))));
        }
        let x = read_split_varint(heads[0], &mut self.tail)?;
        let y = read_split_varint(heads[1], &mut self.tail)?;
        Ok([unzigzag(x), unzigzag(y)])
    }

    /// The residual of a vertex with two-plane `low` bytes.
    #[inline]
    fn two_plane(&mut self, low: [u8; 2]) -> MltResult<Pos> {
        if low[0] != ESCAPE && low[1] != ESCAPE {
            return Ok(low.map(|b| unzigzag(u32::from(b))));
        }
        let x = read_two_plane(low[0], &mut self.tail)?;
        let y = read_two_plane(low[1], &mut self.tail)?;
        Ok([unzigzag(x), unzigzag(y)])
    }

    /// Decodes each vertex of `run` after the first as the one before it plus its residual.
    fn deltas(&mut self, run: &mut [Pos]) -> MltResult<()> {
        let Some((first, later)) = run.split_first_mut() else {
            return Ok(());
        };
        let mut p = *first;
        let pairs = self.pairs(later.len())?;
        for (v, &low) in later.iter_mut().zip(pairs) {
            p = add(p, self.two_plane(low)?);
            *v = p;
        }
        Ok(())
    }

    /// Decodes each vertex of `run` after the second as the last edge continued plus its residual.
    fn continued(&mut self, run: &mut [Pos]) -> MltResult<()> {
        let [mut a, mut b, ..] = *run else {
            return Ok(());
        };
        let later = &mut run[2..];
        let pairs = self.pairs(later.len())?;
        for (v, &low) in later.iter_mut().zip(pairs) {
            let guess = [0, 1].map(|axis| b[axis].wrapping_mul(2).wrapping_sub(a[axis]));
            (a, b) = (b, add(guess, self.two_plane(low)?));
            *v = b;
        }
        Ok(())
    }
}

/// The bytes the first `count` varints of `data` take, or `None` when it holds fewer.
fn varints_len(data: &[u8], count: usize) -> Option<usize> {
    if count == 0 {
        return Some(0);
    }
    let mut left = count;
    for (i, &byte) in data.iter().enumerate() {
        if byte & 0x80 == 0 {
            left -= 1;
            if left == 0 {
                return Some(i + 1);
            }
        }
    }
    None
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

fn guess(
    vertices: &[i32],
    i: usize,
    tag: Tag,
    start: StartGuess,
    previous: (i32, i32),
    run_start: (i32, i32),
) -> (i32, i32) {
    let at = |back: usize, axis: usize| vertices[2 * (i - back) + axis];
    match (tag, start) {
        (Tag::Start, StartGuess::RunStart) => run_start,
        (Tag::Corner, _) => {
            let corner = |axis| {
                at(3, axis)
                    .wrapping_add(at(1, axis))
                    .wrapping_sub(at(2, axis))
            };
            (corner(0), corner(1))
        }
        (Tag::ContinuedLine, _) => {
            let continued = |axis| at(1, axis).wrapping_mul(2).wrapping_sub(at(2, axis));
            (continued(0), continued(1))
        }
        _ => previous,
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

/// One head byte per value, then the continuation bytes of all values.
fn write_split_varint(values: &[u32], out: &mut Vec<u8>) {
    out.extend(values.iter().map(|&v| {
        let low = (v & 0x7F) as u8;
        if v > 0x7F { low | 0x80 } else { low }
    }));
    for &v in values.iter().filter(|&&v| v > 0x7F) {
        write_varint((v >> 7).into_usize(), out);
    }
}

/// The value of split varint `head`, taking its continuation bytes off `tail`.
fn read_split_varint(head: u8, tail: &mut &[u8]) -> MltResult<u32> {
    let mut value = u32::from(head & 0x7F);
    if head & 0x80 == 0 {
        return Ok(value);
    }
    let mut shift = 7;
    loop {
        let Some((&byte, rest)) = tail.split_first() else {
            return Err(Malformed("split varint tail is cut short"));
        };
        *tail = rest;
        let bits = u32::from(byte & 0x7F);
        if shift > 28 || (shift == 28 && bits > 0x0F) {
            return Err(Malformed("split varint exceeds 32 bits"));
        }
        value |= bits << shift;
        if byte & 0x80 == 0 {
            if byte == 0 {
                return Err(Malformed("split varint is not minimal"));
            }
            return Ok(value);
        }
        shift += 7;
    }
}

/// One byte per value with `0xFF` escaping a larger one, then the escaped values less `255` as varints.
fn write_two_plane(values: &[u32], out: &mut Vec<u8>) {
    #[expect(clippy::cast_possible_truncation, reason = "below the escape marker")]
    out.extend(values.iter().map(|&v| v.min(u32::from(ESCAPE)) as u8));
    for &v in values {
        if let Some(wide) = v.checked_sub(u32::from(ESCAPE)) {
            write_varint(wide.into_usize(), out);
        }
    }
}

/// The value of two-plane `low` byte, taking its escaped varint off `high`.
fn read_two_plane(low: u8, high: &mut &[u8]) -> MltResult<u32> {
    if low != ESCAPE {
        return Ok(u32::from(low));
    }
    let mut value = 0_u32;
    let mut shift = 0;
    loop {
        let Some((&byte, rest)) = high.split_first() else {
            return Err(Malformed("two-plane varint is cut short"));
        };
        *high = rest;
        let bits = u32::from(byte & 0x7F);
        if shift > 28 || (shift == 28 && bits > 0x0F) {
            return Err(Malformed("two-plane varint exceeds 32 bits"));
        }
        value |= bits << shift;
        if byte & 0x80 == 0 {
            if byte == 0 && shift > 0 {
                return Err(Malformed("two-plane varint is not minimal"));
            }
            let Some(value) = value.checked_add(u32::from(ESCAPE)) else {
                return Err(Malformed("two-plane value exceeds 32 bits"));
            };
            return Ok(value);
        }
        shift += 7;
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
        let header = Header {
            guesses: Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::ContinuedFromThree,
            },
            sharing: Sharing::NONE,
        };
        let payload = encode_as(&vertices, &[ring(0, 4), line(4, 3)], header).unwrap();
        assert_eq!(
            payload,
            [
                0x05, 0xC8, 0x90, 0x0A, 0x0A, 0x01, 0x03, 0x3C, 0x00, 0x00, 0x50, 0x00, 0x00, 0x14,
                0x00, 0x00, 0x04
            ]
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
        let payload = encode_as(&vertices, &runs, header).unwrap();
        assert_eq!(
            payload,
            [
                0x19, 0x02, 0x01, 0x00, 0x00, 0x03, 0x00, 0x00, 0xC8, 0x00, 0x01, 0xC8, 0x00, 0x00,
                0xC8, 0xC8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC8, 0x00, 0x00, 0xC8, 0x04, 0x05, 0x01,
                0xC0
            ]
        );
        assert_eq!(decode(&payload, &runs, 12, &mut dec()).unwrap(), vertices);
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
        let payload = encode_as(&vertices, &runs, header).unwrap();
        assert_eq!(super::header(&payload).unwrap(), header);
        assert_eq!(decode(&payload, &runs, 24, &mut dec()).unwrap(), vertices);
    }

    #[rstest]
    #[case::below_the_escape(254, &[0xFE])]
    #[case::at_the_escape(255, &[0xFF, 0x00])]
    #[case::above_the_escape(256, &[0xFF, 0x01])]
    #[case::two_varint_bytes(255 + 128, &[0xFF, 0x80, 0x01])]
    #[case::largest(u32::MAX, &[0xFF, 0x80, 0xFE, 0xFF, 0xFF, 0x0F])]
    fn two_planes_escape_from_255(#[case] value: u32, #[case] bytes: &[u8]) {
        let mut written = Vec::new();
        write_two_plane(&[value], &mut written);
        assert_eq!(written, bytes);
        let (&low, mut high) = bytes.split_first().unwrap();
        assert_eq!(read_two_plane(low, &mut high).unwrap(), value);
        assert_eq!(high, &[] as &[u8]);
    }

    #[test]
    fn two_planes_keep_escaped_values_in_order_behind_the_low_bytes() {
        let mut written = Vec::new();
        write_two_plane(&[3, 300, 254, 255, 70_000], &mut written);
        assert_eq!(
            written,
            [0x03, 0xFF, 0xFE, 0xFF, 0xFF, 0x2D, 0x00, 0xF1, 0xA0, 0x04]
        );
    }

    #[test]
    fn split_varints_move_continuation_bytes_behind_the_heads() {
        let mut written = Vec::new();
        write_split_varint(&[5, 300, 7], &mut written);
        assert_eq!(written, [0x05, 0xAC, 0x07, 0x02]);
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
        let payload = encode(&[1, 2, 3, 4], &[line(0, 2)]).unwrap();
        assert_eq!(payload, [0x00, 0x02, 0x04, 0x04, 0x04]);
    }

    #[test]
    fn the_race_keeps_the_run_start_guess_for_parallel_lines() {
        let vertices = [0, 0, 1000, 0, 1, 1, 1001, 1];
        let payload = encode(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            header(&payload).unwrap(),
            Header {
                guesses: Guesses {
                    start: StartGuess::RunStart,
                    line: LineGuess::Previous,
                },
                sharing: Sharing::NONE,
            }
        );
    }

    #[test]
    fn the_race_continues_a_straight_line_of_three() {
        let vertices = [0, 0, 500, 500, 1000, 1000];
        let payload = encode(&vertices, &[line(0, 3)]).unwrap();
        assert_eq!(
            header(&payload).unwrap(),
            Header {
                guesses: Guesses {
                    start: StartGuess::Previous,
                    line: LineGuess::ContinuedFromThree,
                },
                sharing: Sharing::NONE,
            }
        );
    }

    #[test]
    fn the_race_copies_a_shared_wall() {
        let vertices = [
            0, 0, 1000, 0, 1000, 1000, 0, 1000, 1000, 1000, 1000, 0, 2000, 0, 2000, 1000,
        ];
        let payload = encode(&vertices, &[ring(0, 4), ring(4, 4)]).unwrap();
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
        let vertices = [0, 0, 1000, 1000, 5000, 0, 1000, 1000];
        let payload = encode(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            header(&payload).unwrap().sharing,
            Sharing {
                copies: false,
                line_references: true,
            }
        );
    }

    #[rstest]
    #[case::no_header(&[], "no header byte")]
    #[case::line_guess_three(&[0x06, 0x00, 0x00], "unknown line guess")]
    #[case::run_start_line_guess_three(&[0x07, 0x00, 0x00], "unknown line guess")]
    #[case::reserved_bit_5(&[0x20, 0x00, 0x00], "reserved header bit set")]
    #[case::reserved_bit_6(&[0x40, 0x00, 0x00], "reserved header bit set")]
    #[case::reserved_bit_7(&[0x80, 0x00, 0x00], "reserved header bit set")]
    #[case::trailing_bytes(&[0x00, 0x00, 0x00, 0x00], "bytes after the last stream")]
    #[case::cut_heads(&[0x00, 0x00], "split varint heads are cut short")]
    #[case::cut_tail(&[0x00, 0x80, 0x00], "split varint tail is cut short")]
    #[case::not_minimal(&[0x00, 0x80, 0x00, 0x00], "split varint is not minimal")]
    #[case::too_wide(&[0x00, 0x80, 0x00, 0xFF, 0xFF, 0xFF, 0x7F], "split varint exceeds 32 bits")]
    #[case::more_shared_than_vertices(&[0x08, 0x02], "more shared vertices than vertices in a stream")]
    #[case::shared_without_a_copy(&[0x08, 0x01], "shared vertices disagree with the stream lengths")]
    #[case::ops_cut_short(&[0x18, 0x00, 0x01, 0x00, 0x00], "copy ops are cut short")]
    #[case::bits_after_the_reference_flags(&[0x10, 0x00, 0x00, 0x00, 0x00], "bytes after the last reference bit")]
    fn malformed_point_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode(data, &[], 1, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[rstest]
    #[case::cut_low(&[0x00, 0x00, 0x00, 0x00], "two-plane low bytes are cut short")]
    #[case::cut_high(&[0x00, 0x00, 0x00, 0xFF, 0x00], "two-plane varint is cut short")]
    #[case::not_minimal(&[0x00, 0x00, 0x00, 0xFF, 0x00, 0x80, 0x00], "two-plane varint is not minimal")]
    #[case::too_wide(&[0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F], "two-plane varint exceeds 32 bits")]
    #[case::overflow(&[0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F], "two-plane value exceeds 32 bits")]
    #[case::copy_without_a_count(&[0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00], "shared vertices disagree with the stream lengths")]
    #[case::count_without_a_copy(&[0x08, 0x00, 0x01, 0x00, 0x00], "shared vertices disagree with the stream lengths")]
    fn malformed_ring_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode(data, &[ring(0, 2)], 2, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[rstest]
    #[case::flag_cut_short(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], "reference bits are cut short")]
    #[case::index_past_the_slots(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0], "reference index past the slots")]
    #[case::padding_set(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88], "bytes after the last reference bit")]
    #[case::byte_after_the_bits(&[0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00], "bytes after the last reference bit")]
    fn malformed_line_references_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let runs = [line(0, 3), line(3, 2)];
        let err = decode(data, &runs, 5, &mut dec()).unwrap_err();
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
        let payload = encode_as(&vertices, &runs, header).unwrap();
        let err = decode(&payload[..payload.len() - 1], &runs, 6, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "predicted vertex stream is malformed: two-plane low bytes are cut short"
        );
    }

    #[rstest]
    #[case::overlapping(&[ring(0, 4), line(2, 6)], "runs overlap or are out of order")]
    #[case::past_the_end(&[line(0, 9)], "a run ends past the last vertex")]
    fn bad_runs_are_rejected(#[case] runs: &[Run], #[case] reason: &str) {
        let err = decode(&[0x00], runs, 8, &mut dec()).unwrap_err();
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
        let payload = encode_as(&vertices, &runs, header).unwrap();
        assert_eq!(decode(&payload, &runs, 4, &mut dec()).unwrap(), vertices);
    }

    fn arb_layer() -> impl Strategy<Value = (Vec<i32>, Vec<Run>)> {
        let shapes = prop::collection::vec((0_u32..12, any::<bool>(), 0_u32..3), 0..12);
        (shapes, any::<bool>()).prop_flat_map(|(shapes, few_positions)| {
            let mut runs = Vec::new();
            let mut start = 0;
            for (len, ring, gap) in shapes {
                start += gap;
                let kind = if ring { RunKind::Ring } else { RunKind::Line };
                runs.push(Run { start, len, kind });
                start += len;
            }
            let words = 2 * start.into_usize();
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
        fn any_layer_round_trips_every_header((vertices, runs) in arb_layer()) {
            let count = u32::try_from(vertices.len() / 2).unwrap();
            for guesses in Guesses::ALL {
                for sharing in SHARINGS {
                    let payload = encode_as(&vertices, &runs, Header { guesses, sharing }).unwrap();
                    prop_assert_eq!(decode(&payload, &runs, count, &mut dec()).unwrap(), vertices.clone());
                }
            }
        }

        #[test]
        fn any_layer_round_trips_the_race((vertices, runs) in arb_layer()) {
            let payload = encode(&vertices, &runs).unwrap();
            let count = u32::try_from(vertices.len() / 2).unwrap();
            prop_assert_eq!(decode(&payload, &runs, count, &mut dec()).unwrap(), vertices);
        }

        #[test]
        fn the_race_is_never_longer_than_any_unshared_guess_pair((vertices, runs) in arb_layer()) {
            let raced = encode(&vertices, &runs).unwrap();
            for guesses in Guesses::ALL {
                let unshared = Header { guesses, sharing: Sharing::NONE };
                prop_assert!(raced.len() <= encode_as(&vertices, &runs, unshared).unwrap().len());
            }
        }
    }
}
