//! The blocks of a tile's buildings merged where they touch: their footprints are rasterised in
//! cells of a pixel, the cells at least half covered are joined with their neighbours of the same
//! height band, and each group of cells is raised to its mean top. The outline between two groups,
//! or a group and nothing, is simplified to within a cell once for both sides, so that they still
//! meet, and is walled only where one side stands a pixel or more above the other.

use std::collections::{BTreeSet, HashMap};

use mlt_core::ZStep;

use super::parser::{Lods, Part};
use super::{Mesh, Surface};

/// The metres of a height band: only the cells of one band are joined.
const BAND_METRES: f64 = 10.0;
/// The samples along each side of a cell, of which at least half must lie in a footprint for the
/// cell to be covered.
const SAMPLES: usize = 2;
/// How far, in cells, a simplified outline may stray from the edges of the cells it runs along.
const TOLERANCE_CELLS: f64 = 1.0;
/// The label of a cell in no group.
const NONE: usize = usize::MAX;
/// The most cells a raster holds, 2048 squared: four tiles of pixels across. Each cell costs
/// about 24 bytes.
const MAX_CELLS: usize = 1 << 22;
/// The steps from a corner of the cells to the next, east, south, west and north.
const STEPS: [[i64; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];

/// A building on a tile's grid: the triangles of the outline of its ground or, without one, of
/// its bounding box, and its top above the ground in z units.
pub(super) struct Footprint {
    triangles: Vec<[[i32; 2]; 3]>,
    top: i32,
    base: i32,
}

impl Footprint {
    /// The footprint of `surfaces` at `points` (in absolute source z units), or `None` if it
    /// has no area or no positive height above its base.
    pub(super) fn new(surfaces: &[&Surface], points: &[Vec<[i32; 3]>]) -> Option<Self> {
        let top = points.iter().flatten().map(|p| p[2]).max()?;
        let base = surfaces
            .iter()
            .zip(points)
            .filter(|(s, _)| s.part == Part::Ground)
            .flat_map(|(_, p)| p.iter().map(|v| v[2]))
            .min()
            .or_else(|| points.iter().flatten().map(|p| p[2]).min())?;
        let mut triangles: Vec<[[i32; 2]; 3]> = surfaces
            .iter()
            .zip(points)
            .filter(|(s, _)| s.part == Part::Ground)
            .flat_map(|(s, p)| {
                s.triangles
                    .iter()
                    .map(|t| t.map(|i| [p[i as usize][0], p[i as usize][1]]))
            })
            .filter(|&t| area2(t) != 0)
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
                .filter(|&t| area2(t) != 0)
                .collect();
        }
        (top > base && !triangles.is_empty()).then_some(Self {
            triangles,
            top,
            base,
        })
    }
}

/// Twice the signed area of a triangle on the grid, in `i128` so wide coordinates never overflow.
fn area2([a, b, c]: [[i32; 2]; 3]) -> i128 {
    let [a, b, c] = [a, b, c].map(|p| p.map(i128::from));
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// The block of some of the footprints merged: their indices, ascending, and its mesh.
pub(super) struct Merged {
    pub(super) members: Vec<usize>,
    pub(super) mesh: Mesh,
}

/// The blocks of `footprints` in cells of `cell` grid units, each group of covered cells of one
/// height band raised from 0 to its mean top, with walls where it stands at least `min_wall` z
/// units above its neighbours or the ground. A block of more corners than a `u32` indexes is
/// left out.
///
/// Footprints spanning more than [`MAX_CELLS`] together are merged each on its own, and one
/// spanning more on its own is left out.
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
            if let Some(of) = sides.get_mut(side) {
                of.push((c, reversed));
            }
        }
        let (high, low, out) = match (groups.get(left), groups.get(right)) {
            (Some(l), Some(r)) => {
                if l.top > r.top {
                    (left, right, 1)
                } else {
                    (right, left, -1)
                }
            }
            (Some(_), None) => (left, right, 1),
            (None, Some(_)) => (right, left, -1),
            (None, None) => continue,
        };
        let hi = groups[high].top;
        // Against another group, wall down to the lower roof; against empty cells, wall down to
        // this group's lowest footprint base so hillside walls never float above sloped terrain.
        let lo = groups
            .get(low)
            .map_or(groups[high].min_base, |g| g.top.min(hi));
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
            // Shift `z` so `z = 0` is the group's ground datum (`datum_base`), keeping
            // downhill foundations negative (`z < 0`) per `Heights Above the Ground`.
            for p in &mut mesh.xyz {
                p[2] = p[2].checked_sub(group.datum_base)?;
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
    datum_base: i32,
}

/// The cells of a tile's footprints, by row from the north-west one at `origin`.
struct Raster {
    origin: [i64; 2],
    width: usize,
    height: usize,
    cell: f64,
    covered: Vec<bool>,
    /// The highest absolute top of the footprints a covered cell meets, and then its group's mean top.
    top: Vec<i32>,
    /// The lowest base of the footprints a covered cell meets.
    base: Vec<i32>,
    /// The footprint of that top.
    owner: Vec<usize>,
    /// The group of each covered cell, [`NONE`] for the others.
    group: Vec<usize>,
}

impl Raster {
    /// The raster of `footprints`, or `None` without a footprint or if they span more than
    /// [`MAX_CELLS`].
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
            owner: vec![usize::MAX; width * height],
            group: vec![NONE; width * height],
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
                        if footprint.top > raster.top[c] {
                            raster.top[c] = footprint.top;
                            raster.owner[c] = f;
                        }
                        raster.base[c] = raster.base[c].min(footprint.base);
                    }
                }
            }
        }
        for c in 0..width * height {
            let (cx, cy) = ((c % width) * SAMPLES, (c / width) * SAMPLES);
            let hits = (0..SAMPLES * SAMPLES)
                .filter(|s| inside[(cy + s / SAMPLES) * width * SAMPLES + cx + s % SAMPLES])
                .count();
            raster.covered[c] = 2 * hits >= SAMPLES * SAMPLES && raster.owner[c] != usize::MAX;
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

    /// Joins the covered cells with their neighbours whose absolute top and base stay within the
    /// same `band` of z units, sets each cell's top to the mean of its group's, and returns each
    /// group with its top, lowest base (for foundations) and centre-cell base datum (`z = 0`).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a mean of i32 tops fits an i32"
    )]
    fn group(&mut self, band: i32) -> Vec<Group> {
        let mut groups = Vec::new();
        for start in 0..self.top.len() {
            if !self.covered[start] || self.group[start] != NONE {
                continue;
            }
            let g = groups.len();
            let top_band = self.top[start].div_euclid(band);
            let base_band = self.base[start].div_euclid(band);
            let mut cells = vec![start];
            self.group[start] = g;
            let mut next = 0;
            while let Some(&c) = cells.get(next) {
                next += 1;
                for n in self.beside(c).into_iter().flatten() {
                    if self.covered[n]
                        && self.group[n] == NONE
                        && self.top[n].div_euclid(band) == top_band
                        && self.base[n].div_euclid(band) == base_band
                    {
                        self.group[n] = g;
                        cells.push(n);
                    }
                }
            }
            let sum: i64 = cells.iter().map(|&c| i64::from(self.top[c])).sum();
            let len = i64::try_from(cells.len()).unwrap_or(i64::MAX);
            let mean = ((sum + len / 2) / len) as i32;
            let min_base = cells.iter().map(|&c| self.base[c]).min().unwrap_or(0);
            // Pick the covered cell nearest the group's (x, y) bounding-box midpoint as `z = 0`.
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
            let datum_base = cells
                .iter()
                .min_by_key(|&&c| {
                    let (cx, cy) = (c % self.width, c / self.width);
                    cx.abs_diff(mx).pow(2) + cy.abs_diff(my).pow(2)
                })
                .map_or(min_base, |&c| self.base[c]);
            let members: BTreeSet<usize> = cells.iter().map(|&c| self.owner[c]).collect();
            for c in cells {
                self.top[c] = mean;
            }
            groups.push(Group {
                members: members.into_iter().collect(),
                top: mean,
                min_base,
                datum_base,
            });
        }
        groups
    }

    /// The group of the cell `x` across and `y` down, [`NONE`] outside the raster.
    fn label(&self, [x, y]: [i64; 2]) -> usize {
        if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
            && x < self.width
            && y < self.height
        {
            self.group[y * self.width + x]
        } else {
            NONE
        }
    }

    /// The groups of the cells to the left and the right of the edge from the corner `at`, the
    /// north-west one of the cell `at`, along `step`.
    fn sides(&self, at: [i64; 2], step: [i64; 2]) -> [usize; 2] {
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

    /// The outlines between the groups: the runs of edges between the same two groups from one
    /// junction to the next, then the loops without one.
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
        let mut area2 = 0;
        loop {
            seen[self.edge_index(at, step)] = true;
            let next = [at[0] + step[0], at[1] + step[1]];
            area2 += at[0] * next[1] - next[0] * at[1];
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
            points: simplify(&points),
            first,
            last: step,
            area2,
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
            if area2(flat) != 0 {
                roof.push(facing(flat.map(|[x, y]| [x, y, top]), [0, 0, 1]));
            }
        }
    }
}

/// A run of edges between two groups from one junction to the next, or around a loop.
struct Chain {
    /// The groups to its left and right, [`NONE`] for no group.
    sides: [usize; 2],
    /// Its corners, simplified, the first and the last kept.
    points: Vec<[i64; 2]>,
    /// The steps of its first and last edge.
    first: [i64; 2],
    last: [i64; 2],
    /// Its part of twice the signed area of a ring it runs along, before simplifying.
    area2: i64,
}

/// An outer ring of corners and the rings of the holes in it.
type Polygon = (Vec<[i64; 2]>, Vec<Vec<[i64; 2]>>);

/// The rings around a group, of the corners of the `chains` that `sides` names, each reversed
/// where the group lies to its right: each outer ring with the holes in it.
///
/// Where a group touches itself at a corner, a ring takes the leftmost turn, so that it keeps to
/// the cells of one side of the corner.
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
        let (mut ring, mut area2, mut at) = (Vec::new(), 0, start);
        loop {
            let (c, reversed) = sides[at];
            let chain = &chains[c];
            let corners = &chain.points[..chain.points.len() - 1];
            if reversed {
                ring.extend(chain.points[1..].iter().rev());
                area2 -= chain.area2;
            } else {
                ring.extend(corners);
                area2 += chain.area2;
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
        if area2 > 0 {
            outers.push((area2, ring));
        } else {
            holes.push(ring);
        }
    }
    // A group's cells are joined across edges, so all but slivers lie within one outer ring.
    outers.sort_by_key(|&(area2, _)| std::cmp::Reverse(area2));
    let mut holes = Some(holes);
    outers
        .into_iter()
        .map(|(_, outer)| (outer, holes.take().unwrap_or_default()))
        .collect()
}

/// The corners of `points` that keep the others within [`TOLERANCE_CELLS`] of the lines between
/// them, the first and the last kept. A loop also keeps the corner farthest from its start and,
/// if need be, the one farthest from both, so that it keeps an area.
fn simplify(points: &[[i64; 2]]) -> Vec<[i64; 2]> {
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    if points[0] == points[last] && last > 1 {
        let far = (1..last)
            .max_by(|&a, &b| {
                let off = |i: usize| off2(points[i], points[0], points[0]);
                off(a).total_cmp(&off(b))
            })
            .unwrap_or(1);
        keep[far] = true;
        douglas_peucker(points, 0, far, &mut keep);
        douglas_peucker(points, far, last, &mut keep);
        if keep.iter().filter(|&&k| k).count() < 4 {
            let off = |i: usize| off2(points[i], points[0], points[far]);
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

/// Marks in `keep` the corners between `from` and `to` that the Douglas-Peucker algorithm keeps
/// within [`TOLERANCE_CELLS`].
fn douglas_peucker(points: &[[i64; 2]], from: usize, to: usize, keep: &mut [bool]) {
    let mut spans = vec![(from, to)];
    while let Some((a, b)) = spans.pop() {
        let farthest = (a + 1..b)
            .map(|i| (i, off2(points[i], points[a], points[b])))
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
fn off2(p: [i64; 2], a: [i64; 2], b: [i64; 2]) -> f64 {
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

/// `triangle`, or the reverse of it, so that it winds counter-clockwise in (x, -y, z) seen from
/// `out`, a direction on the grid.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A footprint of the rectangle from `nw` to `se` with its `top`.
    fn rectangle(nw: [i32; 2], se: [i32; 2], top: i32) -> Footprint {
        let [ne, sw] = [[se[0], nw[1]], [nw[0], se[1]]];
        Footprint {
            triangles: vec![[nw, ne, se], [nw, se, sw]],
            top,
            base: 0,
        }
    }

    /// A `ZStep` of 1 cm, so that a band is 1000 z units.
    fn centimetres() -> ZStep {
        ZStep::new(-2).unwrap()
    }

    /// The lowest wall the tests keep, in z units.
    const MIN_WALL: i32 = 200;

    /// The wall and roof triangles of `block`, as corners.
    fn parts(block: &Merged) -> [Vec<[[i32; 3]; 3]>; 2] {
        let mesh = &block.mesh;
        let corners = |part| {
            mesh.triangles
                .iter()
                .zip(&mesh.parts)
                .filter(|&(_, p)| *p == part)
                .map(|(t, _)| t.map(|i| mesh.xyz[i as usize]))
                .collect()
        };
        [corners(Part::Wall), corners(Part::Roof)]
    }

    /// The corners of `triangles` that all lie at x = `x`, sorted and without repeats.
    fn at_x(triangles: &[[[i32; 3]; 3]], x: i32) -> Vec<[i32; 3]> {
        let mut corners: Vec<[i32; 3]> = triangles
            .iter()
            .filter(|t| t.iter().all(|p| p[0] == x))
            .flatten()
            .copied()
            .collect();
        corners.sort_unstable();
        corners.dedup();
        corners
    }

    #[test]
    fn touching_footprints_of_a_band_become_one_box_at_their_mean_top() {
        // Two houses of 6 and 8 m side by side, 4 by 2 cells of 10 units each.
        let houses = [
            rectangle([0, 0], [20, 20], 600),
            rectangle([20, 0], [40, 20], 800),
        ];
        let [block] = &merge(&houses, 10.0, centimetres(), MIN_WALL)[..] else {
            panic!("one block");
        };
        assert_eq!(block.members, [0, 1]);
        let mut xyz = block.mesh.xyz.clone();
        xyz.sort_unstable();
        assert_eq!(
            xyz,
            [
                [0, 0, 0],
                [0, 0, 700],
                [0, 20, 0],
                [0, 20, 700],
                [40, 0, 0],
                [40, 0, 700],
                [40, 20, 0],
                [40, 20, 700]
            ]
        );
        let [walls, roof] = parts(block);
        assert_eq!([walls.len(), roof.len()], [8, 2]);
    }

    #[test]
    fn footprints_too_far_apart_for_one_raster_are_merged_each_on_its_own() {
        // 3002 by 3002 cells together, more than MAX_CELLS.
        let houses = [
            rectangle([0, 0], [20, 20], 600),
            rectangle([30_000, 30_000], [30_020, 30_020], 800),
        ];
        let blocks = merge(&houses, 10.0, centimetres(), MIN_WALL);
        let members: Vec<&[usize]> = blocks.iter().map(|b| &b.members[..]).collect();
        assert_eq!(members, [[0], [1]]);
    }

    #[test]
    fn a_taller_band_stays_apart_and_only_the_part_above_its_neighbour_is_walled() {
        let buildings = [
            rectangle([0, 0], [20, 20], 600),
            rectangle([20, 0], [40, 20], 2500),
        ];
        let blocks = merge(&buildings, 10.0, centimetres(), MIN_WALL);
        let members: Vec<&[usize]> = blocks.iter().map(|b| &b.members[..]).collect();
        assert_eq!(members, [[0], [1]]);
        // The low block has no wall towards the tall one, which has one from 6 to 25 m towards it.
        let [low, tall] = [&blocks[0], &blocks[1]].map(|b| parts(b)[0].clone());
        assert_eq!(at_x(&low, 20), [] as [[i32; 3]; 0]);
        assert_eq!(
            at_x(&tall, 20),
            [[20, 0, 600], [20, 0, 2500], [20, 20, 600], [20, 20, 2500]]
        );
    }

    #[test]
    fn walls_lower_than_the_least_are_left_out() {
        // Neighbours of 9.5 and 10.5 m in two bands, and a shed of 1.5 m apart.
        let buildings = [
            rectangle([0, 0], [20, 20], 950),
            rectangle([20, 0], [40, 20], 1050),
            rectangle([60, 0], [80, 20], 150),
        ];
        let blocks = merge(&buildings, 10.0, centimetres(), MIN_WALL);
        let [low, high, shed] = [0, 1, 2].map(|b| parts(&blocks[b]));
        assert_eq!(at_x(&low[0], 20), [] as [[i32; 3]; 0]);
        assert_eq!(at_x(&high[0], 20), [] as [[i32; 3]; 0]);
        assert_eq!(at_x(&high[0], 40).len(), 4);
        assert_eq!([shed[0].len(), shed[1].len()], [0, 2]);
    }

    #[test]
    fn a_staircase_of_cells_is_simplified_to_its_slope() {
        // A triangle 8 cells along each side, rasterised as a staircase of 8 steps.
        let wedge = [Footprint {
            triangles: vec![[[0, 0], [80, 0], [0, 80]]],
            top: 500,
            base: 0,
        }];
        let [block] = &merge(&wedge, 10.0, centimetres(), MIN_WALL)[..] else {
            panic!("one block");
        };
        let [walls, roof] = parts(block);
        assert!(roof.len() <= 3, "{roof:?}");
        assert!(walls.len() <= 10, "{walls:?}");
        // Every corner lies within a cell of the edges of the cells.
        for [x, y, _] in block.mesh.xyz.iter().copied() {
            assert!(x >= 0 && y >= 0 && x + y <= 80 + 10, "{x} {y}");
        }
    }

    #[test]
    fn a_courtyard_is_a_hole_in_the_roof_walled_inside() {
        // A square of 6 by 6 cells around one of 2 by 2.
        let wings = [
            rectangle([0, 0], [60, 20], 900),
            rectangle([0, 40], [60, 60], 900),
            rectangle([0, 20], [20, 40], 900),
            rectangle([40, 20], [60, 40], 900),
        ];
        let [block] = &merge(&wings, 10.0, centimetres(), MIN_WALL)[..] else {
            panic!("one block");
        };
        let [walls, roof] = parts(block);
        let area: i128 = roof
            .iter()
            .map(|t| area2(t.map(|[x, y, _]| [x, y])).abs())
            .sum();
        assert_eq!(area, 2 * (60 * 60 - 20 * 20));
        assert_eq!(walls.len(), 16);
        assert_eq!(at_x(&walls, 20).len(), 4);
    }

    #[test]
    fn a_cell_under_half_covered_is_left_out() {
        // A quarter of a cell: one of its four samples.
        let shed = [rectangle([0, 0], [5, 5], 300)];
        assert!(merge(&shed, 10.0, centimetres(), MIN_WALL).is_empty());
        // Half of it: two samples.
        let house = [rectangle([0, 0], [10, 5], 300)];
        assert_eq!(merge(&house, 10.0, centimetres(), MIN_WALL).len(), 1);
    }

    #[test]
    fn hillside_buildings_step_apart_across_bands_and_keep_downhill_foundations() {
        // Two 12 m houses stepping 15 m down a hill: (base 15 m, top 27 m) and (base 0 m, top 12 m).
        let hillside = [
            Footprint {
                triangles: vec![[[0, 0], [20, 0], [20, 20]], [[0, 0], [20, 20], [0, 20]]],
                top: 2700,
                base: 1500,
            },
            Footprint {
                triangles: vec![[[20, 0], [40, 0], [40, 20]], [[20, 0], [40, 20], [20, 20]]],
                top: 1200,
                base: 0,
            },
        ];
        let blocks = merge(&hillside, 10.0, centimetres(), MIN_WALL);
        assert_eq!(
            blocks.len(),
            2,
            "stepped hillside houses must not flatten into one slab"
        );
        for block in &blocks {
            let z_max = block.mesh.xyz.iter().map(|p| p[2]).max().unwrap();
            assert_eq!(z_max, 1200);
        }
    }

    #[test]
    fn every_triangle_faces_out_of_the_block() {
        // An L of 8 by 8 cells less 4 by 4, so that the walls turn in and out.
        let footprints = [
            rectangle([0, 0], [80, 40], 900),
            rectangle([0, 40], [40, 80], 900),
        ];
        let [block] = &merge(&footprints, 10.0, centimetres(), MIN_WALL)[..] else {
            panic!("one block");
        };
        let mesh = &block.mesh;
        assert_eq!(mesh.xyz.len(), 12);
        for (t, part) in mesh.triangles.iter().zip(&mesh.parts) {
            let corners = t.map(|i| mesh.xyz[i as usize].map(i64::from));
            let [first, second, third] = corners.map(|[x, y, z]| [x, -y, z]);
            let [along, across] = [second, third].map(|p| [0, 1, 2].map(|i| p[i] - first[i]));
            let normal = [
                along[1] * across[2] - along[2] * across[1],
                along[2] * across[0] - along[0] * across[2],
                along[0] * across[1] - along[1] * across[0],
            ];
            // Three times the triangle's centre, a step ahead of it, and the inside of the L at
            // three times its size: x 0 to 80, y 0 to -80 and z 0 to 900, less the notch.
            let ahead = [0, 1, 2].map(|i| first[i] + second[i] + third[i] + normal[i].signum());
            let inside = |[x, y, z]: [i64; 3]| {
                (0 < x && x < 240)
                    && (-240 < y && y < 0)
                    && (0 < z && z < 2700)
                    && !(x > 120 && y < -120)
            };
            if *part == Part::Roof {
                assert!(normal[2] > 0, "{t:?}");
            } else {
                assert!(!inside(ahead), "{t:?} faces inwards");
            }
        }
    }
}
