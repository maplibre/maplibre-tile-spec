//! Pushing each feature's geometry into a layer's [`GeometryValues`], wound and optionally vertex-sorted.

use std::borrow::Cow;

use geo_types::{Coord, Geometry};

use super::geotype::{needs_reversal, reversed_position, rings};
use crate::decoder::GeometryValues;
use crate::tile::{VertexMoves, stored_len};

/// Pushes features one after another, winding rings as MVT does.
/// Under vertex sort, each ring, line and `MultiPoint` starts near the vertex stored before it, see [`crate::encoder::EncoderConfig::with_vertex_sort`].
/// When tracking, it records where each stored vertex of a feature came from, for the values held per vertex.
pub(crate) struct GeometryStager<'a> {
    values: &'a mut GeometryValues,
    sort: bool,
    track: bool,
    /// Entry `i` is the position among the feature's stored source vertices of the vertex stored at `i`.
    moved: Vec<usize>,
    /// Whether `moved` differs from the source order.
    permuted: bool,
}

impl<'a> GeometryStager<'a> {
    /// Stages into `values` in source order, without recording vertex moves.
    pub(crate) fn new(values: &'a mut GeometryValues) -> Self {
        Self {
            values,
            sort: false,
            track: false,
            moved: Vec::new(),
            permuted: false,
        }
    }

    /// Vertex-sorts each feature when `sort`.
    pub(crate) fn vertex_sorted(self, sort: bool) -> Self {
        Self { sort, ..self }
    }

    /// Records where each stored vertex came from when `track`, see [`Self::finish_feature`].
    pub(crate) fn tracked(self, track: bool) -> Self {
        Self { track, ..self }
    }

    /// Ends the feature pushed since the last call, returning where each of its stored vertices came from.
    /// Is [`VertexMoves::Kept`] when not tracking.
    pub(crate) fn finish_feature(&mut self) -> VertexMoves {
        let moved = std::mem::take(&mut self.moved);
        if std::mem::take(&mut self.permuted) {
            VertexMoves::Moved(moved)
        } else {
            VertexMoves::Kept
        }
    }

    pub(crate) fn geom(&mut self, geom: &Geometry<i32>) {
        match geom {
            Geometry::<i32>::Point(p) => self.point(p.0),
            Geometry::<i32>::Line(l) => self.line(&[l.start, l.end]),
            Geometry::<i32>::LineString(ls) => self.line(&ls.0),
            Geometry::<i32>::Polygon(p) => self.polygon(rings(p)),
            Geometry::<i32>::MultiPoint(mp) => self.multi_point(mp.0.iter().map(|p| p.0)),
            Geometry::<i32>::MultiLineString(mls) => {
                self.multi_line(mls.0.iter().map(|ls| ls.0.as_slice()));
            }
            Geometry::<i32>::MultiPolygon(mp) => self.multi_polygon(mp.0.iter().map(rings)),
            Geometry::<i32>::Triangle(t) => self.polygon(rings(&t.to_polygon())),
            Geometry::<i32>::Rect(r) => self.polygon(rings(&r.to_polygon())),
            Geometry::<i32>::GeometryCollection(gc) => {
                for g in gc {
                    self.geom(g);
                }
            }
        }
    }

    pub(crate) fn point(&mut self, coord: Coord<i32>) {
        self.keep(1);
        self.values.push_point(coord);
    }

    pub(crate) fn multi_point(&mut self, points: impl Iterator<Item = Coord<i32>>) {
        if !self.sort {
            let mut count = 0;
            self.values.push_multi_point(points.inspect(|_| count += 1));
            self.keep(count);
            return;
        }
        let points: Vec<_> = points.collect();
        let chain = nearest_neighbour_chain(&points, self.last_vertex());
        self.record(chain.iter().copied());
        self.values
            .push_multi_point(chain.iter().map(|&i| points[i]));
    }

    /// Pushes a mesh in source order, since its triangles name its vertices by position.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn mesh(
        &mut self,
        vertices: impl Iterator<Item = Coord<i32>>,
        triangles: &[[u32; 3]],
    ) {
        let mut count = 0;
        self.values
            .push_mesh(vertices.inspect(|_| count += 1), triangles);
        self.keep(count);
    }

    pub(crate) fn line(&mut self, line: &[Coord<i32>]) {
        let line = self.place_line(line, self.last_vertex());
        self.values.push_linestring(&line);
    }

    pub(crate) fn multi_line<'c>(
        &mut self,
        lines: impl ExactSizeIterator<Item = &'c [Coord<i32>]>,
    ) {
        if !self.sort && !self.track {
            self.values.push_multi_linestring(lines);
            return;
        }
        let mut last = self.last_vertex();
        let lines: Vec<_> = lines
            .map(|line| {
                let line = self.place_line(line, last);
                last = line.last().copied().unwrap_or(last);
                line
            })
            .collect();
        self.values
            .push_multi_linestring(lines.iter().map(AsRef::as_ref));
    }

    /// Pushes a polygon given as its rings, exterior first, each open or closed.
    pub(crate) fn polygon<'c>(&mut self, rings: impl Iterator<Item = &'c [Coord<i32>]> + Clone) {
        if !self.sort && !self.track && !any_reversal(rings.clone()) {
            self.values.push_wound_polygon(rings);
            return;
        }
        let mut last = self.last_vertex();
        let rings = self.place_rings(rings, &mut last);
        self.values
            .push_wound_polygon(rings.iter().map(AsRef::as_ref));
    }

    /// Pushes polygons, each given as its rings like [`Self::polygon`].
    pub(crate) fn multi_polygon<'c, R>(
        &mut self,
        polygons: impl ExactSizeIterator<Item = R> + Clone,
    ) where
        R: Iterator<Item = &'c [Coord<i32>]>,
    {
        if !self.sort && !self.track && !polygons.clone().any(any_reversal) {
            self.values.push_wound_multi_polygon(polygons);
            return;
        }
        let mut last = self.last_vertex();
        let polygons: Vec<_> = polygons
            .map(|rings| self.place_rings(rings, &mut last))
            .collect();
        self.values
            .push_wound_multi_polygon(polygons.iter().map(|rings| rings.iter().map(AsRef::as_ref)));
    }

    /// The vertex stored last, or the origin the vertex deltas start from.
    pub(crate) fn last_vertex(&self) -> Coord<i32> {
        self.values.last_vertex().unwrap_or_default()
    }

    /// Records `len` vertices that keep their place.
    fn keep(&mut self, len: usize) {
        if self.track {
            let base = self.moved.len();
            self.moved.extend(base..base + len);
        }
    }

    /// Records the next part's vertices, stored in `order` of their position in the part.
    fn record(&mut self, order: impl ExactSizeIterator<Item = usize> + Clone) {
        if !self.track {
            return;
        }
        if !self.permuted && order.clone().enumerate().any(|(i, at)| i != at) {
            self.permuted = true;
        }
        let base = self.moved.len();
        self.moved.extend(order.map(|at| base + at));
    }

    /// The line, reversed under vertex sort when its end is nearer `last` than its start.
    fn place_line<'c>(
        &mut self,
        line: &'c [Coord<i32>],
        last: Coord<i32>,
    ) -> Cow<'c, [Coord<i32>]> {
        let reverse = self.sort
            && match (line.first(), line.last()) {
                (Some(&first), Some(&end)) => distance(last, end) < distance(last, first),
                _ => false,
            };
        if !reverse {
            self.keep(line.len());
            return Cow::Borrowed(line);
        }
        self.record((0..line.len()).rev());
        Cow::Owned(line.iter().rev().copied().collect())
    }

    /// The rings of one polygon, each wound and, under vertex sort, rotated to start near the vertex stored before it.
    fn place_rings<'c>(
        &mut self,
        rings: impl Iterator<Item = &'c [Coord<i32>]>,
        last: &mut Coord<i32>,
    ) -> Vec<Cow<'c, [Coord<i32>]>> {
        rings
            .enumerate()
            .map(|(i, ring)| {
                let ring = self.place_ring(ring, i == 0, *last);
                *last = ring[..stored_len(&ring)].last().copied().unwrap_or(*last);
                ring
            })
            .collect()
    }

    /// The ring wound for its role and, under vertex sort, rotated to start at its stored vertex nearest `last`.
    /// Only rings of 3 or more stored vertices are rotated.
    /// A changed ring comes back closed.
    fn place_ring<'c>(
        &mut self,
        ring: &'c [Coord<i32>],
        exterior: bool,
        last: Coord<i32>,
    ) -> Cow<'c, [Coord<i32>]> {
        let len = stored_len(ring);
        let stored = &ring[..len];
        let reverse = needs_reversal(ring, exterior);
        let wound = |i: usize| {
            if reverse {
                reversed_position(i, len)
            } else {
                i
            }
        };
        let start = if self.sort && len >= 3 {
            (0..len)
                .min_by_key(|&i| distance(last, stored[wound(i)]))
                .unwrap_or(0)
        } else {
            0
        };
        let order = (0..len).map(|i| wound((i + start) % len));
        if !reverse && start == 0 {
            self.keep(len);
            return Cow::Borrowed(ring);
        }
        self.record(order.clone());
        let mut placed: Vec<_> = order.map(|i| stored[i]).collect();
        placed.extend(placed.first().copied());
        Cow::Owned(placed)
    }
}

/// Whether any ring of the polygon winds against MVT.
fn any_reversal<'c>(rings: impl Iterator<Item = &'c [Coord<i32>]>) -> bool {
    rings
        .enumerate()
        .any(|(i, ring)| needs_reversal(ring, i == 0))
}

/// The Manhattan distance between two vertices.
pub(crate) fn distance(a: Coord<i32>, b: Coord<i32>) -> u64 {
    (i64::from(a.x) - i64::from(b.x)).unsigned_abs()
        + (i64::from(a.y) - i64::from(b.y)).unsigned_abs()
}

/// The order a walk from `start` visits `points` in, always stepping to the nearest point not yet visited, the earliest of equally near ones.
/// Distinct points are bucketed in a grid of about one per cell, and copies of a point are visited together.
fn nearest_neighbour_chain(points: &[Coord<i32>], start: Coord<i32>) -> Vec<usize> {
    let mut by_coord: Vec<usize> = (0..points.len()).collect();
    by_coord.sort_unstable_by_key(|&i| (points[i].x, points[i].y, i));
    let mut copies: Vec<&[usize]> = by_coord.chunk_by(|&a, &b| points[a] == points[b]).collect();
    // Numbered by first appearance, the nearest site tie-breaks to the earliest point.
    copies.sort_unstable_by_key(|same| same[0]);
    let sites: Vec<_> = copies.iter().map(|same| points[same[0]]).collect();
    let Some(mut grid) = Grid::new(&sites) else {
        return Vec::new();
    };
    let mut at = start;
    let mut chain = Vec::with_capacity(points.len());
    while let Some(site) = grid.take_nearest(&sites, at) {
        chain.extend_from_slice(copies[site]);
        at = sites[site];
    }
    chain
}

/// The points not yet visited, by grid cell.
struct Grid {
    min: Coord<i64>,
    max: Coord<i64>,
    /// How many cells each axis has.
    side: i64,
    cell: Coord<i64>,
    /// Point indexes, grouped by cell.
    items: Vec<u32>,
    /// Where each cell's group starts in `items`.
    starts: Vec<u32>,
    /// How many points of each cell's group are not yet visited, which come first in it.
    live: Vec<u32>,
    /// Where each point is in `items`.
    slots: Vec<u32>,
    left: usize,
}

impl Grid {
    fn new(points: &[Coord<i32>]) -> Option<Self> {
        let first = points.first()?;
        let mut min = Coord {
            x: i64::from(first.x),
            y: i64::from(first.y),
        };
        let mut max = min;
        for p in points {
            min.x = min.x.min(i64::from(p.x));
            min.y = min.y.min(i64::from(p.y));
            max.x = max.x.max(i64::from(p.x));
            max.y = max.y.max(i64::from(p.y));
        }
        let side = i64::try_from(points.len().isqrt().max(1)).expect("point count fits i64");
        let cell = Coord {
            x: (max.x - min.x) / side + 1,
            y: (max.y - min.y) / side + 1,
        };
        let mut grid = Self {
            min,
            max,
            side,
            cell,
            items: vec![0; points.len()],
            starts: Vec::new(),
            live: vec![0; usize::try_from(side * side).expect("cell count fits usize")],
            slots: vec![0; points.len()],
            left: points.len(),
        };
        let cells: Vec<usize> = points
            .iter()
            .map(|p| {
                grid.index(grid.cell_of(Coord {
                    x: i64::from(p.x),
                    y: i64::from(p.y),
                }))
            })
            .collect();
        for &c in &cells {
            grid.live[c] += 1;
        }
        let mut next = 0;
        grid.starts = grid
            .live
            .iter()
            .map(|&count| {
                let start = next;
                next += count;
                start
            })
            .collect();
        let mut fill = grid.starts.clone();
        for (point, &c) in cells.iter().enumerate() {
            let slot = fill[c];
            fill[c] += 1;
            grid.items[slot as usize] = u32::try_from(point).expect("point count fits u32");
            grid.slots[point] = slot;
        }
        Some(grid)
    }

    fn cell_of(&self, p: Coord<i64>) -> Coord<i64> {
        Coord {
            x: (p.x - self.min.x) / self.cell.x,
            y: (p.y - self.min.y) / self.cell.y,
        }
    }

    fn index(&self, cell: Coord<i64>) -> usize {
        usize::try_from(cell.y * self.side + cell.x).expect("cell is in the grid")
    }

    /// Removes and returns the point nearest `at`, the earliest of equally near ones.
    fn take_nearest(&mut self, points: &[Coord<i32>], at: Coord<i32>) -> Option<usize> {
        if self.left == 0 {
            return None;
        }
        // Every point lies in the bounding box, so the nearest to `at` is the nearest to `at` clamped into it.
        let at = Coord {
            x: i64::from(at.x).clamp(self.min.x, self.max.x),
            y: i64::from(at.y).clamp(self.min.y, self.max.y),
        };
        let origin = self.cell_of(at);
        let at = Coord {
            x: i32::try_from(at.x).expect("clamped into i32 points"),
            y: i32::try_from(at.y).expect("clamped into i32 points"),
        };
        let step = u64::try_from(self.cell.x.min(self.cell.y)).expect("cells are not empty");
        let mut best: Option<(u64, u32)> = None;
        for r in 0..self.side {
            // A point in a cell `r` cells away along an axis is more than `r - 1` cells away along it.
            if let Some((d, _)) = best
                && r >= 1
                && d <= u64::try_from(r - 1).expect("r is at least 1") * step
            {
                break;
            }
            for cell in ring_cells(origin, r, self.side) {
                let c = self.index(cell);
                let start = self.starts[c] as usize;
                for &point in &self.items[start..start + self.live[c] as usize] {
                    let d = distance(at, points[point as usize]);
                    if best.is_none_or(|best| (d, point) < best) {
                        best = Some((d, point));
                    }
                }
            }
        }
        let (_, point) = best.expect("a point is left");
        self.remove(points, point as usize);
        Some(point as usize)
    }

    fn remove(&mut self, points: &[Coord<i32>], point: usize) {
        let p = points[point];
        let c = self.index(self.cell_of(Coord {
            x: i64::from(p.x),
            y: i64::from(p.y),
        }));
        let slot = self.slots[point] as usize;
        let last = (self.starts[c] + self.live[c] - 1) as usize;
        self.items.swap(slot, last);
        self.slots[self.items[slot] as usize] = u32::try_from(slot).expect("slot fits u32");
        self.slots[point] = u32::try_from(last).expect("slot fits u32");
        self.live[c] -= 1;
        self.left -= 1;
    }
}

/// The cells of the square ring `r` cells around `origin`, within a grid of `side` cells per axis.
fn ring_cells(origin: Coord<i64>, r: i64, side: i64) -> impl Iterator<Item = Coord<i64>> {
    let inside = move |c: &Coord<i64>| (0..side).contains(&c.x) && (0..side).contains(&c.y);
    let (lo, hi) = (origin.x - r, origin.x + r);
    let rows = [origin.y - r, origin.y + r];
    let edges = rows
        .into_iter()
        .take(if r == 0 { 1 } else { 2 })
        .flat_map(move |y| (lo..=hi).map(move |x| Coord { x, y }));
    let sides = [lo, hi]
        .into_iter()
        .take(if r == 0 { 0 } else { 2 })
        .flat_map(move |x| (origin.y - r + 1..origin.y + r).map(move |y| Coord { x, y }));
    edges.chain(sides).filter(inside)
}
