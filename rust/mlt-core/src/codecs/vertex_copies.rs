//! Vertex copies: runs of vertices written as a walk over earlier decoded vertices, forwards or backwards.

use rustc_hash::FxHashMap;
use usize_cast::IntoUsize as _;

use crate::MltError::MalformedPredictedVertices as Malformed;
use crate::codecs::predicted::{Pos, Run, RunKind, write_varint};
use crate::codecs::varint::parse_varint;
use crate::{Decoder, MltResult};

/// One copy: `len` vertices from `dst` on, read from `dst - dist` on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyOp {
    pub dst: usize,
    pub len: usize,
    pub dist: usize,
    /// Whether the source steps backwards.
    pub backward: bool,
}

/// The fewest vertices the encoder copies at once.
const MIN_LEN: usize = 2;

/// How many earlier vertices at the same position the encoder tries per destination.
const CANDIDATES: usize = 32;

/// The start and end of the non-empty ring that holds each vertex, or `None` outside rings.
fn ring_bounds(runs: &[Run], vertex_count: usize) -> Vec<Option<[usize; 2]>> {
    let mut bounds = vec![None; vertex_count];
    for run in runs.iter().filter(|r| r.kind == RunKind::Ring && r.len > 0) {
        let start = run.start.into_usize();
        let end = start + run.len.into_usize();
        bounds[start..end].fill(Some([start, end]));
    }
    bounds
}

/// The source vertex after `s`, stepping within `ring` with wrap-around, or flat without.
fn step(s: usize, backward: bool, ring: Option<[usize; 2]>) -> Option<usize> {
    match (ring, backward) {
        (Some([start, end]), false) => Some(if s + 1 == end { start } else { s + 1 }),
        (Some([start, end]), true) => Some(if s == start { end - 1 } else { s - 1 }),
        (None, false) => Some(s + 1),
        (None, true) => s.checked_sub(1),
    }
}

/// The bytes `value` takes as a varint.
#[must_use]
pub fn varint_len(value: usize) -> usize {
    1 + ((value | 1).bit_width() - 1).into_usize() / 7
}

fn op_bytes(gap: usize, op: &CopyOp) -> usize {
    varint_len(gap) + varint_len(2 * op.dist + usize::from(op.backward)) + varint_len(op.len - 1)
}

/// Greedily picks copies of two or more vertices that save more residual bytes than their ops take.
///
/// `saved` holds the residual bytes of each vertex.
/// Each destination tries the latest few earlier vertices at its position, so planning is linear in the vertices.
#[must_use]
pub fn plan(vertices: &[i32], runs: &[Run], saved: &[usize]) -> Vec<CopyOp> {
    let n = saved.len();
    let pos = |i: usize| -> Pos { [vertices[2 * i], vertices[2 * i + 1]] };
    let rings = ring_bounds(runs, n);
    let mut prefix = Vec::with_capacity(n + 1);
    prefix.push(0_usize);
    for &s in saved {
        prefix.push(prefix[prefix.len() - 1] + s);
    }
    let mut previous = vec![usize::MAX; n];
    let mut latest: FxHashMap<Pos, usize> = FxHashMap::default();
    for (i, prev) in previous.iter_mut().enumerate() {
        if let Some(j) = latest.insert(pos(i), i) {
            *prev = j;
        }
    }
    let extend = |i: usize, j: usize, backward: bool| {
        let ring = rings[j];
        let mut s = j;
        let mut k = 0;
        while i + k < n && s < i + k && pos(s) == pos(i + k) {
            k += 1;
            match step(s, backward, ring) {
                Some(next) => s = next,
                None => break,
            }
        }
        k
    };
    let mut ops = Vec::new();
    let mut end = 0;
    let mut i = 0;
    while i < n {
        let mut best: Option<(usize, CopyOp)> = None;
        let mut j = previous[i];
        for _ in 0..CANDIDATES {
            if j == usize::MAX {
                break;
            }
            for backward in [false, true] {
                let len = extend(i, j, backward);
                if len < MIN_LEN {
                    continue;
                }
                let op = CopyOp {
                    dst: i,
                    len,
                    dist: i - j,
                    backward,
                };
                let gain = (prefix[i + len] - prefix[i]).saturating_sub(op_bytes(i - end, &op));
                if gain > 0 && best.is_none_or(|(g, _)| gain > g) {
                    best = Some((gain, op));
                }
            }
            j = previous[j];
        }
        if let Some((_, op)) = best {
            ops.push(op);
            i += op.len;
            end = i;
        } else {
            i += 1;
        }
    }
    ops
}

/// Which of `vertex_count` vertices the ops write.
#[must_use]
pub fn mask(ops: &[CopyOp], vertex_count: usize) -> Vec<bool> {
    let mut written = vec![false; vertex_count];
    for op in ops {
        written[op.dst..op.dst + op.len].fill(true);
    }
    written
}

/// Appends `ops`, each as the varints `gap`, `dist << 1 | backward` and `len - 1`.
pub fn write_ops(ops: &[CopyOp], out: &mut Vec<u8>) {
    let mut end = 0;
    for op in ops {
        write_varint(op.dst - end, out);
        write_varint(2 * op.dist + usize::from(op.backward), out);
        write_varint(op.len - 1, out);
        end = op.dst + op.len;
    }
}

fn parse_op(ops: &[u8], end: usize, vertex_count: usize) -> MltResult<(&[u8], CopyOp)> {
    let (ops, gap) = parse_varint::<u32>(ops)?;
    let (ops, source) = parse_varint::<u32>(ops)?;
    let (ops, len) = parse_varint::<u32>(ops)?;
    let Some(dst) = end.checked_add(gap.into_usize()) else {
        return Err(Malformed("copy past the last vertex"));
    };
    let len = len.into_usize() + 1;
    if dst.checked_add(len).is_none_or(|e| e > vertex_count) {
        return Err(Malformed("copy past the last vertex"));
    }
    let distance = (source >> 1).into_usize();
    let backward = source & 1 == 1;
    if distance == 0 || distance > dst {
        return Err(Malformed("copy source out of range"));
    }
    if len == 1 && backward {
        return Err(Malformed("a one-vertex copy has a direction"));
    }
    Ok((
        ops,
        CopyOp {
            dst,
            len,
            dist: distance,
            backward,
        },
    ))
}

/// Replays copy ops while the vertices are decoded in order.
pub struct Copier<'a> {
    ops: &'a [u8],
    vertex_count: usize,
    /// The start and end of every non-empty ring, in order.
    rings: Vec<[usize; 2]>,
    /// The first vertex of the pending op, or `usize::MAX` when none is left.
    next: usize,
    pending: Option<CopyOp>,
    remaining: usize,
    src: usize,
    backward: bool,
    ring: Option<[usize; 2]>,
}

impl<'a> Copier<'a> {
    /// A copier over `ops` for `vertex_count` vertices in `runs`.
    pub fn new(
        ops: &'a [u8],
        runs: &[Run],
        vertex_count: usize,
        dec: &mut Decoder,
    ) -> MltResult<Self> {
        let non_empty_rings = || runs.iter().filter(|r| r.kind == RunKind::Ring && r.len > 0);
        let rings = if ops.is_empty() {
            Vec::new()
        } else {
            let mut rings = dec.alloc::<[usize; 2]>(non_empty_rings().count())?;
            rings.extend(non_empty_rings().map(|r| {
                let start = r.start.into_usize();
                [start, start + r.len.into_usize()]
            }));
            rings
        };
        let mut copier = Self {
            ops,
            vertex_count,
            rings,
            next: usize::MAX,
            pending: None,
            remaining: 0,
            src: 0,
            backward: false,
            ring: None,
        };
        copier.read_next(0)?;
        Ok(copier)
    }

    fn read_next(&mut self, end: usize) -> MltResult<()> {
        if self.ops.is_empty() {
            self.next = usize::MAX;
            self.pending = None;
            return Ok(());
        }
        let (rest, op) = parse_op(self.ops, end, self.vertex_count)?;
        self.ops = rest;
        self.next = op.dst;
        self.pending = Some(op);
        Ok(())
    }

    /// The first vertex of the next op, or `usize::MAX` when none is left.
    #[must_use]
    pub fn next(&self) -> usize {
        self.next
    }

    /// Whether an op writes a vertex before `end` from the next vertex on.
    #[must_use]
    pub fn copies_before(&self, end: usize) -> bool {
        self.remaining > 0 || self.next < end
    }

    /// Writes the vertices of `vertices` from `i` on that one op covers and returns how many.
    #[inline]
    pub fn copies(&mut self, i: usize, vertices: &mut [Pos]) -> MltResult<usize> {
        if self.remaining == 0 {
            if i != self.next {
                return Ok(0);
            }
            self.start(i)?;
        }
        let count = self.remaining.min(vertices.len() - i);
        for at in i..i + count {
            let s = self.src;
            if s >= at {
                return Err(Malformed("copy reads a vertex not yet decoded"));
            }
            vertices[at] = vertices[s];
            self.remaining -= 1;
            if self.remaining > 0 {
                let Some(next) = step(s, self.backward, self.ring) else {
                    return Err(Malformed("copy source out of range"));
                };
                self.src = next;
            }
        }
        Ok(count)
    }

    fn start(&mut self, i: usize) -> MltResult<()> {
        let Some(op) = self.pending else {
            return Err(Malformed("copy past the last vertex"));
        };
        let src = i - op.dist;
        let at = self.rings.partition_point(|r| r[0] <= src);
        self.ring = at
            .checked_sub(1)
            .map(|at| self.rings[at])
            .filter(|r| src < r[1]);
        self.src = src;
        self.backward = op.backward;
        self.remaining = op.len;
        self.read_next(op.dst + op.len)
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

    fn replay(ops: &[u8], runs: &[Run], vertices: &[i32]) -> MltResult<Vec<i32>> {
        let n = vertices.len() / 2;
        let mut copier = Copier::new(ops, runs, n, &mut dec())?;
        let mut out = vec![[0, 0]; n];
        let mut i = 0;
        while i < n {
            match copier.copies(i, &mut out)? {
                0 => {
                    out[i] = vertices.as_chunks::<2>().0[i];
                    i += 1;
                }
                shared => i += shared,
            }
        }
        Ok(out.into_flattened())
    }

    fn ops_of(copies: &[CopyOp]) -> Vec<u8> {
        let mut ops = Vec::new();
        write_ops(copies, &mut ops);
        ops
    }

    const HOUSES: [i32; 16] = [0, 0, 10, 0, 10, 10, 0, 10, 10, 10, 10, 0, 20, 0, 20, 10];

    #[test]
    fn a_shared_wall_walked_backwards_is_one_backward_copy() {
        let runs = [ring(0, 4), ring(4, 4)];
        let copies = plan(&HOUSES, &runs, &[3; 8]);
        assert_eq!(
            copies,
            [CopyOp {
                dst: 4,
                len: 2,
                dist: 2,
                backward: true
            }]
        );
        let ops = ops_of(&copies);
        assert_eq!(ops, [0x04, 0x05, 0x01]);
        assert_eq!(replay(&ops, &runs, &HOUSES).unwrap(), HOUSES);
    }

    #[test]
    fn a_repeated_line_is_one_forward_copy() {
        let vertices = [0, 0, 7, 3, 9, 8, 50, 50, 0, 0, 7, 3, 9, 8];
        let runs = [line(0, 3), line(4, 3)];
        let copies = plan(&vertices, &runs, &[3; 7]);
        assert_eq!(
            copies,
            [CopyOp {
                dst: 4,
                len: 3,
                dist: 4,
                backward: false
            }]
        );
        let ops = ops_of(&copies);
        assert_eq!(ops, [0x04, 0x08, 0x02]);
        assert_eq!(replay(&ops, &runs, &vertices).unwrap(), vertices);
    }

    #[test]
    fn a_backward_copy_wraps_over_the_start_of_its_source_ring() {
        let vertices = [0, 0, 10, 0, 10, 10, 0, 10, 0, 0, 0, 10, 5, 20];
        let runs = [ring(0, 4), ring(4, 3)];
        let copies = plan(&vertices, &runs, &[3; 7]);
        assert_eq!(
            copies,
            [CopyOp {
                dst: 4,
                len: 2,
                dist: 4,
                backward: true
            }]
        );
        let ops = ops_of(&copies);
        assert_eq!(ops, [0x04, 0x09, 0x01]);
        assert_eq!(replay(&ops, &runs, &vertices).unwrap(), vertices);
    }

    #[test]
    fn a_forward_copy_wraps_over_the_end_of_its_source_ring() {
        let vertices = [0, 0, 10, 0, 10, 10, 0, 10, 0, 10, 0, 0, 10, 0, 5, -20];
        let runs = [ring(0, 4), ring(4, 4)];
        let copies = plan(&vertices, &runs, &[3; 8]);
        assert_eq!(
            copies,
            [CopyOp {
                dst: 4,
                len: 3,
                dist: 1,
                backward: false
            }]
        );
        let ops = ops_of(&copies);
        assert_eq!(ops, [0x04, 0x02, 0x02]);
        assert_eq!(replay(&ops, &runs, &vertices).unwrap(), vertices);
    }

    #[test]
    fn a_copy_that_saves_less_than_its_op_is_dropped() {
        let runs = [ring(0, 4), ring(4, 4)];
        assert_eq!(plan(&HOUSES, &runs, &[1; 8]), []);
    }

    #[test]
    fn a_single_shared_vertex_is_not_copied() {
        let vertices = [0, 0, 50, 50, 99, 0, 50, 50, 0, 99];
        assert_eq!(plan(&vertices, &[], &[4; 5]), []);
    }

    #[rstest]
    #[case::one_vertex(&[0x03, 0x02, 0x00], &[0, 0, 10, 0, 10, 10, 10, 10])]
    #[case::two_vertices(&[0x02, 0x02, 0x01], &[0, 0, 10, 0, 10, 0, 10, 0])]
    fn n_less_one_is_written_for_the_vertex_count(#[case] ops: &[u8], #[case] decoded: &[i32]) {
        let vertices = [0, 0, 10, 0, 10, 10, 99, 99];
        assert_eq!(replay(ops, &[], &vertices).unwrap(), decoded);
    }

    #[test]
    fn a_varint_takes_one_byte_per_seven_bits() {
        assert_eq!(
            [0, 1, 127, 128, 16_383, 16_384].map(varint_len),
            [1, 1, 1, 2, 2, 3]
        );
    }

    #[rstest]
    #[case::past_the_last_vertex(&[0x08, 0x02, 0x00], "copy past the last vertex")]
    #[case::too_long(&[0x04, 0x02, 0x04], "copy past the last vertex")]
    #[case::zero_distance(&[0x04, 0x00, 0x01], "copy source out of range")]
    #[case::before_the_first_vertex(&[0x04, 0x0A, 0x01], "copy source out of range")]
    #[case::directed_single(&[0x04, 0x03, 0x00], "a one-vertex copy has a direction")]
    #[case::flat_source_runs_off_the_front(&[0x01, 0x03, 0x02], "copy source out of range")]
    #[case::wrap_reads_ahead(&[0x05, 0x03, 0x02], "copy reads a vertex not yet decoded")]
    #[case::second_op_past_the_end(&[0x04, 0x04, 0x01, 0x00, 0x02, 0x02], "copy past the last vertex")]
    fn malformed_ops_are_rejected(#[case] ops: &[u8], #[case] reason: &str) {
        let err = replay(ops, &[ring(4, 4)], &HOUSES).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn a_truncated_op_is_rejected() {
        let err = replay(&[0x04, 0x05], &[ring(0, 4), ring(4, 4)], &HOUSES).unwrap_err();
        assert_eq!(
            err.to_string(),
            "buffer underflow: needed 1 bytes, but only 0 remain"
        );
    }

    #[test]
    fn a_zero_padded_op_is_rejected() {
        let err = replay(&[0x84, 0x00, 0x05, 0x01], &[], &HOUSES).unwrap_err();
        assert_eq!(
            err.to_string(),
            "varint uses more bytes than necessary (non-canonical encoding)"
        );
    }

    fn arb_layer() -> impl Strategy<Value = (Vec<i32>, Vec<Run>)> {
        prop::collection::vec((0_u32..9, any::<bool>(), 0_u32..3), 0..10).prop_flat_map(|shapes| {
            let mut runs = Vec::new();
            let mut start = 0;
            for (len, ring, gap) in shapes {
                start += gap;
                let kind = if ring { RunKind::Ring } else { RunKind::Line };
                runs.push(Run { start, len, kind });
                start += len;
            }
            let words = 2 * start.into_usize();
            (prop::collection::vec(0..3_i32, words), Just(runs))
        })
    }

    proptest! {
        #[test]
        fn any_plan_replays_to_its_vertices((vertices, runs) in arb_layer()) {
            let copies = plan(&vertices, &runs, &vec![2; vertices.len() / 2]);
            prop_assert_eq!(replay(&ops_of(&copies), &runs, &vertices).unwrap(), vertices);
        }
    }
}
