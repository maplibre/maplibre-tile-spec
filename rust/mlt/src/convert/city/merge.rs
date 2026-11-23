//! The blocks of a tile's buildings, merged where they touch.

use std::cmp::Reverse;
use std::collections::HashMap;

use mlt_core::ZStep;

use super::mesh::Mesh;
use super::model::{Lods, Part, Surface};

/// The metres of a height band, only the cells of one band being joined.
const BAND_METRES: f64 = 10.0;
/// The samples along each side of a cell, at least half of which must lie in a footprint for the cell to be covered.
const SAMPLES: usize = 2;
/// How far, in cells, a simplified outline may stray from the edges of the cells it runs along.
const TOLERANCE_CELLS: f64 = 0.75;
/// The most cells a raster holds, 2048 squared.
const MAX_CELLS: usize = 1 << 22;
/// The steps from a corner of the cells to the next, east, south, west and north.
const STEPS: [[i64; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];

/// A building on a tile's grid: the triangles of its grounds and roofs, or else of its bounding box, with its top, base and ground in z units.
pub(super) struct Footprint {
    triangles: Vec<[[i32; 2]; 3]>,
    top: i32,
    base: i32,
    ground: i32,
}

impl Footprint {
    /// The footprint of `surfaces` at `points`, standing on `ground`, or `None` if it has no area or no height.
    pub(super) fn new(
        surfaces: &[&Surface],
        points: &[Vec<[i32; 3]>],
        ground: i32,
    ) -> Option<Self> {
        let top = points.iter().flatten().map(|p| p[2]).max()?;
        let base = points.iter().flatten().map(|p| p[2]).min()?;
        let mut triangles: Vec<[[i32; 2]; 3]> = surfaces
            .iter()
            .zip(points)
            .filter(|(s, _)| matches!(s.part, Part::Ground | Part::Roof))
            .flat_map(|(s, p)| {
                s.triangles
                    .iter()
                    .map(|t| t.map(|i| [p[i as usize][0], p[i as usize][1]]))
            })
            .filter(|&t| doubled_area(t) != 0)
            .collect();
        if triangles.is_empty() {
            let [mut min, mut max] = [[i32::MAX; 2], [i32::MIN; 2]];
            for p in points.iter().flatten() {
                min = [min[0].min(p[0]), min[1].min(p[1])];
                max = [max[0].max(p[0]), max[1].max(p[1])];
            }
            let [nw, ne, se, sw] = [min, [max[0], min[1]], max, [min[0], max[1]]];
            triangles = [[nw, ne, se], [nw, se, sw]]
                .into_iter()
                .filter(|&t| doubled_area(t) != 0)
                .collect();
        }
        (top > base && !triangles.is_empty()).then_some(Self {
            triangles,
            top,
            base,
            ground,
        })
    }
}

/// Twice the signed area of a triangle on the grid.
fn doubled_area([a, b, c]: [[i32; 2]; 3]) -> i128 {
    let [a, b, c] = [a, b, c].map(|p| p.map(i128::from));
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// The block of some of the footprints merged: their indices, ascending, and its mesh.
pub(super) struct Merged {
    pub(super) members: Vec<usize>,
    pub(super) mesh: Mesh,
}

/// The blocks of `footprints` in cells of `cell` grid units, each group of covered cells of one height band raised to its mean top.
/// A block has walls where it stands at least `min_wall` z units above its neighbours or the ground.
/// Footprints spanning more than [`MAX_CELLS`] together are merged each on its own, and one spanning more on its own is left out.
/// A block of more corners than a `u32` indexes is left out.
pub(super) fn merge(
    footprints: &[Footprint],
    cell: f64,
    z_step: ZStep,
    min_wall: i32,
) -> Vec<Merged> {
    if let Some(blocks) = merge_within(footprints, cell, z_step, min_wall) {
        return blocks;
    }
    (0..footprints.len())
        .flat_map(|f| {
            let blocks = merge_within(&footprints[f..=f], cell, z_step, min_wall);
            blocks.into_iter().flatten().map(move |mut block| {
                block.members = vec![f];
                block
            })
        })
        .collect()
}

/// [`merge`], or `None` if the footprints span more than [`MAX_CELLS`].
fn merge_within(
    footprints: &[Footprint],
    cell: f64,
    z_step: ZStep,
    min_wall: i32,
) -> Option<Vec<Merged>> {
    if footprints.is_empty() {
        return Some(Vec::new());
    }
    let mut raster = Raster::new(footprints, cell)?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a band is a few metres in z units of at least a millimetre"
    )]
    let band = ((BAND_METRES / z_step.metres()).round() as i32).max(1);
    let groups = raster.group(band);
    let chains = raster.chains();
    let mut walls = vec![Vec::new(); groups.len()];
    let mut sides = vec![Vec::new(); groups.len()];
    for (c, chain) in chains.iter().enumerate() {
        let [left, right] = chain.sides;
        for (side, reversed) in [(left, false), (right, true)] {
            if let Some(side) = side {
                sides[side].push((c, reversed));
            }
        }
        let (high, low, out) = match (left, right) {
            (Some(l), Some(r)) => {
                if groups[l].top > groups[r].top {
                    (l, right, 1)
                } else {
                    (r, left, -1)
                }
            }
            (Some(l), None) => (l, None, 1),
            (None, Some(r)) => (r, None, -1),
            (None, None) => continue,
        };
        let hi = groups[high].top;
        // A wall runs down to the lower group's top, or else to this group's lowest base.
        let lo = low.map_or(groups[high].min_base, |g| groups[g].top.min(hi));
        if hi.saturating_sub(lo) < min_wall.max(1) {
            continue;
        }
        for pair in chain.points.windows(2) {
            let [[ax, ay], [bx, by]] = [pair[0], pair[1]].map(|p| raster.corner(p));
            if [ax, ay] == [bx, by] {
                continue;
            }
            // The left of a step (x, y) lies towards (-y, x).
            let step = [pair[1][0] - pair[0][0], pair[1][1] - pair[0][1]];
            let out = [out * step[1], -out * step[0], 0];
            let [start, end] = [[ax, ay, lo], [bx, by, lo]];
            let [end_top, start_top] = [[bx, by, hi], [ax, ay, hi]];
            walls[high].extend([
                facing([start, end, end_top], out),
                facing([start, end_top, start_top], out),
            ]);
        }
    }
    let mut earcut = earcut::Earcut::new();
    let mut indices = Vec::new();
    let blocks = groups
        .into_iter()
        .zip(walls.into_iter().zip(sides))
        .filter_map(|(group, (walls, sides))| {
            let mut roof = Vec::new();
            for (outer, holes) in rings(&chains, &sides) {
                raster.roof(
                    &outer,
                    &holes,
                    group.top,
                    &mut earcut,
                    &mut indices,
                    &mut roof,
                );
            }
            let mut mesh = weld(&walls, &roof)?;
            // Basements and downhill foundations stay below the group's ground at `z = 0`.
            for p in &mut mesh.xyz {
                p[2] = p[2].checked_sub(group.ground)?;
            }
            Some(Merged {
                members: group.members,
                mesh,
            })
        })
        .collect();
    Some(blocks)
}

struct Group {
    members: Vec<usize>,
    top: i32,
    min_base: i32,
    ground: i32,
}

/// The cells of a tile's footprints, by row from the north-west one at `origin`.
struct Raster {
    origin: [i64; 2],
    width: usize,
    height: usize,
    cell: f64,
    covered: Vec<bool>,
    /// The highest top of the footprints a covered cell meets, and then its group's mean top.
    top: Vec<i32>,
    /// The lowest base of the footprints a covered cell meets.
    base: Vec<i32>,
    /// The lowest ground of the footprints a covered cell meets.
    ground: Vec<i32>,
    /// Each cell and a footprint that meets it.
    touched: Vec<(usize, usize)>,
    /// The group of each covered cell.
    group: Vec<Option<usize>>,
}

impl Raster {
    /// The raster of `footprints`, or `None` without a footprint or if they span more than [`MAX_CELLS`].
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the cell indices of a tile's grid, of at most a few thousand cells across"
    )]
    fn new(footprints: &[Footprint], cell: f64) -> Option<Self> {
        let corners = || footprints.iter().flat_map(|f| f.triangles.iter().flatten());
        let index = |v: i32| (f64::from(v) / cell).floor() as i64;
        let min = [0, 1].map(|i| corners().map(|p| index(p[i])).min());
        let max = [0, 1].map(|i| corners().map(|p| index(p[i])).max());
        let ([Some(x0), Some(y0)], [Some(x1), Some(y1)]) = (min, max) else {
            return None;
        };
        let cells = |lo: i64, hi: i64| usize::try_from(hi - lo + 1).ok();
        let (width, height) = (cells(x0, x1)?, cells(y0, y1)?);
        if width.checked_mul(height)? > MAX_CELLS {
            return None;
        }
        let mut raster = Self {
            origin: [x0, y0],
            width,
            height,
            cell,
            covered: vec![false; width * height],
            top: vec![i32::MIN; width * height],
            base: vec![i32::MAX; width * height],
            ground: vec![i32::MAX; width * height],
            touched: Vec::new(),
            group: vec![None; width * height],
        };
        // The samples of each cell inside a footprint, at the centres of a SAMPLES-square grid.
        let n = SAMPLES as f64;
        let mut inside = vec![false; width * SAMPLES * height * SAMPLES];
        for (f, footprint) in footprints.iter().enumerate() {
            for &triangle in &footprint.triangles {
                let span = |i: usize, cells: usize, origin: i64| {
                    let at = |v: i32| (f64::from(v) / cell - origin as f64) * n - 0.5;
                    let lo = triangle
                        .iter()
                        .map(|p| at(p[i]))
                        .fold(f64::INFINITY, f64::min);
                    let hi = triangle
                        .iter()
                        .map(|p| at(p[i]))
                        .fold(f64::NEG_INFINITY, f64::max);
                    let last = cells * SAMPLES - 1;
                    (lo.ceil().max(0.0) as usize)..=(hi.floor().max(0.0) as usize).min(last)
                };
                for sy in span(1, height, y0) {
                    for sx in span(0, width, x0) {
                        let at =
                            |s: usize, origin: i64| (origin as f64 + (s as f64 + 0.5) / n) * cell;
                        if !contains(triangle, [at(sx, x0), at(sy, y0)]) {
                            continue;
                        }
                        inside[sy * width * SAMPLES + sx] = true;
                        let c = (sy / SAMPLES) * width + sx / SAMPLES;
                        raster.top[c] = raster.top[c].max(footprint.top);
                        raster.base[c] = raster.base[c].min(footprint.base);
                        raster.ground[c] = raster.ground[c].min(footprint.ground);
                        if raster.touched.last() != Some(&(c, f)) {
                            raster.touched.push((c, f));
                        }
                    }
                }
            }
        }
        for c in 0..width * height {
            let (cx, cy) = ((c % width) * SAMPLES, (c / width) * SAMPLES);
            let hits = (0..SAMPLES * SAMPLES)
                .filter(|s| inside[(cy + s / SAMPLES) * width * SAMPLES + cx + s % SAMPLES])
                .count();
            raster.covered[c] = 2 * hits >= SAMPLES * SAMPLES && raster.top[c] > i32::MIN;
        }
        Some(raster)
    }

    /// The cells beside cell `c`, north, south, west and east, if within the raster.
    fn beside(&self, c: usize) -> [Option<usize>; 4] {
        let (x, y) = (c % self.width, c / self.width);
        [
            (y > 0).then(|| c - self.width),
            (y + 1 < self.height).then(|| c + self.width),
            (x > 0).then(|| c - 1),
            (x + 1 < self.width).then(|| c + 1),
        ]
    }

    /// Joins the covered cells with their neighbours whose top and base lie in the same `band` of z units, and sets each cell's top to its group's mean.
    /// Returns each group with the footprints of which it holds the most cells, its top, its lowest base, and the ground of its middle cell.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a mean of i32 tops fits an i32"
    )]
    fn group(&mut self, band: i32) -> Vec<Group> {
        let mut groups = Vec::new();
        for start in 0..self.top.len() {
            if !self.covered[start] || self.group[start].is_some() {
                continue;
            }
            let g = groups.len();
            let top_band = self.top[start].div_euclid(band);
            let base_band = self.base[start].div_euclid(band);
            let mut cells = vec![start];
            self.group[start] = Some(g);
            let mut next = 0;
            while let Some(&c) = cells.get(next) {
                next += 1;
                for n in self.beside(c).into_iter().flatten() {
                    if self.covered[n]
                        && self.group[n].is_none()
                        && self.top[n].div_euclid(band) == top_band
                        && self.base[n].div_euclid(band) == base_band
                    {
                        self.group[n] = Some(g);
                        cells.push(n);
                    }
                }
            }
            let sum: i64 = cells.iter().map(|&c| i64::from(self.top[c])).sum();
            let len = i64::try_from(cells.len()).unwrap_or(i64::MAX);
            let mean = ((sum + len / 2) / len) as i32;
            let min_base = cells.iter().map(|&c| self.base[c]).min().unwrap_or(0);
            let (mut x0, mut x1) = (usize::MAX, 0);
            let (mut y0, mut y1) = (usize::MAX, 0);
            for &c in &cells {
                let (cx, cy) = (c % self.width, c / self.width);
                x0 = x0.min(cx);
                x1 = x1.max(cx);
                y0 = y0.min(cy);
                y1 = y1.max(cy);
            }
            let (mx, my) = (usize::midpoint(x0, x1), usize::midpoint(y0, y1));
            let ground = cells
                .iter()
                .min_by_key(|&&c| {
                    let (cx, cy) = (c % self.width, c / self.width);
                    cx.abs_diff(mx).pow(2) + cy.abs_diff(my).pow(2)
                })
                .map_or(min_base, |&c| self.ground[c]);
            for c in cells {
                self.top[c] = mean;
            }
            groups.push(Group {
                members: Vec::new(),
                top: mean,
                min_base,
                ground,
            });
        }
        let mut touched: Vec<(usize, usize)> = self.touched.iter().map(|&(c, f)| (f, c)).collect();
        touched.sort_unstable();
        touched.dedup();
        let mut touched: Vec<(usize, usize)> = touched
            .into_iter()
            .filter_map(|(f, c)| Some((f, self.group[c]?)))
            .collect();
        touched.sort_unstable();
        for cells in touched.chunk_by(|a, b| a.0 == b.0) {
            let most = cells
                .chunk_by(|a, b| a.1 == b.1)
                .max_by_key(|run| (run.len(), Reverse(run[0].1)));
            if let Some(&[(f, g), ..]) = most {
                groups[g].members.push(f);
            }
        }
        groups
    }

    /// The group of the cell `x` across and `y` down.
    fn label(&self, [x, y]: [i64; 2]) -> Option<usize> {
        if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
            && x < self.width
            && y < self.height
        {
            self.group[y * self.width + x]
        } else {
            None
        }
    }

    /// The groups of the cells to the left and the right of the edge from the corner `at`, the north-west one of the cell `at`, along `step`.
    fn sides(&self, at: [i64; 2], step: [i64; 2]) -> [Option<usize>; 2] {
        let left = [-step[1], step[0]];
        [1, -1]
            .map(|s| self.label([0, 1].map(|i| (2 * at[i] + step[i] + s * left[i]).div_euclid(2))))
    }

    /// Whether the edge from the corner `at` along `step` lies between two groups.
    fn edge(&self, at: [i64; 2], step: [i64; 2]) -> bool {
        let [left, right] = self.sides(at, step);
        left != right
    }

    /// Whether the corner `at` is where other than two edges between groups meet.
    fn junction(&self, at: [i64; 2]) -> bool {
        STEPS.into_iter().filter(|&s| self.edge(at, s)).count() != 2
    }

    /// The index of the edge from the corner `at` along `step`, the same either way along it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the corners of a raster's cells, at most a few thousand across"
    )]
    fn edge_index(&self, at: [i64; 2], step: [i64; 2]) -> usize {
        let from = if step[0] + step[1] < 0 {
            [at[0] + step[0], at[1] + step[1]]
        } else {
            at
        };
        2 * (from[1] as usize * (self.width + 1) + from[0] as usize) + usize::from(step[1] != 0)
    }

    /// The outlines between the groups: the runs of edges between the same two groups from one junction to the next, then the loops without one.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "the corners of a raster's cells, at most a few thousand across"
    )]
    fn chains(&self) -> Vec<Chain> {
        let corners = (self.width + 1) * (self.height + 1);
        let mut seen = vec![false; 2 * corners];
        let mut chains = Vec::new();
        for loops in [false, true] {
            for c in 0..corners {
                let at = [c % (self.width + 1), c / (self.width + 1)].map(|v| v as i64);
                if self.junction(at) == loops {
                    continue;
                }
                for step in STEPS {
                    if self.edge(at, step) && !seen[self.edge_index(at, step)] {
                        chains.push(self.trace(at, step, &mut seen));
                    }
                }
            }
        }
        chains
    }

    /// The chain from the corner `start` along `first`, its edges marked in `seen`.
    fn trace(&self, start: [i64; 2], first: [i64; 2], seen: &mut [bool]) -> Chain {
        let (mut at, mut step) = (start, first);
        let mut points = vec![start];
        let mut doubled_area = 0;
        loop {
            seen[self.edge_index(at, step)] = true;
            let next = [at[0] + step[0], at[1] + step[1]];
            doubled_area += at[0] * next[1] - next[0] * at[1];
            points.push(next);
            if next == start || self.junction(next) {
                break;
            }
            let back = [-step[0], -step[1]];
            at = next;
            step = STEPS
                .into_iter()
                .find(|&s| s != back && self.edge(at, s))
                .unwrap_or(back);
        }
        Chain {
            sides: self.sides(start, first),
            points: simplify_outline(&points),
            first,
            last: step,
            doubled_area,
        }
    }

    /// The grid position of the corner `at`, the north-west one of the cell `at`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the corners of a tile's cells are near its grid, which fits an i32"
    )]
    fn corner(&self, at: [i64; 2]) -> [i32; 2] {
        [0, 1].map(|i| ((self.origin[i] + at[i]) as f64 * self.cell).round() as i32)
    }

    /// Adds to `roof` the polygon of the corners `outer` less `holes`, cut into triangles at `top`.
    fn roof(
        &self,
        outer: &[[i64; 2]],
        holes: &[Vec<[i64; 2]>],
        top: i32,
        earcut: &mut earcut::Earcut<f64>,
        indices: &mut Vec<u32>,
        roof: &mut Vec<[[i32; 3]; 3]>,
    ) {
        let rings = std::iter::once(outer).chain(holes.iter().map(Vec::as_slice));
        let corners: Vec<[i32; 2]> = rings.flatten().map(|&p| self.corner(p)).collect();
        let starts: Vec<u32> = holes
            .iter()
            .scan(outer.len(), |start, hole| {
                let at = *start;
                *start += hole.len();
                u32::try_from(at).ok()
            })
            .collect();
        if starts.len() != holes.len() {
            return;
        }
        let [x0, y0] = corners[0];
        let flat = corners
            .iter()
            .map(|p| [f64::from(p[0] - x0), f64::from(p[1] - y0)]);
        earcut.earcut(flat, &starts, indices);
        for t in indices.as_chunks::<3>().0 {
            let flat = t.map(|i| corners[i as usize]);
            if doubled_area(flat) != 0 {
                roof.push(facing(flat.map(|[x, y]| [x, y, top]), [0, 0, 1]));
            }
        }
    }
}

/// A run of edges between two groups from one junction to the next, or around a loop.
struct Chain {
    /// The groups to its left and right.
    sides: [Option<usize>; 2],
    /// Its corners, simplified, the first and the last kept.
    points: Vec<[i64; 2]>,
    /// The steps of its first and last edge.
    first: [i64; 2],
    last: [i64; 2],
    /// Its part of twice the signed area of a ring it runs along, before simplifying.
    doubled_area: i64,
}

/// An outer ring of corners and the rings of the holes in it.
type Polygon = (Vec<[i64; 2]>, Vec<Vec<[i64; 2]>>);

/// The rings around a group of the corners of the `chains` that `sides` names, each reversed where the group lies to its right, as outer rings with their holes.
/// Where a group touches itself at a corner, a ring takes the leftmost turn.
fn rings(chains: &[Chain], sides: &[(usize, bool)]) -> Vec<Polygon> {
    let ends = |&(c, reversed): &(usize, bool)| {
        let chain = &chains[c];
        let [first, last] = [chain.points[0], chain.points[chain.points.len() - 1]];
        if reversed {
            let back = |s: [i64; 2]| [-s[0], -s[1]];
            (last, first, back(chain.last), back(chain.first))
        } else {
            (first, last, chain.first, chain.last)
        }
    };
    let mut starting: HashMap<[i64; 2], Vec<usize>> = HashMap::new();
    for (d, side) in sides.iter().enumerate() {
        starting.entry(ends(side).0).or_default().push(d);
    }
    let turn = |from: [i64; 2], to: [i64; 2]| {
        let cross = from[0] * to[1] - from[1] * to[0];
        let dot = from[0] * to[0] + from[1] * to[1];
        if cross != 0 {
            2 * cross.signum()
        } else if dot > 0 {
            1
        } else {
            -3
        }
    };
    let mut used = vec![false; sides.len()];
    let (mut outers, mut holes) = (Vec::new(), Vec::new());
    for start in 0..sides.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (first_corner, _, first_step, _) = ends(&sides[start]);
        let (mut ring, mut doubled_area, mut at) = (Vec::new(), 0, start);
        loop {
            let (c, reversed) = sides[at];
            let chain = &chains[c];
            let corners = &chain.points[..chain.points.len() - 1];
            if reversed {
                ring.extend(chain.points[1..].iter().rev());
                doubled_area -= chain.doubled_area;
            } else {
                ring.extend(corners);
                doubled_area += chain.doubled_area;
            }
            let (_, end, _, step) = ends(&sides[at]);
            let next = starting.get(&end).and_then(|next| {
                next.iter()
                    .copied()
                    .filter(|&d| d == start || !used[d])
                    .max_by_key(|&d| turn(step, ends(&sides[d]).2))
            });
            match next {
                Some(d) if d != start => {
                    used[d] = true;
                    at = d;
                }
                _ => break,
            }
        }
        if ring.len() < 3 {
            continue;
        }
        if doubled_area > 0 {
            outers.push((doubled_area, ring));
        } else {
            let right = [first_step[1], -first_step[0]];
            let cell =
                [0, 1].map(|i| (2 * first_corner[i] + first_step[i] + right[i]).div_euclid(2));
            holes.push((cell, ring));
        }
    }
    // The smallest outer ring around a hole is the one it is in.
    outers.sort_by_key(|&(doubled_area, _)| doubled_area);
    let mut assigned: Vec<Vec<Vec<[i64; 2]>>> = vec![Vec::new(); outers.len()];
    for (cell, hole) in holes {
        if let Some(idx) = outers
            .iter()
            .position(|(_, outer)| cell_in_ring(cell, outer))
        {
            assigned[idx].push(hole);
        }
    }
    outers
        .into_iter()
        .zip(assigned)
        .map(|((_, outer), holes)| (outer, holes))
        .collect()
}

/// Whether the centre of `cell` lies inside `ring`.
fn cell_in_ring(cell: [i64; 2], ring: &[[i64; 2]]) -> bool {
    let [px, py] = cell.map(|c| 2 * i128::from(c) + 1);
    let mut inside = false;
    for i in 0..ring.len() {
        let [x1, y1] = ring[i].map(|v| 2 * i128::from(v));
        let [x2, y2] = ring[(i + 1) % ring.len()].map(|v| 2 * i128::from(v));
        if (y1 > py) != (y2 > py) {
            let crosses_right = if y2 > y1 {
                (px - x1) * (y2 - y1) < (x2 - x1) * (py - y1)
            } else {
                (px - x1) * (y1 - y2) < (x2 - x1) * (y1 - py)
            };
            if crosses_right {
                inside = !inside;
            }
        }
    }
    inside
}

/// The corners of `points` that keep the others within [`TOLERANCE_CELLS`] of the lines between them, the first and the last kept.
/// A loop also keeps the corner farthest from its start and, if need be, the one farthest from both, so that it keeps an area.
fn simplify_outline(points: &[[i64; 2]]) -> Vec<[i64; 2]> {
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    if points[0] == points[last] && last > 1 {
        let far = (1..last)
            .max_by(|&a, &b| {
                let off = |i: usize| squared_distance_to_segment(points[i], points[0], points[0]);
                off(a).total_cmp(&off(b))
            })
            .unwrap_or(1);
        keep[far] = true;
        douglas_peucker(points, 0, far, &mut keep);
        douglas_peucker(points, far, last, &mut keep);
        if keep.iter().filter(|&&k| k).count() < 4 {
            let off = |i: usize| squared_distance_to_segment(points[i], points[0], points[far]);
            if let Some(widest) = (1..last).max_by(|&a, &b| off(a).total_cmp(&off(b))) {
                keep[widest] = true;
            }
        }
    } else {
        douglas_peucker(points, 0, last, &mut keep);
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(&p, k)| k.then_some(p))
        .collect()
}

/// Marks in `keep` the corners between `from` and `to` that the Douglas-Peucker algorithm keeps within [`TOLERANCE_CELLS`].
fn douglas_peucker(points: &[[i64; 2]], from: usize, to: usize, keep: &mut [bool]) {
    let mut spans = vec![(from, to)];
    while let Some((a, b)) = spans.pop() {
        let farthest = (a + 1..b)
            .map(|i| {
                (
                    i,
                    squared_distance_to_segment(points[i], points[a], points[b]),
                )
            })
            .max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, off)) = farthest
            && off > TOLERANCE_CELLS * TOLERANCE_CELLS
        {
            keep[i] = true;
            spans.extend([(a, i), (i, b)]);
        }
    }
}

/// The squared distance of `p` from the segment from `a` to `b`.
#[expect(
    clippy::cast_precision_loss,
    reason = "the corners of a raster's cells, at most a few thousand across"
)]
fn squared_distance_to_segment(p: [i64; 2], a: [i64; 2], b: [i64; 2]) -> f64 {
    let [p, a, b] = [p, a, b].map(|v| v.map(|c| c as f64));
    let (along, to) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let length2 = along[0] * along[0] + along[1] * along[1];
    let t = if length2 > 0.0 {
        ((to[0] * along[0] + to[1] * along[1]) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let off = [to[0] - t * along[0], to[1] - t * along[1]];
    off[0] * off[0] + off[1] * off[1]
}

/// Whether `p` lies in `triangle` or on its edges.
fn contains(triangle: [[i32; 2]; 3], p: [f64; 2]) -> bool {
    let [a, b, c] = triangle.map(|v| v.map(f64::from));
    let side =
        |u: [f64; 2], v: [f64; 2]| (v[0] - u[0]) * (p[1] - u[1]) - (v[1] - u[1]) * (p[0] - u[0]);
    let [d0, d1, d2] = [side(a, b), side(b, c), side(c, a)];
    (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0)
}

/// `triangle`, or the reverse of it, so that it winds counter-clockwise in `(x, -y, z)` seen from `out`, a direction on the grid.
fn facing(triangle: [[i32; 3]; 3], out: [i64; 3]) -> [[i32; 3]; 3] {
    let [first, second, third] =
        triangle.map(|[x, y, z]| [i64::from(x), -i64::from(y), i64::from(z)]);
    let [along, across] = [second, third].map(|p| [0, 1, 2].map(|i| p[i] - first[i]));
    let normal = [
        along[1] * across[2] - along[2] * across[1],
        along[2] * across[0] - along[0] * across[2],
        along[0] * across[1] - along[1] * across[0],
    ];
    let out = [out[0], -out[1], out[2]];
    if (0..3).map(|i| normal[i] * out[i]).sum::<i64>() >= 0 {
        triangle
    } else {
        [triangle[0], triangle[2], triangle[1]]
    }
}

/// One mesh of `walls` and `roof`, triangles of grid positions, their shared corners welded.
/// `None` if their corners are more than a `u32` indexes.
fn weld(walls: &[[[i32; 3]; 3]], roof: &[[[i32; 3]; 3]]) -> Option<Mesh> {
    let surface = |part, triangles: &[[[i32; 3]; 3]]| {
        let corners = u32::try_from(triangles.len().checked_mul(3)?).ok()?;
        let surface = Surface {
            part,
            lods: Lods::Both,
            positions: Vec::new(),
            triangles: (0..corners).step_by(3).map(|c| [c, c + 1, c + 2]).collect(),
        };
        Some((surface, triangles.iter().flatten().copied().collect()))
    };
    let (wall, wall_points) = surface(Part::Wall, walls)?;
    let (roof, roof_points) = surface(Part::Roof, roof)?;
    Mesh::weld(&[wall, roof], &[wall_points, roof_points])
}
