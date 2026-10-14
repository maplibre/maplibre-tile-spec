//! Triangle trees: a tessellation stored as two-bit symbols and the outline cycle of each tree.
//!
//! ```text
//! symbols   one per triangle in preorder: bit 0 a child across v->w, bit 1 a child across w->u
//! cycles    per tree of n triangles: the root's u and v, then n apexes in in-order
//! ```

use std::collections::{HashMap, HashSet};

use usize_cast::IntoUsize as _;

use crate::MltError::MalformedTriangles;
use crate::errors::AsMltError as _;
use crate::{Decoder, MltResult};

/// No child across that edge.
const NONE: u32 = u32::MAX;

/// A tessellation as tree symbols and tree cycles.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TriTrees {
    pub(crate) symbols: Vec<u32>,
    pub(crate) cycles: Vec<u32>,
}

fn rotated(t: [u32; 3], r: usize) -> [u32; 3] {
    [t[r % 3], t[(r + 1) % 3], t[(r + 2) % 3]]
}

/// Code the triangles of each feature as trees, `tri_offsets` cumulative in triangles.
pub(crate) fn encode_trees(indices: &[u32], tri_offsets: &[u32]) -> TriTrees {
    let mut out = TriTrees::default();
    for w in tri_offsets.windows(2) {
        let (lo, hi) = (w[0] as usize, w[1] as usize);
        let tris = indices[lo * 3..hi * 3].as_chunks::<3>().0;
        encode_feature(tris, &mut out);
    }
    out
}

fn encode_feature(tris: &[[u32; 3]], out: &mut TriTrees) {
    let n = tris.len();
    let mut edges: HashMap<(u32, u32), u32> = HashMap::with_capacity(n * 3);
    for (t, tri) in tris.iter().enumerate() {
        for r in 0..3 {
            edges
                .entry((tri[r], tri[(r + 1) % 3]))
                .or_insert(u32::try_from(t).expect("triangle count fits u32"));
        }
    }
    let mut first = FirstBuild::default();
    let mut tree_edges = HashSet::new();
    let roots = build(
        tris,
        &edges,
        (0..n).map(|t| (t, 0)),
        None,
        Some(&mut tree_edges),
        &mut first,
    );

    // Start each tree's cycle at its smallest vertex, so consecutive trees' cycles run on.
    let mut rerooted = Vec::with_capacity(roots.len());
    let mut c0 = 0;
    for &(root, size) in &roots {
        let cyc = &first.cycles[c0..c0 + size + 2];
        let m = cyc.len();
        let k = (0..m)
            .min_by_key(|&i| (cyc[i], cyc[(i + 1) % m] != cyc[i].wrapping_add(1)))
            .unwrap_or(0);
        let gate = (cyc[k], cyc[(k + 1) % m]);
        let owner = edges
            .get(&gate)
            .map(|&t| t as usize)
            .filter(|t| first.owner[*t] == root);
        rerooted.push(match owner {
            Some(t) => (t, rotation_of(tris[t], gate)),
            None => (root, first.rot0[root]),
        });
        c0 += size + 2;
    }
    let tail = (0..n).map(|t| (t, 0));
    build(
        tris,
        &edges,
        rerooted.into_iter().chain(tail),
        Some(&tree_edges),
        None,
        out,
    );
}

/// The rotation that makes `gate` the first edge of `tri`.
fn rotation_of(tri: [u32; 3], gate: (u32, u32)) -> u8 {
    let r = (0..3)
        .find(|&r| tri[r] == gate.0 && tri[(r + 1) % 3] == gate.1)
        .expect("the edge map points at a triangle holding the edge");
    u8::try_from(r).expect("rotation below 3")
}

/// Claim trees from `roots` in turn, emitting each into `out`, and return each tree's root and size.
///
/// With `allowed`, only edges in it join triangles.
/// With `record`, the edges used are added to it.
fn build(
    tris: &[[u32; 3]],
    edges: &HashMap<(u32, u32), u32>,
    roots: impl Iterator<Item = (usize, u8)>,
    allowed: Option<&HashSet<(u32, u32)>>,
    mut record: Option<&mut HashSet<(u32, u32)>>,
    out: &mut impl Emit,
) -> Vec<(usize, usize)> {
    let n = tris.len();
    let key = |p: u32, q: u32| (p.min(q), p.max(q));
    let mut visited = vec![false; n];
    let mut rot = vec![0u8; n];
    let mut child = vec![[NONE; 2]; n];
    let mut stack = Vec::new();
    let mut trees = Vec::new();
    out.begin(n);
    for (root, r0) in roots {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        rot[root] = r0;
        out.root(root, r0);
        let mut size = 0;
        stack.push(root);
        while let Some(t) = stack.pop() {
            size += 1;
            out.own(t, root);
            let [u, v, w] = rotated(tris[t], usize::from(rot[t]));
            for (slot, gate) in [(0, (w, v)), (1, (u, w))] {
                if allowed.is_some_and(|a| !a.contains(&key(gate.0, gate.1))) {
                    continue;
                }
                let Some(&nb) = edges.get(&gate) else {
                    continue;
                };
                let nb = nb as usize;
                if visited[nb] {
                    continue;
                }
                visited[nb] = true;
                rot[nb] = rotation_of(tris[nb], gate);
                child[t][slot] = u32::try_from(nb).expect("triangle count fits u32");
                if let Some(rec) = record.as_deref_mut() {
                    rec.insert(key(gate.0, gate.1));
                }
                stack.push(nb);
            }
        }
        trees.push((root, size));
        let apex = |t: usize| rotated(tris[t], usize::from(rot[t]))[2];
        let kid = |t: usize, slot: usize| (child[t][slot] != NONE).then(|| child[t][slot] as usize);

        stack.push(root);
        while let Some(t) = stack.pop() {
            let (a, b) = (kid(t, 0), kid(t, 1));
            out.symbol(u32::from(a.is_some()) | (u32::from(b.is_some()) << 1));
            stack.extend(b);
            stack.extend(a);
        }

        let [u, v, _] = rotated(tris[root], usize::from(rot[root]));
        out.vertex(u);
        out.vertex(v);
        let mut cur = Some(root);
        loop {
            while let Some(t) = cur {
                stack.push(t);
                cur = kid(t, 0);
            }
            let Some(t) = stack.pop() else { break };
            out.vertex(apex(t));
            cur = kid(t, 1);
        }
    }
    trees
}

/// Where a tree build writes what it found.
trait Emit {
    fn begin(&mut self, _triangles: usize) {}
    fn root(&mut self, _root: usize, _rotation: u8) {}
    fn own(&mut self, _t: usize, _root: usize) {}
    fn symbol(&mut self, s: u32);
    fn vertex(&mut self, v: u32);
}

impl Emit for TriTrees {
    fn symbol(&mut self, s: u32) {
        self.symbols.push(s);
    }
    fn vertex(&mut self, v: u32) {
        self.cycles.push(v);
    }
}

/// A first build, which also keeps who owns each triangle and how each root was entered.
#[derive(Default)]
struct FirstBuild {
    cycles: Vec<u32>,
    owner: Vec<usize>,
    rot0: Vec<u8>,
}

impl Emit for FirstBuild {
    fn begin(&mut self, triangles: usize) {
        self.owner = vec![usize::MAX; triangles];
        self.rot0 = vec![0; triangles];
    }
    fn root(&mut self, root: usize, rotation: u8) {
        self.rot0[root] = rotation;
    }
    fn own(&mut self, t: usize, root: usize) {
        self.owner[t] = root;
    }
    fn symbol(&mut self, _s: u32) {}
    fn vertex(&mut self, v: u32) {
        self.cycles.push(v);
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
    let mut size = vec![0u32; triangles + 1];
    // Slot 0 is a zero the pops of an absent child read.
    // Both stacks grow only as deep as a tree goes, so no buffer is sized by the triangle count.
    let mut sizes = vec![0u32; 64];
    let mut frames = vec![[0u32; 5]; 64];
    let (mut s0, mut c0) = (0usize, 0usize);
    while s0 < triangles {
        let mut pending = 1u32;
        let mut end = s0;
        while pending > 0 {
            let sym = *symbols
                .get(end)
                .ok_or(MalformedTriangles("symbols end inside a tree"))?;
            pending = pending - 1 + sym.count_ones();
            end += 1;
        }
        let mut sp = 1usize;
        for i in (s0..end).rev() {
            let (ha, hb) = (symbols[i] & 1, symbols[i] >> 1);
            debug_assert!(
                sp > (ha + hb).into_usize(),
                "the pending count proves every pop finds a subtree"
            );
            let left = sizes[sp - 1] * ha;
            sp -= ha.into_usize();
            let right = sizes[sp - 1] * hb;
            sp -= hb.into_usize();
            size[i] = 1 + left + right;
            if sp == sizes.len() {
                sizes.resize(sp * 2, 0);
            }
            sizes[sp] = size[i];
            sp += 1;
        }
        let tree_len = end - s0;
        let ring = cycles
            .get(c0..c0 + tree_len + 2)
            .ok_or(MalformedTriangles("cycles end inside a tree"))?;
        let apexes = &ring[2..];
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the triangle count fits u32"
        )]
        let (first, len) = (s0 as u32, tree_len as u32);
        frames[0] = [first, ring[0], ring[1], 0, len];
        let mut fp = 1usize;
        while fp > 0 {
            fp -= 1;
            let [node, u, v, lo, hi] = frames[fp];
            let sym = symbols[node.into_usize()];
            let (ha, hb) = (sym & 1, sym >> 1);
            let left = size[node.into_usize() + 1] * ha;
            let w = apexes[(lo + left).into_usize()];
            out.extend_from_slice(&[u, v, w]);
            if fp + 2 > frames.len() {
                frames.resize(frames.len() * 2, [0; 5]);
            }
            frames[fp] = [node + 1 + left, u, w, lo + left + 1, hi];
            fp += hb.into_usize();
            frames[fp] = [node + 1, w, v, lo, lo + left];
            fp += ha.into_usize();
        }
        s0 = end;
        c0 += tree_len + 2;
    }
    if c0 != cycles.len() {
        return Err(MalformedTriangles(
            "cycles hold more entries than the trees take",
        ));
    }
    Ok(out)
}

/// Whether both buffers hold the same triangles per feature, up to order and rotation.
pub(crate) fn same_triangles(a: &[u32], b: &[u32], tri_offsets: &[u32]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let canon = |s: &[u32]| {
        let mut v: Vec<[u32; 3]> = s
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&t| {
                let r = (0..3).min_by_key(|&r| rotated(t, r)).unwrap_or(0);
                rotated(t, r)
            })
            .collect();
        v.sort_unstable();
        v
    };
    tri_offsets.windows(2).all(|w| {
        let r = w[0] as usize * 3..w[1] as usize * 3;
        canon(&a[r.clone()]) == canon(&b[r])
    })
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
            for _ in 0..1 + below(3) {
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
