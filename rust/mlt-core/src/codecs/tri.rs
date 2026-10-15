//! Triangle trees: a tessellation stored as two-bit symbols and the outline cycle of each tree.
//!
//! ```text
//! symbols   one per triangle in preorder: bit 0 a child across v->w, bit 1 a child across w->u
//! cycles    per tree of n triangles: the root's u and v, then n apexes in in-order
//! ```

use usize_cast::IntoUsize as _;

use crate::MltError::MalformedTriangles;
use crate::errors::AsMltError as _;
use crate::{Decoder, MltResult};

/// A tessellation as tree symbols and tree cycles.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TriTrees {
    pub(crate) symbols: Vec<u32>,
    pub(crate) cycles: Vec<u32>,
}

/// Code the triangles of each feature as trees, `tri_offsets` cumulative in triangles.
pub(crate) fn encode_trees(indices: &[u32], tri_offsets: &[u32]) -> TriTrees {
    let mut out = TriTrees::default();
    let mut forest = Forest::default();
    let mut outline = Vec::new();
    for &[lo, hi] in tri_offsets.array_windows() {
        forest.grow(&indices[lo.into_usize() * 3..hi.into_usize() * 3]);
        for i in 0..forest.roots.len() {
            // Rerooting a tree only rotates its outline, so each tree starts at its smallest vertex.
            // Consecutive trees' cycles then run on.
            forest.outline(forest.roots[i] * 3, &mut outline);
            let k = (0..outline.len())
                .min_by_key(|&i| {
                    let (from, to) = forest.ends(outline[i]);
                    (from, to != from.wrapping_add(1))
                })
                .unwrap_or(0);
            outline.rotate_left(k);
            forest.symbols(outline[0], &mut out.symbols);
            out.cycles.extend(outline.iter().map(|&s| forest.ends(s).0));
        }
    }
    out
}

/// No side: the twin of a side that joins nothing.
const NONE: u32 = u32::MAX;

/// The side after side `side` of its triangle, side `3 * tri + k` running from vertex `k` to the next.
fn next(side: u32) -> u32 {
    if side % 3 == 2 { side - 2 } else { side + 1 }
}

/// The other two sides of the triangle, in winding order after this one.
fn rest(side: u32) -> [u32; 2] {
    let after = next(side);
    [after, next(after)]
}

/// A spanning forest over the triangles of one feature, its buffers kept for the next feature.
#[derive(Default)]
struct Forest<'a> {
    indices: &'a [u32],
    /// The side of the tree neighbour across each side, or [`NONE`].
    twins: Vec<u32>,
    /// The triangle each tree was claimed from, in claim order.
    roots: Vec<u32>,
    /// The sides by the bucket of their first vertex, each bucket in side order.
    by_from: Vec<(u64, u32)>,
    /// Where each bucket of [`Self::by_from`] starts, then where the last one ends.
    starts: Vec<u32>,
    /// The first vertex of the feature, and the bucket mask over vertices counted from it.
    min: u32,
    mask: u32,
    claimed: Vec<bool>,
    stack: Vec<u32>,
}

impl<'a> Forest<'a> {
    /// Claim a tree from each unclaimed triangle in turn.
    ///
    /// A side joins the first triangle holding the same edge reversed, unless that one is claimed already.
    fn grow(&mut self, indices: &'a [u32]) {
        self.indices = indices;
        let tris = u32::try_from(indices.len() / 3).expect("fewer than 2^32 triangles");
        self.twins.clear();
        self.twins.resize(indices.len(), NONE);
        self.roots.clear();
        self.bucket_sides();
        self.claimed.clear();
        self.claimed.resize(indices.len() / 3, false);
        for root in 0..tris {
            if self.claimed[root.into_usize()] {
                continue;
            }
            self.claimed[root.into_usize()] = true;
            self.roots.push(root);
            self.stack.push(root * 3);
            while let Some(entry) = self.stack.pop() {
                for side in rest(entry) {
                    let (from, to) = self.ends(side);
                    if let Some(child) = self.holder(pair(to, from))
                        && !self.claimed[(child / 3).into_usize()]
                    {
                        self.claimed[(child / 3).into_usize()] = true;
                        self.twins[side.into_usize()] = child;
                        self.twins[child.into_usize()] = side;
                        self.stack.push(child);
                    }
                }
            }
        }
    }

    fn ends(&self, side: u32) -> (u32, u32) {
        (
            self.indices[side.into_usize()],
            self.indices[next(side).into_usize()],
        )
    }

    fn key(&self, side: u32) -> u64 {
        let (from, to) = self.ends(side);
        pair(from, to)
    }

    fn bucket(&self, vertex: u32) -> usize {
        (vertex.wrapping_sub(self.min) & self.mask).into_usize()
    }

    /// Counting-sort the sides into buckets by their first vertex.
    ///
    /// The vertices of an earcut feature are consecutive, so a bucket rarely holds more than one vertex.
    fn bucket_sides(&mut self) {
        let sides = u32::try_from(self.indices.len()).expect("fewer than 2^32 sides");
        self.min = self.indices.iter().copied().min().unwrap_or(0);
        self.mask = sides.next_power_of_two() - 1;
        self.starts.clear();
        self.starts.resize(self.mask.into_usize() + 3, 0);
        for &v in self.indices {
            let b = self.bucket(v);
            self.starts[b + 2] += 1;
        }
        for b in 2..self.starts.len() {
            self.starts[b] += self.starts[b - 1];
        }
        self.by_from.clear();
        self.by_from.resize(self.indices.len(), (0, 0));
        for side in 0..sides {
            let b = self.bucket(self.indices[side.into_usize()]);
            let key = self.key(side);
            let at = self.starts[b + 1];
            self.by_from[at.into_usize()] = (key, side);
            self.starts[b + 1] = at + 1;
        }
    }

    /// The first side running from `key`'s first vertex to its second.
    fn holder(&self, key: u64) -> Option<u32> {
        let b = self.bucket((key >> 32) as u32);
        let bucket = &self.by_from[self.starts[b].into_usize()..self.starts[b + 1].into_usize()];
        bucket
            .iter()
            .find(|&&(k, _)| k == key)
            .map(|&(_, side)| side)
    }

    fn joins(&self, side: u32) -> bool {
        self.twins[side.into_usize()] != NONE
    }

    /// Append the symbol of each triangle of the tree entered across `gate`, in preorder.
    fn symbols(&mut self, gate: u32, out: &mut Vec<u32>) {
        self.stack.push(gate);
        while let Some(entry) = self.stack.pop() {
            let [left, right] = rest(entry);
            out.push(u32::from(self.joins(left)) | (u32::from(self.joins(right)) << 1));
            for side in [right, left] {
                if self.joins(side) {
                    self.stack.push(self.twins[side.into_usize()]);
                }
            }
        }
    }

    /// The sides of the tree entered across `gate` that join nothing, in winding order from `gate`.
    fn outline(&mut self, gate: u32, outline: &mut Vec<u32>) {
        outline.clear();
        outline.push(gate);
        let [left, right] = rest(gate);
        self.stack.extend([right, left]);
        while let Some(side) = self.stack.pop() {
            match self.twins[side.into_usize()] {
                NONE => outline.push(side),
                child => {
                    let [left, right] = rest(child);
                    self.stack.extend([right, left]);
                }
            }
        }
    }
}

fn pair(from: u32, to: u32) -> u64 {
    u64::from(from) << 32 | u64::from(to)
}

/// Rebuild three vertex indices per symbol from the trees.
///
/// Branch-free over the symbols: a node's flags move the stack pointer instead of choosing a path.
pub(crate) fn decode_trees(
    symbols: &[u32],
    cycles: &[u32],
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    if symbols.iter().any(|&s| s > 3) {
        return Err(MalformedTriangles("symbol above 3"));
    }
    let triangles = symbols.len();
    u32::try_from(triangles).or_overflow()?;
    dec.consume_items::<u32>(triangles.saturating_mul(3))?;
    let sizes = subtree_sizes(symbols)?;
    let mut out = Vec::with_capacity(triangles * 3);
    // The stack grows only as deep as a tree goes, so it is not sized by the triangle count.
    let mut frames = vec![Frame::default(); 64];
    let (mut root, mut cycles) = (0, cycles);
    while root < triangles {
        let tree = sizes[root].into_usize();
        let (cycle, rest) = cycles
            .split_at_checked(tree + 2)
            .ok_or(MalformedTriangles("cycles end inside a tree"))?;
        unfold(symbols, &sizes, root, tree, cycle, &mut frames, &mut out);
        (root, cycles) = (root + tree, rest);
    }
    if !cycles.is_empty() {
        return Err(MalformedTriangles(
            "cycles hold more entries than the trees take",
        ));
    }
    Ok(out)
}

/// The triangle count of each node's subtree, and a zero past the last node.
///
/// A node's left subtree starts right after it and its right subtree right after that.
/// No stack is needed: walking backwards, both are sized already.
fn subtree_sizes(symbols: &[u32]) -> MltResult<Vec<u32>> {
    let end = symbols.len();
    let mut sizes = vec![0u32; end + 1];
    let mut open = 0;
    for (node, &sym) in symbols.iter().enumerate().rev() {
        let (has_left, has_right) = (sym & 1, sym >> 1);
        // Every size spans no further than the end, so the right child is at most one past it.
        let left = sizes[node + 1] * has_left;
        let right_at = node + 1 + left.into_usize();
        let right = sizes[right_at] * has_right;
        open |= (has_left & u32::from(node + 1 == end)) | (has_right & u32::from(right_at == end));
        sizes[node] = 1 + left + right;
    }
    if open == 0 {
        Ok(sizes)
    } else {
        Err(MalformedTriangles("symbols end inside a tree"))
    }
}

/// A subtree still to unfold: the edge it is entered across, and its first apex.
#[derive(Clone, Copy, Default)]
struct Frame {
    u: u32,
    v: u32,
    apex: u32,
}

/// Append the `tree` triangles from `root`, taking each apex from `cycle` by the left subtree's size.
///
/// The frames pop in preorder, so the node a frame enters is the next one.
fn unfold(
    symbols: &[u32],
    sizes: &[u32],
    root: usize,
    tree: usize,
    cycle: &[u32],
    frames: &mut Vec<Frame>,
    out: &mut Vec<u32>,
) {
    let apexes = &cycle[2..];
    frames[0] = Frame {
        u: cycle[0],
        v: cycle[1],
        apex: 0,
    };
    let mut fp = 1;
    for node in root..root + tree {
        fp -= 1;
        let Frame { u, v, apex } = frames[fp];
        let sym = symbols[node];
        let (has_left, has_right) = (sym & 1, sym >> 1);
        let left = sizes[node + 1] * has_left;
        let w = apexes[(apex + left).into_usize()];
        out.extend_from_slice(&[u, v, w]);
        if fp + 2 > frames.len() {
            frames.resize(frames.len() * 2, Frame::default());
        }
        frames[fp] = Frame {
            u,
            v: w,
            apex: apex + left + 1,
        };
        fp += has_right.into_usize();
        frames[fp] = Frame { u: w, v, apex };
        fp += has_left.into_usize();
    }
}

/// Whether both buffers hold the same triangles per feature, up to order and rotation.
pub(crate) fn same_triangles(a: &[u32], b: &[u32], tri_offsets: &[u32]) -> bool {
    a.len() == b.len()
        && tri_offsets.array_windows().all(|&[lo, hi]| {
            let range = lo.into_usize() * 3..hi.into_usize() * 3;
            canonical(&a[range.clone()]) == canonical(&b[range])
        })
}

/// The triangles each rotated to start lexicographically smallest, then sorted.
fn canonical(indices: &[u32]) -> Vec<[u32; 3]> {
    let mut tris: Vec<_> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&t| {
            (0..3)
                .map(|r| {
                    let mut t = t;
                    t.rotate_left(r);
                    t
                })
                .min()
                .unwrap_or(t)
        })
        .collect();
    tris.sort_unstable();
    tris
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{dec, starved_dec};

    #[test]
    fn fan_with_a_hole_bridge_round_trips() {
        let triangles = [[0, 1, 2], [0, 2, 3], [0, 3, 4], [4, 3, 5], [6, 7, 8]];
        let tri_offsets = [0, 4, 5];
        let trees = encode_trees(triangles.as_flattened(), &tri_offsets);
        assert_eq!(
            trees,
            TriTrees {
                symbols: vec![2, 2, 1, 0, 0],
                cycles: vec![0, 1, 2, 3, 5, 4, 6, 7, 8],
            }
        );
        let decoded = decode_trees(&trees.symbols, &trees.cycles, &mut dec()).unwrap();
        assert!(same_triangles(
            triangles.as_flattened(),
            &decoded,
            &tri_offsets
        ));
    }

    #[test]
    fn spec_example_encodes() {
        let triangles = [
            [1, 0, 2],
            [3, 6, 8],
            [7, 6, 3],
            [4, 3, 8],
            [7, 3, 5],
            [5, 4, 8],
            [8, 7, 5],
            [10, 9, 11],
        ];
        assert_eq!(
            encode_trees(triangles.as_flattened(), &[0, 1, 8]),
            TriTrees {
                symbols: vec![0, 2, 1, 1, 2, 1, 0, 0],
                cycles: vec![0, 2, 1, 3, 6, 8, 7, 6, 3, 5, 4, 9, 11, 10],
            }
        );
    }

    #[test]
    fn spec_example_decodes() {
        let symbols = [0, 2, 1, 1, 2, 1, 0, 0];
        let cycles = [0, 2, 1, 3, 6, 8, 7, 6, 3, 5, 4, 9, 11, 10];
        let triangles = [
            [0, 2, 1],
            [3, 6, 8],
            [3, 8, 4],
            [4, 8, 5],
            [5, 8, 7],
            [5, 7, 3],
            [3, 7, 6],
            [9, 11, 10],
        ];
        assert_eq!(
            decode_trees(&symbols, &cycles, &mut dec()).unwrap(),
            triangles.as_flattened()
        );
    }

    #[test]
    fn no_symbols_decode_to_no_triangles() {
        assert_eq!(
            decode_trees(&[], &[], &mut dec()).unwrap(),
            Vec::<u32>::new()
        );
    }

    #[test]
    fn a_comb_of_100_two_child_triangles_outgrows_the_starting_stacks_and_round_trips() {
        let symbols = [vec![3; 100], vec![0; 101]].concat();
        let cycles: Vec<u32> = (0..203).collect();
        let triangles = decode_trees(&symbols, &cycles, &mut dec()).unwrap();
        assert_eq!(triangles.len(), 201 * 3);
        assert_eq!(
            encode_trees(&triangles, &[0, 201]),
            TriTrees { symbols, cycles }
        );
    }

    #[test]
    fn decoding_past_the_memory_budget_is_rejected() {
        let err = decode_trees(&[0], &[0, 1, 2], &mut starved_dec()).unwrap_err();
        assert!(
            matches!(err, crate::MltError::MemoryLimitExceeded { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn every_input_of_up_to_6_symbols_decodes_three_indices_per_symbol_or_errs() {
        for symbol_count in 0..=6 {
            for digits in 0..5u32.pow(symbol_count) {
                let symbols: Vec<u32> = (0..symbol_count)
                    .map(|i| digits / 5u32.pow(i) % 5)
                    .collect();
                for cycle_count in 0..symbol_count + 5 {
                    let cycles: Vec<u32> = (0..cycle_count).collect();
                    if let Ok(triangles) = decode_trees(&symbols, &cycles, &mut dec()) {
                        assert_eq!(triangles.len(), symbols.len() * 3);
                    }
                }
            }
        }
    }

    #[test]
    fn random_triangle_soups_round_trip() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut below = |bound: u32| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            u32::try_from(state % u64::from(bound)).unwrap()
        };
        for _ in 0..2000 {
            let vertex_count = 3 + below(8);
            let mut tri_offsets = vec![0];
            let mut indices = Vec::new();
            for _ in 0..=below(3) {
                let triangle_count = below(12);
                for _ in 0..triangle_count * 3 {
                    indices.push(below(vertex_count));
                }
                tri_offsets.push(tri_offsets.last().unwrap() + triangle_count);
            }
            let trees = encode_trees(&indices, &tri_offsets);
            let decoded = decode_trees(&trees.symbols, &trees.cycles, &mut dec()).unwrap();
            assert!(
                same_triangles(&indices, &decoded, &tri_offsets),
                "{indices:?} {tri_offsets:?}"
            );
        }
    }

    #[rstest::rstest]
    #[case::reordered_and_rotated(&[[0, 1, 2], [3, 4, 5]], &[[4, 5, 3], [1, 2, 0]], &[0, 2], true)]
    #[case::reversed_winding(&[[0, 1, 2]], &[[0, 2, 1]], &[0, 1], false)]
    #[case::moved_to_another_feature(&[[0, 1, 2], [3, 4, 5]], &[[3, 4, 5], [0, 1, 2]], &[0, 1, 2], false)]
    #[case::one_triangle_missing(&[[0, 1, 2], [3, 4, 5]], &[[0, 1, 2]], &[0, 1], false)]
    fn same_triangles_ignores_order_and_rotation_only(
        #[case] a: &[[u32; 3]],
        #[case] b: &[[u32; 3]],
        #[case] tri_offsets: &[u32],
        #[case] same: bool,
    ) {
        assert_eq!(
            same_triangles(a.as_flattened(), b.as_flattened(), tri_offsets),
            same
        );
    }

    #[rstest::rstest]
    #[case::symbol_above_3(&[4], &[0, 1, 2], "symbol above 3")]
    #[case::symbols_end_inside_a_tree(&[1], &[0, 1, 2, 3], "symbols end inside a tree")]
    #[case::cycles_end_inside_a_tree(&[0], &[0, 1], "cycles end inside a tree")]
    #[case::cycles_too_long(&[0], &[0, 1, 2, 3], "cycles hold more entries than the trees take")]
    #[case::cycles_without_symbols(&[], &[0, 1, 2], "cycles hold more entries than the trees take")]
    fn malformed_trees_are_rejected(
        #[case] symbols: &[u32],
        #[case] cycles: &[u32],
        #[case] reason: &str,
    ) {
        let err = decode_trees(symbols, cycles, &mut dec()).unwrap_err();
        assert!(matches!(err, MalformedTriangles(r) if r == reason), "{err}");
    }
}
