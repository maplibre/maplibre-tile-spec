//! Meshes simplified by meshoptimizer, each triangle keeping its `part`.

use meshopt::{SimplifyOptions, simplify_decoder};

use super::mesh::Mesh;
use super::model::Part;

/// `mesh`, of `metres` per grid unit along x, y and z, simplified within `error` metres, keeping its fold lines and the seams between parts.
/// With `prune`, disconnected pieces smaller than `error` are removed, and without it, open borders stay where they are.
/// `None` if its vertices, one copy per part they are in, are more than a `u32` indexes.
pub(super) fn simplify(mesh: &Mesh, metres: [f64; 3], error: f64, prune: bool) -> Option<Mesh> {
    let Some(&origin) = mesh.xyz.first() else {
        return compact(&[], Vec::new(), Vec::new());
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "metres from a building's first vertex, within a tile, fit an f32"
    )]
    let positions: Vec<[f32; 3]> = mesh
        .xyz
        .iter()
        .map(|p| [0, 1, 2].map(|i| ((f64::from(p[i]) - f64::from(origin[i])) * metres[i]) as f32))
        .collect();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an error of centimetres to metres fits an f32"
    )]
    let error = error as f32;
    let mut options = SimplifyOptions::ErrorAbsolute | SimplifyOptions::PreserveFolds;
    if prune {
        options |= SimplifyOptions::Prune;
    } else {
        options |= SimplifyOptions::LockBorder;
    }
    // A copy of each vertex per part it is in makes the seams between parts borders that meshoptimizer keeps.
    let mut copies: Vec<Vec<(Part, u32)>> = vec![Vec::new(); mesh.xyz.len()];
    let mut source = Vec::new();
    let mut part_of = Vec::new();
    let split: Vec<u32> = mesh
        .triangles
        .iter()
        .zip(&mesh.parts)
        .flat_map(|(t, &part)| t.map(|v| (v, part)))
        .map(|(v, part)| {
            let copies = &mut copies[v as usize];
            if let Some(&(_, copy)) = copies.iter().find(|&&(p, _)| p == part) {
                return Some(copy);
            }
            let copy = u32::try_from(source.len()).ok()?;
            copies.push((part, copy));
            source.push(v);
            part_of.push(part);
            Some(copy)
        })
        .collect::<Option<_>>()?;
    let split_positions: Vec<[f32; 3]> = source.iter().map(|&v| positions[v as usize]).collect();
    let simplified = simplify_decoder(&split, &split_positions, 0, error, options, None);
    let mut triangles = Vec::new();
    let mut kept = Vec::new();
    for t in simplified.as_chunks::<3>().0 {
        triangles.push(t.map(|copy| source[copy as usize]));
        kept.push(part_of[t[0] as usize]);
    }
    compact(&mesh.xyz, triangles, kept)
}

/// The mesh of the `triangles` and their `parts` between `xyz`, without the vertices none names.
fn compact(xyz: &[[i32; 3]], mut triangles: Vec<[u32; 3]>, parts: Vec<Part>) -> Option<Mesh> {
    let mut renumbered: Vec<Option<u32>> = vec![None; xyz.len()];
    let mut kept = Vec::new();
    for t in &mut triangles {
        for v in t {
            let new = if let Some(new) = renumbered[*v as usize] {
                new
            } else {
                let new = u32::try_from(kept.len()).ok()?;
                kept.push(xyz[*v as usize]);
                renumbered[*v as usize] = Some(new);
                new
            };
            *v = new;
        }
    }
    Some(Mesh {
        xyz: kept,
        triangles,
        parts,
    })
}
