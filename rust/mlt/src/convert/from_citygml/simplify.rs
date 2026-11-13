//! Meshes simplified by meshoptimizer within an error in metres, removing the pieces smaller than
//! it: with the seams where parts meet kept, their vertices sliding along them, or as a whole, each
//! triangle then of the part its facing says.

use meshopt::{SimplifyOptions, simplify_decoder};

use super::parser::Part;
use super::{Mesh, ROOF_MIN_UP};

/// How the parts of a mesh are kept by [`simplify`].
#[derive(Clone, Copy)]
pub(super) enum Parts {
    /// The parts are simplified together, the seams between them kept, their vertices sliding
    /// along them.
    Seamed,
    /// The mesh is simplified as a whole, and a triangle facing up is a roof, one facing down a
    /// ground, and the others walls.
    Faced,
}

/// `mesh`, `metres` per grid unit along x, y and z, simplified within `error` metres. `None` if
/// its vertices, one copy per part they are in, are more than a `u32` indexes.
pub(super) fn simplify(mesh: &Mesh, metres: [f64; 3], error: f64, parts: Parts) -> Option<Mesh> {
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
    let options = SimplifyOptions::ErrorAbsolute | SimplifyOptions::Prune;
    let mut triangles = Vec::new();
    let mut kept = Vec::new();
    match parts {
        Parts::Seamed => {
            // Each part its own copy of a vertex it shares: meshoptimizer then keeps the seams
            // between parts, sliding their vertices along them, and each copy names its part.
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
            let split_positions: Vec<[f32; 3]> =
                source.iter().map(|&v| positions[v as usize]).collect();
            let simplified = simplify_decoder(&split, &split_positions, 0, error, options, None);
            for t in simplified.as_chunks::<3>().0 {
                triangles.push(t.map(|copy| source[copy as usize]));
                kept.push(part_of[t[0] as usize]);
            }
        }
        Parts::Faced => {
            let indices: Vec<u32> = mesh.triangles.iter().flatten().copied().collect();
            let simplified = simplify_decoder(&indices, &positions, 0, error, options, None);
            for t in simplified.as_chunks::<3>().0 {
                triangles.push(*t);
                kept.push(facing(t.map(|i| positions[i as usize])));
            }
        }
    }
    compact(&mesh.xyz, triangles, kept)
}

/// The part of a triangle by its facing: a roof facing up, a ground facing down, else a wall.
fn facing([first, second, third]: [[f32; 3]; 3]) -> Part {
    // The grid's y points south, so (x, -y, z) is right-handed.
    let [first, second, third] = [first, second, third].map(|[x, y, z]| [x, -y, z].map(f64::from));
    let [along, across] = [second, third].map(|p| [0, 1, 2].map(|i| p[i] - first[i]));
    let normal = [
        along[1] * across[2] - along[2] * across[1],
        along[2] * across[0] - along[0] * across[2],
        along[0] * across[1] - along[1] * across[0],
    ];
    let length = normal.iter().map(|n| n * n).sum::<f64>().sqrt();
    if normal[2] > ROOF_MIN_UP * length {
        Part::Roof
    } else if normal[2] < -ROOF_MIN_UP * length {
        Part::Ground
    } else {
        Part::Wall
    }
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
        let simplified = simplify(&roof, [1.0; 3], 0.05, Parts::Seamed).unwrap();
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
        let simplified = simplify(&mesh, [1.0; 3], 0.05, Parts::Seamed).unwrap();
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
    fn a_piece_smaller_than_the_error_goes_and_a_whole_mesh_is_faced() {
        // A square of 10 m in cm, and a sliver of 1 cm far from it.
        let mut mesh = grid(1, Part::Wall);
        for p in &mut mesh.xyz {
            *p = p.map(|v| v * 1000);
        }
        let n = u32::try_from(mesh.xyz.len()).unwrap();
        mesh.xyz
            .extend([[5000, 5000, 0], [5001, 5000, 0], [5000, 5001, 0]]);
        mesh.triangles.push([n, n + 2, n + 1]);
        mesh.parts.push(Part::Wall);
        let simplified = simplify(&mesh, [0.01; 3], 0.05, Parts::Faced).unwrap();
        assert_eq!(simplified.triangles.len(), 2);
        assert_eq!(simplified.parts, [Part::Roof, Part::Roof]);
    }
}
