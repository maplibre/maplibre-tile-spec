//! Triangle trees: a tessellation stored as two-bit symbols and the outline cycle of each tree.
//!
//! ```text
//! symbols   one per triangle in preorder: bit 0 a child across v->w, bit 1 a child across w->u
//! cycles    per tree of n triangles: the root's u and v, then n apexes in in-order
//! ```

use std::collections::HashMap;

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
    for &[lo, hi] in tri_offsets.array_windows() {
        let tris = &indices[lo.into_usize() * 3..hi.into_usize() * 3];
        let forest = Forest::grow(tris.as_chunks::<3>().0);
        for &root in &forest.roots {
            // Rerooting a tree only rotates its outline, so each tree starts at its smallest vertex.
            // Consecutive trees' cycles then run on.
            let mut outline = forest.outline(Side { tri: root, edge: 0 });
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

/// Side `edge` of triangle `tri`, running from its vertex `edge` to the next.
#[derive(Clone, Copy, Debug)]
struct Side {
    tri: usize,
    edge: usize,
}

impl Side {
    /// The other two sides of the triangle, in winding order after this one.
    fn rest(self) -> [Self; 2] {
        [1, 2].map(|k| Self {
            tri: self.tri,
            edge: (self.edge + k) % 3,
        })
    }
}

/// A spanning forest over the triangles of one feature.
struct Forest<'a> {
    tris: &'a [[u32; 3]],
    /// The side of the tree neighbour across each side.
    twins: Vec<[Option<Side>; 3]>,
    /// The triangle each tree was claimed from, in claim order.
    roots: Vec<usize>,
}

impl<'a> Forest<'a> {
    /// Claim a tree from each unclaimed triangle in turn.
    ///
    /// A side joins the first triangle holding the same edge reversed, unless that one is claimed already.
    fn grow(tris: &'a [[u32; 3]]) -> Self {
        let mut forest = Self {
            tris,
            twins: vec![[None; 3]; tris.len()],
            roots: Vec::new(),
        };
        let mut holder = HashMap::with_capacity(tris.len() * 3);
        for tri in 0..tris.len() {
            for edge in 0..3 {
                let side = Side { tri, edge };
                holder.entry(forest.ends(side)).or_insert(side);
            }
        }
        let mut claimed = vec![false; tris.len()];
        let mut stack = Vec::new();
        for root in 0..tris.len() {
            if claimed[root] {
                continue;
            }
            claimed[root] = true;
            forest.roots.push(root);
            stack.push(Side { tri: root, edge: 0 });
            while let Some(entry) = stack.pop() {
                for side in entry.rest() {
                    let (from, to) = forest.ends(side);
                    if let Some(&child) = holder.get(&(to, from))
                        && !claimed[child.tri]
                    {
                        claimed[child.tri] = true;
                        forest.twins[side.tri][side.edge] = Some(child);
                        forest.twins[child.tri][child.edge] = Some(side);
                        stack.push(child);
                    }
                }
            }
        }
        forest
    }

    fn ends(&self, side: Side) -> (u32, u32) {
        let t = self.tris[side.tri];
        (t[side.edge], t[(side.edge + 1) % 3])
    }

    fn twin(&self, side: Side) -> Option<Side> {
        self.twins[side.tri][side.edge]
    }

    /// Append the symbol of each triangle of the tree entered across `gate`, in preorder.
    fn symbols(&self, gate: Side, out: &mut Vec<u32>) {
        let mut stack = vec![gate];
        while let Some(entry) = stack.pop() {
            let [left, right] = entry.rest().map(|s| self.twin(s));
            out.push(u32::from(left.is_some()) | (u32::from(right.is_some()) << 1));
            stack.extend(right);
            stack.extend(left);
        }
    }

    /// The sides of the tree entered across `gate` that join nothing, in winding order from `gate`.
    fn outline(&self, gate: Side) -> Vec<Side> {
        let mut outline = vec![gate];
        let mut stack: Vec<_> = gate.rest().into_iter().rev().collect();
        while let Some(side) = stack.pop() {
            match self.twin(side) {
                Some(child) => stack.extend(child.rest().into_iter().rev()),
                None => outline.push(side),
            }
        }
        outline
    }
}

/// Rebuild three vertex indices per symbol from the trees.
///
/// Branch-free over the symbols: a node's flags move the stack pointers instead of choosing a path.
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
    let mut out = Vec::with_capacity(triangles * 3);
    // Both stacks grow only as deep as a tree goes, so no buffer is sized by the triangle count.
    let mut size_stack = vec![0u32; 64];
    let mut frames = vec![Frame::default(); 64];
    let mut sizes = vec![0u32; triangles + 1];
    let (mut symbols, mut cycles) = (symbols, cycles);
    while !symbols.is_empty() {
        let (tree, rest) = symbols.split_at(tree_len(symbols)?);
        let (cycle, cycles_rest) = cycles
            .split_at_checked(tree.len() + 2)
            .ok_or(MalformedTriangles("cycles end inside a tree"))?;
        let sizes = &mut sizes[..=tree.len()];
        subtree_sizes(tree, sizes, &mut size_stack);
        unfold(tree, sizes, cycle, &mut frames, &mut out);
        (symbols, cycles) = (rest, cycles_rest);
    }
    if !cycles.is_empty() {
        return Err(MalformedTriangles(
            "cycles hold more entries than the trees take",
        ));
    }
    Ok(out)
}

/// The number of symbols in the tree at the front of `symbols`.
fn tree_len(symbols: &[u32]) -> MltResult<usize> {
    let mut pending = 1u32;
    symbols
        .iter()
        .position(|&sym| {
            pending = pending - 1 + sym.count_ones();
            pending == 0
        })
        .map(|last| last + 1)
        .ok_or(MalformedTriangles("symbols end inside a tree"))
}

/// Fill `sizes` with the triangle count of each node's subtree.
///
/// Slot 0 of `stack` is a zero the pops of an absent child read.
fn subtree_sizes(tree: &[u32], sizes: &mut [u32], stack: &mut Vec<u32>) {
    let mut sp = 1;
    for (node, &sym) in tree.iter().enumerate().rev() {
        let (has_left, has_right) = (sym & 1, sym >> 1);
        debug_assert!(
            sp > (has_left + has_right).into_usize(),
            "the pending count proves every pop finds a subtree"
        );
        let left = stack[sp - 1] * has_left;
        sp -= has_left.into_usize();
        let right = stack[sp - 1] * has_right;
        sp -= has_right.into_usize();
        sizes[node] = 1 + left + right;
        if sp == stack.len() {
            stack.resize(sp * 2, 0);
        }
        stack[sp] = sizes[node];
        sp += 1;
    }
}

/// A subtree still to unfold: its first node, the edge it is entered across, and its first apex.
#[derive(Clone, Copy, Default)]
struct Frame {
    node: u32,
    u: u32,
    v: u32,
    apex: u32,
}

/// Append the triangles of `tree` to `out`, taking each apex from `cycle` by the left subtree's size.
fn unfold(tree: &[u32], sizes: &[u32], cycle: &[u32], frames: &mut Vec<Frame>, out: &mut Vec<u32>) {
    let apexes = &cycle[2..];
    frames[0] = Frame {
        node: 0,
        u: cycle[0],
        v: cycle[1],
        apex: 0,
    };
    let mut fp = 1;
    while fp > 0 {
        fp -= 1;
        let Frame { node, u, v, apex } = frames[fp];
        let sym = tree[node.into_usize()];
        let (has_left, has_right) = (sym & 1, sym >> 1);
        let left = sizes[node.into_usize() + 1] * has_left;
        let w = apexes[(apex + left).into_usize()];
        out.extend_from_slice(&[u, v, w]);
        if fp + 2 > frames.len() {
            frames.resize(frames.len() * 2, Frame::default());
        }
        frames[fp] = Frame {
            node: node + 1 + left,
            u,
            v: w,
            apex: apex + left + 1,
        };
        fp += has_right.into_usize();
        frames[fp] = Frame {
            node: node + 1,
            u: w,
            v,
            apex,
        };
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
