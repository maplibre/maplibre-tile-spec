//! Closed shells of a mesh turned to face outward.

use rustc_hash::FxHashMap;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

/// Turns each closed shell component of `triangles` whose triangles face inward (`(x, -y, z)`
/// signed volume `< 0`) outward, so every triangle winds counter-clockwise seen from outside.
/// Open shells have no enclosed volume and keep their winding.
/// Returns how many shells were turned.
pub fn orient_outward(xyz: &[[i32; 3]], triangles: &mut [[u32; 3]]) -> usize {
    let count = triangles.len();
    if count < 4 {
        return 0;
    }
    let mut components = QuickUnionUf::<UnionBySize>::new(count);
    let mut by_edge: Vec<(u64, usize)> = Vec::with_capacity(count * 3);
    for (i, &[a, b, c]) in triangles.iter().enumerate() {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let (lo, hi) = (u.min(v), u.max(v));
            by_edge.push(((u64::from(lo) << 32) | u64::from(hi), i));
        }
    }
    by_edge.sort_unstable_by_key(|&(edge, _)| edge);
    for pair in by_edge.windows(2) {
        if pair[0].0 == pair[1].0 {
            components.union(pair[0].1, pair[1].1);
        }
    }
    let mut shells: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for i in 0..count {
        shells.entry(components.find(i)).or_default().push(i);
    }
    let mut turned = 0;
    let mut shell = Vec::new();
    for indices in shells.into_values() {
        if indices.len() < 4 {
            continue;
        }
        shell.clear();
        shell.extend(indices.iter().map(|&i| triangles[i]));
        if is_closed_shell(&shell) && signed_volume_x6(xyz, &shell) < 0 {
            for i in indices {
                triangles[i].swap(1, 2);
            }
            turned += 1;
        }
    }
    turned
}

/// Whether every directed edge `(a, b)` of `triangles` is paired with `(b, a)`, so the mesh
/// encloses a volume without open boundary edges.
pub fn is_closed_shell(triangles: &[[u32; 3]]) -> bool {
    let mut forward = Vec::with_capacity(triangles.len() * 3);
    let mut reverse = Vec::with_capacity(triangles.len() * 3);
    for &[a, b, c] in triangles {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            forward.push((u64::from(u) << 32) | u64::from(v));
            reverse.push((u64::from(v) << 32) | u64::from(u));
        }
    }
    forward.sort_unstable();
    reverse.sort_unstable();
    forward == reverse
}

/// Six times the signed volume enclosed by `triangles` in right-handed `(x, -y, z)` coordinates,
/// positive when triangles wind counter-clockwise seen from outside.
pub fn signed_volume_x6(xyz: &[[i32; 3]], triangles: &[[u32; 3]]) -> i128 {
    let Some(&[x0, y0, z0]) = xyz.first() else {
        return 0;
    };
    let at = |v: u32| {
        let [x, y, z] = xyz[v as usize];
        [
            i128::from(x) - i128::from(x0),
            -(i128::from(y) - i128::from(y0)),
            i128::from(z) - i128::from(z0),
        ]
    };
    triangles
        .iter()
        .map(|&[a, b, c]| {
            let ([ax, ay, az], [bx, by, bz], [cx, cy, cz]) = (at(a), at(b), at(c));
            ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx)
        })
        .sum()
}
