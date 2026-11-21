//! Meshes simplified by meshoptimizer within an error in metres: each triangle keeps its semantic
//! `part`, and vertices shared between parts slide along the seams where parts meet without
//! pulling wall triangles onto roofs or roof triangles down facades.

use meshopt::{SimplifyOptions, simplify_decoder};

use super::Mesh;
use super::parser::Part;

/// `mesh`, `metres` per grid unit along x, y and z, simplified within `error` metres while
/// keeping fold lines, the seams between parts, and each triangle's semantic `part`. When `prune`
/// is `true`, disconnected slivers smaller than `error` are removed; when `false`, open borders
/// are locked so overhanging eaves and unwelded T-junctions between `CityGML` roof facets cannot
/// tear apart or collapse away at coarse zoom levels. `None` if its vertices, one copy per part
/// they are in, are more than a `u32` indexes.
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
    // Each part gets its own copy of any vertex it shares: meshoptimizer keeps the seams between
    // parts, sliding their vertices along them, while preserving every triangle's semantic `Part`
    // (including steep spires, domes and barrel vaults).
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat square of `n` by `n` cells of 1 m, each two triangles facing up, as one part.
    fn grid(n: u32, part: Part) -> Mesh {
        let side = i32::try_from(n).unwrap() + 1;
        let xyz = (0..side * side).map(|i| [i % side, i / side, 0]).collect();
        let at = |x: u32, y: u32| y * (n + 1) + x;
        let triangles: Vec<[u32; 3]> = (0..n * n)
            .flat_map(|c| {
                let (x, y) = (c % n, c / n);
                let [nw, ne, sw, se] = [at(x, y), at(x + 1, y), at(x, y + 1), at(x + 1, y + 1)];
                [[nw, sw, ne], [ne, sw, se]]
            })
            .collect();
        let parts = vec![part; triangles.len()];
        Mesh {
            xyz,
            triangles,
            parts,
        }
    }

    #[test]
    fn a_flat_part_keeps_its_outline_and_loses_its_inside() {
        let roof = grid(4, Part::Roof);
        let simplified = simplify(&roof, [1.0; 3], 0.05, true).unwrap();
        assert!(simplified.triangles.len() < roof.triangles.len());
        assert!(simplified.parts.iter().all(|&p| p == Part::Roof));
        assert!(
            simplified
                .xyz
                .iter()
                .all(|&[x, y, _]| x % 4 == 0 || y % 4 == 0)
        );
    }

    #[test]
    fn the_line_two_parts_meet_on_stays() {
        // The west half of the grid a wall, the east half a roof: the vertices on the line between
        // them may slide along it, but each half keeps its 8 m².
        let mut mesh = grid(4, Part::Roof);
        for (t, part) in mesh.triangles.iter().zip(&mut mesh.parts) {
            if t.iter().all(|&v| mesh.xyz[v as usize][0] <= 2) {
                *part = Part::Wall;
            }
        }
        let simplified = simplify(&mesh, [1.0; 3], 0.05, false).unwrap();
        assert!(simplified.triangles.len() < mesh.triangles.len());
        for part in [Part::Wall, Part::Roof] {
            let doubled_area: i32 = simplified
                .triangles
                .iter()
                .zip(&simplified.parts)
                .filter(|&(_, &p)| p == part)
                .map(|(t, _)| {
                    let [a, b, c] = t.map(|v| simplified.xyz[v as usize]);
                    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs()
                })
                .sum();
            assert_eq!(doubled_area, 16, "{part:?}");
        }
    }

    #[test]
    fn prune_removes_debris_only_when_enabled() {
        // A 10 m roof square in cm, plus a disconnected 1 cm sliver.
        let mut mesh = grid(1, Part::Roof);
        for p in &mut mesh.xyz {
            *p = p.map(|v| v * 1000);
        }
        let n = u32::try_from(mesh.xyz.len()).unwrap();
        mesh.xyz
            .extend([[5000, 5000, 0], [5001, 5000, 0], [5000, 5001, 0]]);
        mesh.triangles.push([n, n + 2, n + 1]);
        mesh.parts.push(Part::Roof);

        let pruned = simplify(&mesh, [0.01; 3], 0.05, true).unwrap();
        assert_eq!(pruned.triangles.len(), 2);
        assert_eq!(pruned.parts, [Part::Roof, Part::Roof]);

        let unpruned = simplify(&mesh, [0.01; 3], 0.05, false).unwrap();
        assert_eq!(unpruned.triangles.len(), 3);
        assert!(unpruned.parts.iter().all(|&p| p == Part::Roof));
    }

    #[test]
    fn a_steep_spire_roof_keeps_its_roof_part_when_simplified() {
        // 8 x 8 m brick tower rising to 40 m (`Wall`), topped by an 81-degree spire rising to 65 m (`Roof`).
        let xyz = vec![
            [0, 0, 0],
            [8, 0, 0],
            [8, -8, 0],
            [0, -8, 0],
            [0, 0, 40],
            [8, 0, 40],
            [8, -8, 40],
            [0, -8, 40],
            [4, -4, 65],
        ];
        let triangles = vec![
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
            [4, 5, 8],
            [5, 6, 8],
            [6, 7, 8],
            [7, 4, 8],
        ];
        let mut parts = vec![Part::Wall; 8];
        parts.extend([Part::Roof; 4]);
        let mesh = Mesh {
            xyz,
            triangles,
            parts,
        };
        let simplified = simplify(&mesh, [1.0; 3], 1.5, false).unwrap();
        let max_roof_z = simplified
            .triangles
            .iter()
            .zip(&simplified.parts)
            .filter(|&(_, &p)| p == Part::Roof)
            .flat_map(|(t, _)| t.map(|v| simplified.xyz[v as usize][2]))
            .max();
        let max_wall_z = simplified
            .triangles
            .iter()
            .zip(&simplified.parts)
            .filter(|&(_, &p)| p == Part::Wall)
            .flat_map(|(t, _)| t.map(|v| simplified.xyz[v as usize][2]))
            .max();
        assert_eq!(max_roof_z, Some(65));
        assert_eq!(max_wall_z, Some(40));
    }
}
