//! A building's mesh on a tile's grid, welded from its surfaces or raised as a block.

use std::collections::{HashMap, HashSet};

use mlt_core::ZStep;

use super::model::{Lods, Part, Surface};
use super::orient::orient_outward;

/// The height above `z = 0` up to which downward ground triangles are dropped below the highest zoom.
const HIDDEN_GROUND_CAP_METRES: f64 = 0.5;
/// How far above the lowest ground surface a ground vertex may lie and still count as the building's ground.
pub const MAX_GROUND_SLOPE_METRES: f64 = 15.0;

/// A building on a tile's grid: its vertices, the triangles between them that have three distinct corners, and the part of each triangle.
#[derive(Debug, PartialEq)]
pub struct Mesh {
    pub xyz: Vec<[i32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub parts: Vec<Part>,
}

impl Mesh {
    /// Welds `surfaces`, `points` holding each one's vertices on the grid, so that surfaces share the vertices they meet at.
    /// Vertices keep their order, and those no triangle names are dropped.
    /// `None` if they are more than a `u32` indexes.
    pub fn weld<'a>(
        surfaces: impl IntoIterator<Item = &'a Surface>,
        points: &[Vec<[i32; 3]>],
    ) -> Option<Self> {
        let mut ids: HashMap<[i32; 3], u32> = HashMap::new();
        let mut welded = Vec::new();
        let mut triangles = Vec::new();
        let mut parts = Vec::new();
        let mut seen = HashSet::new();
        for (surface, points) in surfaces.into_iter().zip(points) {
            let local: Vec<u32> = points
                .iter()
                .map(|&p| {
                    let next = u32::try_from(welded.len()).ok()?;
                    Some(*ids.entry(p).or_insert_with(|| {
                        welded.push(p);
                        next
                    }))
                })
                .collect::<Option<_>>()?;
            for t in &surface.triangles {
                let [a, b, c] = t.map(|i| local[i as usize]);
                if a != b && b != c && c != a && seen.insert((rotated([a, b, c]), surface.part)) {
                    triangles.push([a, b, c]);
                    parts.push(surface.part);
                }
            }
        }
        let mut used = vec![false; welded.len()];
        for &i in triangles.iter().flatten() {
            used[i as usize] = true;
        }
        let mut renumbered = vec![0; welded.len()];
        let mut xyz = Vec::new();
        for (i, p) in welded.into_iter().enumerate() {
            if used[i] {
                renumbered[i] = u32::try_from(xyz.len()).ok()?;
                xyz.push(p);
            }
        }
        for t in &mut triangles {
            *t = t.map(|i| renumbered[i as usize]);
        }
        let mut mesh = Self {
            xyz,
            triangles,
            parts,
        };
        mesh.orient_outward();
        Some(mesh)
    }

    /// Turns each closed shell that faces inward outward.
    pub fn orient_outward(&mut self) {
        orient_outward(&self.xyz, &mut self.triangles);
    }
}

/// Drops the downward ground triangles of `mesh` no higher than [`HIDDEN_GROUND_CAP_METRES`] above `z = 0`.
pub fn drop_ground_caps(mesh: &mut Mesh, z_step: ZStep) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "0.5 m in z_step units of at least 1 mm fits in i32"
    )]
    let cap = (HIDDEN_GROUND_CAP_METRES / z_step.metres()).round() as i32;
    (mesh.triangles, mesh.parts) = mesh
        .triangles
        .iter()
        .zip(&mesh.parts)
        .filter(|&(&t, &part)| {
            part != Part::Ground || t.iter().any(|&v| mesh.xyz[v as usize][2] > cap)
        })
        .map(|(&t, &p)| (t, p))
        .unzip();
}

/// The triangle turned to start at its least vertex, alike for the triangles that wind alike through the same vertices.
fn rotated([a, b, c]: [u32; 3]) -> [u32; 3] {
    if b < a && b < c {
        [b, c, a]
    } else if c < a && c < b {
        [c, a, b]
    } else {
        [a, b, c]
    }
}

/// The block of `surfaces` at `points`: the outline of their ground or, without one, their bounding box, raised from 0 to their top.
/// Its walls and roof face out, and it has no bottom.
/// `None` if it has no height, or if its vertices are more than a `u32` indexes.
pub fn block(surfaces: &[&Surface], points: &[Vec<[i32; 3]>], z_step: ZStep) -> Option<Mesh> {
    let (min, max) =
        points
            .iter()
            .flatten()
            .fold(([i32::MAX; 3], [i32::MIN; 3]), |(min, max), p| {
                (
                    [0, 1, 2].map(|i| min[i].min(p[i])),
                    [0, 1, 2].map(|i| max[i].max(p[i])),
                )
            });
    // Foundations below the ground at the centre reach down to it.
    let base = min[2].min(0);
    if max[2] <= base {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "15 m in z_step units of at least 1 mm fits in i32"
    )]
    let slope_cap = (MAX_GROUND_SLOPE_METRES / z_step.metres()).round() as i32;
    let min_ground_z = surfaces
        .iter()
        .zip(points)
        .filter(|(s, _)| s.part == Part::Ground)
        .flat_map(|(_, p)| p.iter().map(|v| v[2]))
        .min()
        .unwrap_or(0);
    let max_collar_z = min_ground_z.saturating_add(slope_cap);
    let (grounds, flat): (Vec<&Surface>, Vec<Vec<[i32; 3]>>) = surfaces
        .iter()
        .zip(points)
        .filter(|(s, p)| s.part == Part::Ground && p.iter().all(|v| v[2] <= max_collar_z))
        .map(|(&s, p)| (s, p.iter().map(|&[x, y, _]| [x, y, base]).collect()))
        .unzip();
    let ground = Mesh::weld(grounds, &flat)?;
    let mut xyz = ground.xyz;
    let mut triangles = facing_down(&xyz, &ground.triangles);
    if triangles.is_empty() {
        xyz = vec![
            [min[0], min[1], base],
            [max[0], min[1], base],
            [max[0], max[1], base],
            [min[0], max[1], base],
        ];
        triangles = facing_down(&xyz, &[[0, 1, 2], [0, 2, 3]]);
    }
    // Grounds stacked onto each other would count their edges twice, leaving out their walls.
    let mut seen = HashSet::new();
    triangles.retain(|&t| seen.insert(rotated(t)));
    let edges = |&[a, b, c]: &[u32; 3]| [[a, b], [b, c], [c, a]];
    let mut uses: HashMap<[u32; 2], u32> = HashMap::new();
    for [a, b] in triangles.iter().flat_map(edges) {
        *uses.entry([a.min(b), a.max(b)]).or_default() += 1;
    }
    // The roof's vertices follow the ground's, so twice their count must fit.
    let n = u32::try_from(xyz.len())
        .ok()
        .filter(|&n| n <= u32::MAX / 2)?;
    let surface = |part, triangles| Surface {
        part,
        lods: Lods::Both,
        positions: Vec::new(),
        triangles,
    };
    let walls = triangles
        .iter()
        .flat_map(edges)
        .filter(|&[a, b]| uses.get(&[a.min(b), a.max(b)]) == Some(&1))
        .flat_map(|[a, b]| [[b, a, a + n], [b, a + n, b + n]])
        .collect();
    let roof = triangles.iter().map(|&[a, b, c]| [a + n, c + n, b + n]);
    let mut raised = xyz.clone();
    raised.extend(xyz.iter().map(|&[x, y, _]| [x, y, max[2]]));
    Mesh::weld(
        &[
            surface(Part::Wall, walls),
            surface(Part::Roof, roof.collect()),
        ],
        &[raised.clone(), raised],
    )
}

/// The `triangles` between flat `xyz` that have an area, each turned to face down.
fn facing_down(xyz: &[[i32; 3]], triangles: &[[u32; 3]]) -> Vec<[u32; 3]> {
    // The grid's y points south, so a triangle facing down turns from +x towards +y.
    triangles
        .iter()
        .filter_map(|&[a, b, c]| {
            let [pa, pb, pc] = [a, b, c].map(|i| xyz[i as usize].map(i128::from));
            match ((pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0])).signum() {
                1 => Some([a, b, c]),
                -1 => Some([a, c, b]),
                _ => None,
            }
        })
        .collect()
}
