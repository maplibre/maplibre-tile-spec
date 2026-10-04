//! Clipping of projected lines and rings to a tile rectangle, interpolating the
//! elevation at every cut.
//!
//! Both algorithms treat the rectangle as four half-planes and find cut points
//! by [`Vertex::lerp`], so z is interpolated the same way. They differ in what
//! they keep connected:
//!
//! - Lines use Liang-Barsky, which clips one segment at a time. A line that
//!   leaves and re-enters becomes separate parts, so no stroke is invented.
//! - Rings use Sutherland-Hodgman, which clips the whole ring against one edge
//!   at a time and always returns one closed ring: the part outside is replaced
//!   by the tile border, which is what a fill needs. A concave ring re-entering
//!   the tile keeps a zero-width bridge along the edge instead of splitting, as
//!   in `geojson-vt`; [`clip_geom`] drops polygons left with no area, counting
//!   the area of a vertical polygon such as a wall.
//! - Each ring of a polygon is clipped on its own, so a hole crossing the edge
//!   shares the tile border with its exterior instead of becoming a notch in
//!   it, again as in `geojson-vt`. Like the bridge, this is invalid as a simple
//!   feature but fills the right area, and it lies in the buffer, outside the
//!   visible tile.
//!
//! Points are not cut; the caller says which ones a tile owns.

use super::project::{Geom, Vertex};

/// An axis-aligned rectangle in the same coordinates as the vertices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// The parameter range `t0..=t1` of segment `a -> b` inside `rect`, or `None`
/// when the segment misses it. Liang-Barsky.
fn liang_barsky(a: Vertex, b: Vertex, rect: &Rect) -> Option<(f64, f64)> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    // (p, q) per edge: the segment is inside the edge's half-plane where p * t <= q.
    let checks = [
        (-dx, a.x - rect.min_x),
        (dx, rect.max_x - a.x),
        (-dy, a.y - rect.min_y),
        (dy, rect.max_y - a.y),
    ];
    for (p, q) in checks {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    Some((t0, t1))
}

/// Clip a polyline to `rect`. Each returned part has at least two vertices.
pub(super) fn clip_line(line: &[Vertex], rect: &Rect) -> Vec<Vec<Vertex>> {
    let mut parts: Vec<Vec<Vertex>> = Vec::new();
    let mut part: Vec<Vertex> = Vec::new();
    let mut flush = |part: &mut Vec<Vertex>| {
        if part.len() >= 2 {
            parts.push(std::mem::take(part));
        } else {
            part.clear();
        }
    };
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let Some((t0, t1)) = liang_barsky(a, b, rect) else {
            flush(&mut part);
            continue;
        };
        // A segment that starts inside continues the current part; one that
        // enters from outside starts a new part at the entry point. A segment
        // that only touches the rectangle at a point does not start a part.
        if t0 > 0.0 || part.is_empty() {
            flush(&mut part);
            if t0 >= t1 {
                continue;
            }
            part.push(a.lerp(b, t0));
        }
        part.push(a.lerp(b, t1));
        if t1 < 1.0 {
            flush(&mut part);
        }
    }
    flush(&mut part);
    parts
}

/// Which side of a rectangle edge a ring is clipped against.
#[derive(Clone, Copy)]
enum Edge {
    MinX(f64),
    MaxX(f64),
    MinY(f64),
    MaxY(f64),
}

impl Edge {
    fn inside(self, v: Vertex) -> bool {
        match self {
            Self::MinX(x) => v.x >= x,
            Self::MaxX(x) => v.x <= x,
            Self::MinY(y) => v.y >= y,
            Self::MaxY(y) => v.y <= y,
        }
    }

    /// Where segment `a -> b` crosses the edge line. Only called when `a` and `b`
    /// lie on different sides, so the denominator is non-zero.
    fn cross(self, a: Vertex, b: Vertex) -> Vertex {
        let t = match self {
            Self::MinX(x) | Self::MaxX(x) => (x - a.x) / (b.x - a.x),
            Self::MinY(y) | Self::MaxY(y) => (y - a.y) / (b.y - a.y),
        };
        a.lerp(b, t)
    }
}

/// Clip a ring to `rect` with Sutherland-Hodgman. The input may or may not
/// repeat its first vertex at the end; the output never does and is empty when
/// fewer than three vertices remain.
pub(super) fn clip_ring(ring: &[Vertex], rect: &Rect) -> Vec<Vertex> {
    // A closing vertex is an exact repeat of the first, z included: one only
    // above or below it is a corner of a vertical ring.
    let open = match ring {
        [first, .., last] if first == last => &ring[..ring.len() - 1],
        _ => ring,
    };
    let mut out: Vec<Vertex> = open.to_vec();
    let mut input: Vec<Vertex> = Vec::with_capacity(open.len() + 4);
    for edge in [
        Edge::MinX(rect.min_x),
        Edge::MaxX(rect.max_x),
        Edge::MinY(rect.min_y),
        Edge::MaxY(rect.max_y),
    ] {
        if out.is_empty() {
            break;
        }
        std::mem::swap(&mut input, &mut out);
        out.clear();
        let mut prev = input[input.len() - 1];
        for &cur in &input {
            let (prev_in, cur_in) = (edge.inside(prev), edge.inside(cur));
            if cur_in {
                if !prev_in {
                    out.push(edge.cross(prev, cur));
                }
                out.push(cur);
            } else if prev_in {
                out.push(edge.cross(prev, cur));
            }
            prev = cur;
        }
    }
    if out.len() < 3 {
        out.clear();
    }
    out
}

/// A clipped polygon keeping less than this fraction of the rectangle's area (or,
/// for a wall, of the rectangle's width times its height) is treated as empty:
/// below it lies only floating-point noise from the cuts.
const EMPTY_AREA_FRACTION: f64 = 1e-9;

/// Twice the unsigned area of an open ring, measured from the rectangle's corner
/// so that a tiny ring far from the origin does not lose its area to cancellation.
fn ring_area2(ring: &[Vertex], rect: &Rect) -> f64 {
    let mut sum = 0.0;
    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        let (ax, ay) = (a.x - rect.min_x, a.y - rect.min_y);
        let (bx, by) = (b.x - rect.min_x, b.y - rect.min_y);
        sum += ax * by - bx * ay;
    }
    sum.abs()
}

/// Twice the area of an open ring on the vertical plane it stands in, measured
/// from the rectangle's corner and the ring's lowest z as [`ring_area2`] does. It
/// is the horizontal part of the ring's Newell normal, so it is meaningful for a
/// ring with no area on the ground, like a wall, whose x/y share units and whose
/// z is in grid units.
fn vertical_area2(ring: &[Vertex], rect: &Rect, min_z: f64) -> f64 {
    let (mut yz, mut zx) = (0.0, 0.0);
    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        let (ax, ay, az) = (a.x - rect.min_x, a.y - rect.min_y, a.z - min_z);
        let (bx, by, bz) = (b.x - rect.min_x, b.y - rect.min_y, b.z - min_z);
        yz += ay * bz - by * az;
        zx += az * bx - bz * ax;
    }
    yz.hypot(zx)
}

/// Whether a clipped polygon has no area left. One with area on the ground is
/// measured there, net of its holes; one without, like a wall, is measured on
/// the vertical plane it stands in, against the rectangle's width times its
/// height. Either way, less than [`EMPTY_AREA_FRACTION`] of that is empty.
fn is_empty_polygon(exterior: &[Vertex], holes: &[Vec<Vertex>], rect: &Rect) -> bool {
    let (width, height) = (rect.max_x - rect.min_x, rect.max_y - rect.min_y);
    let ground = 2.0 * width * height * EMPTY_AREA_FRACTION;
    let exterior_area2 = ring_area2(exterior, rect);
    if exterior_area2 > ground {
        let holes_area2: f64 = holes.iter().map(|h| ring_area2(h, rect)).sum();
        return exterior_area2 - holes_area2 <= ground;
    }
    let (min_z, max_z) = exterior
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v.z), hi.max(v.z))
        });
    let vertical = 2.0 * width.max(height) * (max_z - min_z) * EMPTY_AREA_FRACTION;
    let holes_area2: f64 = holes.iter().map(|h| vertical_area2(h, rect, min_z)).sum();
    vertical_area2(exterior, rect, min_z) - holes_area2 <= vertical
}

/// Clip a whole geometry to `rect`, keeping the vertices' coordinate space.
///
/// A `LineString` that leaves and re-enters becomes a `MultiLineString`; other
/// multi-part types keep their type and drop the parts that vanish. A polygon
/// is gone when its exterior vanishes or when its holes leave it no area, as
/// happens to a rectangle lying entirely inside a hole (see [`is_empty_polygon`],
/// which keeps a vertical polygon such as a wall); a hole that vanishes is
/// dropped. Points are not cut: `owns` says which ones the tile keeps, so that
/// a point on a shared edge goes to exactly one tile. `None` when nothing remains.
pub(super) fn clip_geom(geom: &Geom, rect: &Rect, owns: impl Fn(&Vertex) -> bool) -> Option<Geom> {
    let polygon = |rings: &[Vec<Vertex>]| {
        let mut rings = rings.iter();
        let exterior = clip_ring(rings.next()?, rect);
        if exterior.is_empty() {
            return None;
        }
        let holes: Vec<Vec<Vertex>> = rings
            .map(|r| clip_ring(r, rect))
            .filter(|r| !r.is_empty())
            .collect();
        if is_empty_polygon(&exterior, &holes, rect) {
            return None;
        }
        let mut out = Vec::with_capacity(holes.len() + 1);
        out.push(exterior);
        out.extend(holes);
        Some(out)
    };
    Some(match geom {
        Geom::Point(v) => Geom::Point(*owns(v).then_some(v)?),
        Geom::MultiPoint(points) => {
            let kept: Vec<Vertex> = points.iter().filter(|v| owns(v)).copied().collect();
            if kept.is_empty() {
                return None;
            }
            Geom::MultiPoint(kept)
        }
        Geom::LineString(line) => {
            let mut parts = clip_line(line, rect);
            match parts.len() {
                0 => return None,
                1 => Geom::LineString(parts.remove(0)),
                _ => Geom::MultiLineString(parts),
            }
        }
        Geom::MultiLineString(lines) => {
            let parts: Vec<Vec<Vertex>> = lines.iter().flat_map(|l| clip_line(l, rect)).collect();
            if parts.is_empty() {
                return None;
            }
            Geom::MultiLineString(parts)
        }
        Geom::Polygon(rings) => Geom::Polygon(polygon(rings)?),
        Geom::MultiPolygon(polygons) => {
            let kept: Vec<Vec<Vec<Vertex>>> = polygons.iter().filter_map(|p| polygon(p)).collect();
            if kept.is_empty() {
                return None;
            }
            Geom::MultiPolygon(kept)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64, z: f64) -> Vertex {
        Vertex { x, y, z }
    }

    fn close(a: Vertex, b: Vertex) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9 && (a.z - b.z).abs() < 1e-9
    }

    #[track_caller]
    fn assert_parts(actual: &[Vec<Vertex>], expected: &[Vec<Vertex>]) {
        let same = actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| a.len() == e.len() && a.iter().zip(e).all(|(a, e)| close(*a, *e)));
        assert!(same, "expected {expected:?}, got {actual:?}");
    }

    const UNIT: Rect = Rect {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 10.0,
        max_y: 10.0,
    };

    /// The point-ownership rule of the tests: the rectangle, half-open.
    fn inside_unit(v: &Vertex) -> bool {
        (0.0..10.0).contains(&v.x) && (0.0..10.0).contains(&v.y)
    }

    #[test]
    fn a_line_inside_the_rect_is_unchanged() {
        let line = [v(1.0, 1.0, 5.0), v(9.0, 9.0, 7.0)];
        assert_eq!(clip_line(&line, &UNIT), vec![line.to_vec()]);
    }

    #[test]
    fn a_line_outside_the_rect_vanishes() {
        let line = [v(11.0, 1.0, 0.0), v(19.0, 9.0, 0.0)];
        assert_eq!(clip_line(&line, &UNIT), Vec::<Vec<Vertex>>::new());
    }

    #[test]
    fn a_crossing_line_is_cut_with_z_interpolated() {
        let line = [v(-10.0, 5.0, 0.0), v(20.0, 5.0, 300.0)];
        assert_parts(
            &clip_line(&line, &UNIT),
            &[vec![v(0.0, 5.0, 100.0), v(10.0, 5.0, 200.0)]],
        );
    }

    #[test]
    fn a_line_leaving_and_returning_splits_into_parts() {
        let line = [
            v(5.0, 2.0, 0.0),
            v(15.0, 2.0, 10.0),
            v(15.0, 8.0, 20.0),
            v(5.0, 8.0, 30.0),
        ];
        assert_parts(
            &clip_line(&line, &UNIT),
            &[
                vec![v(5.0, 2.0, 0.0), v(10.0, 2.0, 5.0)],
                vec![v(10.0, 8.0, 25.0), v(5.0, 8.0, 30.0)],
            ],
        );
    }

    #[test]
    fn a_segment_only_touching_the_rect_corner_yields_no_part() {
        let line = [v(0.0, 0.0, 0.0), v(-5.0, -5.0, 1.0)];
        assert_eq!(clip_line(&line, &UNIT), Vec::<Vec<Vertex>>::new());
    }

    #[test]
    fn a_ring_inside_the_rect_loses_only_its_closing_vertex() {
        let ring = [
            v(1.0, 1.0, 1.0),
            v(9.0, 1.0, 2.0),
            v(9.0, 9.0, 3.0),
            v(1.0, 1.0, 1.0),
        ];
        assert_eq!(clip_ring(&ring, &UNIT), ring[..3].to_vec());
    }

    #[test]
    fn a_ring_around_the_rect_becomes_the_rect() {
        let ring = [
            v(-10.0, -10.0, 0.0),
            v(20.0, -10.0, 0.0),
            v(20.0, 20.0, 0.0),
            v(-10.0, 20.0, 0.0),
        ];
        let out = clip_ring(&ring, &UNIT);
        assert_eq!(out.len(), 4);
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
            assert!(
                out.iter().any(|p| close(*p, v(x, y, 0.0))),
                "({x}, {y}) in {out:?}"
            );
        }
    }

    #[test]
    fn a_ring_straddling_one_edge_is_cut_with_z_interpolated() {
        // A square from x=5 to x=15 at z rising with x: the cut at x=10 reads z=50.
        let ring = [
            v(5.0, 2.0, 0.0),
            v(15.0, 2.0, 100.0),
            v(15.0, 8.0, 100.0),
            v(5.0, 8.0, 0.0),
            v(5.0, 2.0, 0.0),
        ];
        let out = clip_ring(&ring, &UNIT);
        assert_eq!(out.len(), 4);
        let cuts: Vec<_> = out.iter().filter(|p| (p.x - 10.0).abs() < 1e-9).collect();
        assert_eq!(cuts.len(), 2);
        assert!(cuts.iter().all(|p| (p.z - 50.0).abs() < 1e-9), "{cuts:?}");
    }

    #[test]
    fn a_ring_outside_the_rect_vanishes() {
        let ring = [v(11.0, 11.0, 0.0), v(19.0, 11.0, 0.0), v(19.0, 19.0, 0.0)];
        assert_eq!(clip_ring(&ring, &UNIT), Vec::<Vertex>::new());
    }

    #[test]
    fn a_line_split_by_the_rect_becomes_a_multi_line_string() {
        let geom = Geom::LineString(vec![
            v(5.0, 2.0, 0.0),
            v(15.0, 2.0, 10.0),
            v(15.0, 8.0, 20.0),
            v(5.0, 8.0, 30.0),
        ]);
        // The parts themselves are checked by `a_line_leaving_and_returning_splits_into_parts`.
        assert!(matches!(
            clip_geom(&geom, &UNIT, inside_unit),
            Some(Geom::MultiLineString(parts)) if parts.len() == 2
        ));
    }

    #[test]
    fn a_polygon_whose_hole_is_clipped_away_keeps_only_the_shell() {
        let geom = Geom::Polygon(vec![
            vec![
                v(0.0, 0.0, 1.0),
                v(20.0, 0.0, 1.0),
                v(20.0, 20.0, 1.0),
                v(0.0, 20.0, 1.0),
            ],
            vec![
                v(12.0, 12.0, 2.0),
                v(18.0, 12.0, 2.0),
                v(18.0, 18.0, 2.0),
                v(12.0, 18.0, 2.0),
            ],
        ]);
        let Some(Geom::Polygon(rings)) = clip_geom(&geom, &UNIT, inside_unit) else {
            panic!("a polygon");
        };
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 4);
    }

    #[test]
    fn a_rect_inside_a_hole_leaves_no_polygon() {
        let geom = Geom::Polygon(vec![
            vec![
                v(-50.0, -50.0, 0.0),
                v(50.0, -50.0, 0.0),
                v(50.0, 50.0, 0.0),
                v(-50.0, 50.0, 0.0),
            ],
            vec![
                v(-20.0, -20.0, 0.0),
                v(30.0, -20.0, 0.0),
                v(30.0, 30.0, 0.0),
                v(-20.0, 30.0, 0.0),
            ],
        ]);
        assert_eq!(clip_geom(&geom, &UNIT, inside_unit), None);
    }

    #[test]
    fn a_hole_overlapping_the_rect_is_kept() {
        let geom = Geom::Polygon(vec![
            vec![
                v(-50.0, -50.0, 0.0),
                v(50.0, -50.0, 0.0),
                v(50.0, 50.0, 0.0),
                v(-50.0, 50.0, 0.0),
            ],
            vec![
                v(5.0, 5.0, 0.0),
                v(30.0, 5.0, 0.0),
                v(30.0, 30.0, 0.0),
                v(5.0, 30.0, 0.0),
            ],
        ]);
        let Some(Geom::Polygon(rings)) = clip_geom(&geom, &UNIT, inside_unit) else {
            panic!("a polygon");
        };
        assert_eq!(rings.len(), 2);
        assert!((ring_area2(&rings[0], &UNIT) - 200.0).abs() < 1e-9);
        assert!((ring_area2(&rings[1], &UNIT) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_polygon_only_touching_the_rect_edge_leaves_no_polygon() {
        let geom = Geom::Polygon(vec![vec![
            v(10.0, 2.0, 0.0),
            v(20.0, 2.0, 0.0),
            v(20.0, 8.0, 0.0),
            v(10.0, 8.0, 0.0),
        ]]);
        assert_eq!(clip_geom(&geom, &UNIT, inside_unit), None);
    }

    #[test]
    fn a_sloped_polygon_only_touching_the_rect_edge_leaves_no_polygon() {
        // On the plane z = 10y + 5(x - 10), so its part on the edge is a line.
        let geom = Geom::Polygon(vec![vec![
            v(10.0, 2.0, 20.0),
            v(20.0, 2.0, 70.0),
            v(20.0, 8.0, 130.0),
            v(10.0, 8.0, 80.0),
        ]]);
        assert_eq!(clip_geom(&geom, &UNIT, inside_unit), None);
    }

    #[test]
    fn a_wall_inside_the_rect_is_kept() {
        let wall = vec![
            v(2.0, 5.0, 0.0),
            v(8.0, 5.0, 0.0),
            v(8.0, 5.0, 50.0),
            v(2.0, 5.0, 50.0),
        ];
        let geom = Geom::Polygon(vec![wall.clone()]);
        assert_eq!(
            clip_geom(&geom, &UNIT, inside_unit),
            Some(Geom::Polygon(vec![wall]))
        );
    }

    #[test]
    fn a_wall_crossing_the_rect_is_cut_keeping_its_height() {
        let geom = Geom::Polygon(vec![vec![
            v(5.0, 5.0, 0.0),
            v(15.0, 5.0, 0.0),
            v(15.0, 5.0, 50.0),
            v(5.0, 5.0, 50.0),
        ]]);
        let Some(Geom::Polygon(rings)) = clip_geom(&geom, &UNIT, inside_unit) else {
            panic!("a polygon");
        };
        assert_parts(
            &rings,
            &[vec![
                v(5.0, 5.0, 0.0),
                v(10.0, 5.0, 0.0),
                v(10.0, 5.0, 50.0),
                v(5.0, 5.0, 50.0),
            ]],
        );
    }

    #[test]
    fn a_vertical_line_inside_the_rect_is_kept() {
        let line = [v(5.0, 5.0, 0.0), v(5.0, 5.0, 10.0)];
        assert_eq!(clip_line(&line, &UNIT), vec![line.to_vec()]);
    }

    #[test]
    fn a_multi_polygon_drops_the_polygons_outside_the_rect() {
        let square = |x0: f64| {
            vec![vec![
                v(x0, 1.0, 0.0),
                v(x0 + 5.0, 1.0, 0.0),
                v(x0 + 5.0, 6.0, 0.0),
                v(x0, 6.0, 0.0),
            ]]
        };
        let geom = Geom::MultiPolygon(vec![square(1.0), square(30.0)]);
        let Some(Geom::MultiPolygon(polygons)) = clip_geom(&geom, &UNIT, inside_unit) else {
            panic!("a multi polygon");
        };
        assert_eq!(polygons.len(), 1);
        assert_eq!(
            clip_geom(&Geom::MultiPolygon(vec![square(30.0)]), &UNIT, inside_unit),
            None
        );
    }

    #[test]
    fn points_follow_the_ownership_rule_not_the_rect() {
        assert_eq!(
            clip_geom(&Geom::Point(v(0.0, 0.0, 1.0)), &UNIT, inside_unit),
            Some(Geom::Point(v(0.0, 0.0, 1.0)))
        );
        // On the far edge: inside the rect, but owned by the next tile.
        assert_eq!(
            clip_geom(&Geom::Point(v(10.0, 0.0, 1.0)), &UNIT, inside_unit),
            None
        );
        assert_eq!(
            clip_geom(
                &Geom::MultiPoint(vec![v(-1.0, 0.0, 0.0), v(1.0, 1.0, 2.0)]),
                &UNIT,
                inside_unit
            ),
            Some(Geom::MultiPoint(vec![v(1.0, 1.0, 2.0)]))
        );
    }
}
