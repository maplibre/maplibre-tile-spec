//! Predicted vertices: residuals against a guess from the decoded vertices, in six class streams.

use usize_cast::IntoUsize as _;

use crate::MltError::MalformedPredictedVertices as Malformed;
use crate::{Decoder, MltResult};

/// A vertex as `[x, y]`.
type Pos = [i32; 2];

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

/// The guesses a payload was written with, stored in its header byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guesses {
    pub start: StartGuess,
    pub line: LineGuess,
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
const RESERVED: u8 = 0xF8;

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

    fn header(self) -> u8 {
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

    fn from_header(byte: u8) -> MltResult<Self> {
        if byte & RESERVED != 0 {
            return Err(Malformed("reserved header bit set"));
        }
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

/// The guesses a payload was written with.
pub fn guesses(data: &[u8]) -> MltResult<Guesses> {
    let (&header, _) = data.split_first().ok_or(Malformed("no header byte"))?;
    Guesses::from_header(header)
}

/// The shortest payload for interleaved `(x, y)` `vertices` over `runs`, the lowest header byte on a tie.
pub fn encode(vertices: &[i32], runs: &[Run]) -> MltResult<Vec<u8>> {
    let mut best: Option<Vec<u8>> = None;
    for guesses in Guesses::ALL.into_iter().filter(|g| g.useful(runs)) {
        let payload = encode_as(vertices, runs, guesses)?;
        if best.as_ref().is_none_or(|b| payload.len() < b.len()) {
            best = Some(payload);
        }
    }
    Ok(best.expect("the previous-vertex guesses are always useful"))
}

/// The payload for interleaved `(x, y)` `vertices` over `runs` with `guesses`.
fn encode_as(vertices: &[i32], runs: &[Run], guesses: Guesses) -> MltResult<Vec<u8>> {
    if !vertices.len().is_multiple_of(2) {
        return Err(Malformed("odd vertex word count"));
    }
    let tags = classify(runs, vertices.len() / 2, guesses.line)?;
    let mut streams: [Vec<u32>; CLASSES] = Default::default();
    let mut previous = (0_i32, 0_i32);
    let mut run_start = (0_i32, 0_i32);
    for (i, (&tag, v)) in tags.iter().zip(vertices.as_chunks::<2>().0).enumerate() {
        let (gx, gy) = guess(vertices, i, tag, guesses.start, previous, run_start);
        let stream = &mut streams[tag.stream()];
        stream.push(zigzag(v[0].wrapping_sub(gx)));
        stream.push(zigzag(v[1].wrapping_sub(gy)));
        if tag == Tag::Start {
            run_start = (v[0], v[1]);
        }
        previous = (v[0], v[1]);
    }
    let mut out = Vec::with_capacity(vertices.len() + 1);
    out.push(guesses.header());
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
    Ok(out)
}

/// Decode `vertex_count` interleaved `(x, y)` vertices over `runs`.
pub fn decode(
    data: &[u8],
    runs: &[Run],
    vertex_count: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let (&header, rest) = data.split_first().ok_or(Malformed("no header byte"))?;
    let guesses = Guesses::from_header(header)?;
    let n = vertex_count.into_usize();
    let lengths = class_lengths(runs, n)?;
    let (mut walk, rest) = Walk::new(rest, &lengths, guesses)?;
    if !rest.is_empty() {
        return Err(Malformed("bytes after the last stream"));
    }
    let mut out = dec.alloc::<i32>(2 * n)?;
    out.resize(2 * n, 0);
    let vertices = out.as_chunks_mut::<2>().0;
    let mut end = 0;
    for run in runs.iter().filter(|r| r.len > 0) {
        let start = run.start.into_usize();
        if start > end {
            walk.points(&mut vertices[..start], end)?;
        }
        end = start + run.len.into_usize();
        walk.run(&mut vertices[..end], start, run.kind)?;
    }
    walk.points(vertices, end)?;
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

    /// Decodes the points from `from` to the end of `vertices`.
    fn points(&mut self, vertices: &mut [Pos], from: usize) -> MltResult<()> {
        let mut p = previous(vertices, from);
        let points = &mut vertices[from..];
        let pairs = self.point.pairs(points.len())?;
        for (v, &heads) in points.iter_mut().zip(pairs) {
            p = add(p, self.point.split_varint(heads)?);
            *v = p;
        }
        Ok(())
    }

    /// Decodes the run from `start` to the end of `vertices`.
    fn run(&mut self, vertices: &mut [Pos], start: usize, kind: RunKind) -> MltResult<()> {
        let guess = match self.start_guess {
            StartGuess::Previous => previous(vertices, start),
            StartGuess::RunStart => self.run_start,
        };
        let heads = self.start.pair()?;
        let first = add(guess, self.start.split_varint(heads)?);
        self.run_start = first;
        let run = &mut vertices[start..];
        run[0] = first;
        match kind {
            RunKind::Ring if run.len() == QUAD.into_usize() => {
                let low = self.ring.pair()?;
                run[1] = add(run[0], self.ring.two_plane(low)?);
                let low = self.ring.pair()?;
                run[2] = add(run[1], self.ring.two_plane(low)?);
                let corner = [0, 1].map(|axis| {
                    run[0][axis]
                        .wrapping_add(run[2][axis])
                        .wrapping_sub(run[1][axis])
                });
                let low = self.corner.pair()?;
                run[3] = add(corner, self.corner.two_plane(low)?);
            }
            RunKind::Ring => self.ring.deltas(run)?,
            RunKind::Line => {
                if let [first, second, ..] = run {
                    let low = self.line_second.pair()?;
                    *second = add(*first, self.line_second.two_plane(low)?);
                }
                if run.len() >= self.continued_from {
                    self.line.continued(run)?;
                } else if let Some(later) = run.get_mut(1..) {
                    self.line.deltas(later)?;
                }
            }
        }
        Ok(())
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
            return Err(Malformed("a class stream ends early"));
        };
        self.heads = rest;
        Ok(pairs.as_chunks::<2>().0)
    }

    /// The head bytes of the next vertex.
    fn pair(&mut self) -> MltResult<[u8; 2]> {
        let Some((&pair, rest)) = self.heads.split_first_chunk() else {
            return Err(Malformed("a class stream ends early"));
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

fn write_varint(mut v: u32, out: &mut Vec<u8>) {
    loop {
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
        write_varint(v >> 7, out);
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
            write_varint(wide, out);
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

    #[test]
    fn a_ring_and_a_line_encode_to_the_exact_payload() {
        let vertices = [
            100, 200, 130, 200, 130, 240, 100, 240, 105, 205, 115, 205, 125, 207,
        ];
        let guesses = Guesses {
            start: StartGuess::RunStart,
            line: LineGuess::ContinuedFromThree,
        };
        let payload = encode_as(&vertices, &[ring(0, 4), line(4, 3)], guesses).unwrap();
        assert_eq!(
            payload,
            [
                0x05, 0xC8, 0x90, 0x0A, 0x0A, 0x01, 0x03, 0x3C, 0x00, 0x00, 0x50, 0x00, 0x00, 0x14,
                0x00, 0x00, 0x04
            ]
        );
    }

    #[rstest]
    fn every_guess_pair_round_trips_mixed_runs(
        #[values(StartGuess::Previous, StartGuess::RunStart)] start: StartGuess,
        #[values(
            LineGuess::Previous,
            LineGuess::ContinuedFromSix,
            LineGuess::ContinuedFromThree
        )]
        line_guess: LineGuess,
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
            i32::MIN,
            i32::MAX,
            i32::MAX,
            i32::MIN,
            0,
            0,
            -1,
            1,
            300,
            -70_000,
            1 << 30,
            -(1 << 30),
            7,
            7,
            9,
            9,
            11,
            12,
            900,
            900,
            300,
            300,
            301,
            299,
        ];
        let runs = [
            ring(1, 4),
            line(5, 7),
            ring(12, 4),
            line(16, 2),
            line(19, 3),
            ring(22, 2),
        ];
        let guesses = Guesses {
            start,
            line: line_guess,
        };
        let payload = encode_as(&vertices, &runs, guesses).unwrap();
        assert_eq!(super::guesses(&payload).unwrap(), guesses);
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
    fn the_race_keeps_the_lowest_header_byte_on_a_tie() {
        let payload = encode(&[1, 2, 3, 4], &[line(0, 2)]).unwrap();
        assert_eq!(payload, [0x00, 0x02, 0x04, 0x04, 0x04]);
    }

    #[test]
    fn the_race_keeps_the_run_start_guess_for_parallel_lines() {
        let vertices = [0, 0, 1000, 0, 1, 1, 1001, 1];
        let payload = encode(&vertices, &[line(0, 2), line(2, 2)]).unwrap();
        assert_eq!(
            guesses(&payload).unwrap(),
            Guesses {
                start: StartGuess::RunStart,
                line: LineGuess::Previous,
            }
        );
    }

    #[test]
    fn the_race_continues_a_straight_line_of_three() {
        let vertices = [0, 0, 500, 500, 1000, 1000];
        let payload = encode(&vertices, &[line(0, 3)]).unwrap();
        assert_eq!(
            guesses(&payload).unwrap(),
            Guesses {
                start: StartGuess::Previous,
                line: LineGuess::ContinuedFromThree,
            }
        );
    }

    #[rstest]
    #[case::no_header(&[], "no header byte")]
    #[case::line_guess_three(&[0x06, 0x00, 0x00], "unknown line guess")]
    #[case::run_start_line_guess_three(&[0x07, 0x00, 0x00], "unknown line guess")]
    #[case::reserved_bit_3(&[0x08, 0x00, 0x00], "reserved header bit set")]
    #[case::reserved_bit_4(&[0x10, 0x00, 0x00], "reserved header bit set")]
    #[case::reserved_bit_7(&[0x80, 0x00, 0x00], "reserved header bit set")]
    #[case::trailing_bytes(&[0x00, 0x00, 0x00, 0x00], "bytes after the last stream")]
    #[case::cut_heads(&[0x00, 0x00], "split varint heads are cut short")]
    #[case::cut_tail(&[0x00, 0x80, 0x00], "split varint tail is cut short")]
    #[case::not_minimal(&[0x00, 0x80, 0x00, 0x00], "split varint is not minimal")]
    #[case::too_wide(&[0x00, 0x80, 0x00, 0xFF, 0xFF, 0xFF, 0x7F], "split varint exceeds 32 bits")]
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
    fn malformed_ring_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode(data, &[ring(0, 2)], 2, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn a_truncated_payload_is_rejected() {
        let vertices = [0, 0, 300, 0, 300, 300, 0, 300, 10, 10, 20, 20];
        let runs = [ring(0, 4), line(4, 2)];
        let guesses = Guesses {
            start: StartGuess::Previous,
            line: LineGuess::Previous,
        };
        let payload = encode_as(&vertices, &runs, guesses).unwrap();
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
        let guesses = Guesses {
            start: StartGuess::RunStart,
            line: LineGuess::ContinuedFromThree,
        };
        let payload = encode_as(&vertices, &runs, guesses).unwrap();
        assert_eq!(decode(&payload, &runs, 4, &mut dec()).unwrap(), vertices);
    }

    fn arb_layer() -> impl Strategy<Value = (Vec<i32>, Vec<Run>)> {
        prop::collection::vec((0_u32..12, any::<bool>(), 0_u32..3), 0..12).prop_flat_map(|shapes| {
            let mut runs = Vec::new();
            let mut start = 0;
            for (len, ring, gap) in shapes {
                start += gap;
                let kind = if ring { RunKind::Ring } else { RunKind::Line };
                runs.push(Run { start, len, kind });
                start += len;
            }
            let words = 2 * start.into_usize();
            (prop::collection::vec(any::<i32>(), words), Just(runs))
        })
    }

    proptest! {
        #[test]
        fn any_layer_round_trips_every_guess_pair((vertices, runs) in arb_layer()) {
            let count = u32::try_from(vertices.len() / 2).unwrap();
            for guesses in Guesses::ALL {
                let payload = encode_as(&vertices, &runs, guesses).unwrap();
                prop_assert_eq!(decode(&payload, &runs, count, &mut dec()).unwrap(), vertices.clone());
            }
        }

        #[test]
        fn any_layer_round_trips_the_race((vertices, runs) in arb_layer()) {
            let payload = encode(&vertices, &runs).unwrap();
            let count = u32::try_from(vertices.len() / 2).unwrap();
            prop_assert_eq!(decode(&payload, &runs, count, &mut dec()).unwrap(), vertices);
        }

        #[test]
        fn the_race_is_never_longer_than_any_guess_pair((vertices, runs) in arb_layer()) {
            let raced = encode(&vertices, &runs).unwrap();
            for guesses in Guesses::ALL {
                prop_assert!(raced.len() <= encode_as(&vertices, &runs, guesses).unwrap().len());
            }
        }
    }
}
