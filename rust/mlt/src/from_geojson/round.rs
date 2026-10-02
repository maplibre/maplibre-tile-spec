//! Rounding of clipped geometries onto a tile's integer grid, keeping the z of
//! each stored vertex. Vertices that land on their predecessor are merged, and
//! parts left too short to draw are dropped.

use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};

use super::EXTENT;
use super::project::{Geom, Vertex};

/// Round a vertex of the unit square onto a tile's integer grid: `scale` is the
/// grid's size across the whole world at the tile's zoom and `origin` the tile's
/// corner on it. The z is already in grid units.
#[expect(
    clippy::cast_possible_truncation,
    reason = "clipped x/y lie within -buffer..=extent+buffer of the tile origin, and z was range-checked at projection; a cut point lies between its endpoints"
)]
fn round_vertex(v: &Vertex, scale: f64, origin: (f64, f64)) -> (Coord<i32>, i32) {
    let coord = Coord {
        x: (v.x * scale - origin.0).round() as i32,
        y: (v.y * scale - origin.1).round() as i32,
    };
    (coord, v.z.round() as i32)
}

/// Round a run of vertices, dropping any that lands on the previous one.
fn round_run(vertices: &[Vertex], scale: f64, origin: (f64, f64)) -> (Vec<Coord<i32>>, Vec<i32>) {
    let mut coords: Vec<Coord<i32>> = Vec::with_capacity(vertices.len());
    let mut z = Vec::with_capacity(vertices.len());
    for v in vertices {
        let (c, vz) = round_vertex(v, scale, origin);
        if coords.last() != Some(&c) {
            coords.push(c);
            z.push(vz);
        }
    }
    (coords, z)
}

/// Round a line, or `None` when fewer than two distinct vertices remain.
fn round_line(
    line: &[Vertex],
    scale: f64,
    origin: (f64, f64),
) -> Option<(LineString<i32>, Vec<i32>)> {
    let (coords, z) = round_run(line, scale, origin);
    (coords.len() >= 2).then(|| (LineString::new(coords), z))
}

/// Round an open ring and close it, or `None` when fewer than three distinct
/// vertices remain. The z values cover the stored vertices only.
fn round_ring(
    open: &[Vertex],
    scale: f64,
    origin: (f64, f64),
) -> Option<(LineString<i32>, Vec<i32>)> {
    let (mut coords, mut z) = round_run(open, scale, origin);
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop();
        z.pop();
    }
    if coords.len() < 3 {
        return None;
    }
    coords.push(coords[0]);
    Some((LineString::new(coords), z))
}

/// Round a polygon's rings, or `None` when its exterior collapses. Holes that
/// collapse are dropped.
fn round_polygon(
    rings: &[Vec<Vertex>],
    scale: f64,
    origin: (f64, f64),
) -> Option<(Polygon<i32>, Vec<i32>)> {
    let mut rings = rings.iter();
    let (exterior, mut z) = round_ring(rings.next()?, scale, origin)?;
    let (interiors, holes_z) =
        gather(rings.map(|r| round_ring(r, scale, origin))).unwrap_or_default();
    z.extend(holes_z);
    Some((Polygon::new(exterior, interiors), z))
}

/// Collect the parts that survive rounding, with their z in order; `None` when none do.
fn gather<T>(parts: impl Iterator<Item = Option<(T, Vec<i32>)>>) -> Option<(Vec<T>, Vec<i32>)> {
    let mut out = Vec::new();
    let mut z = Vec::new();
    for (part, part_z) in parts.flatten() {
        out.push(part);
        z.extend(part_z);
    }
    (!out.is_empty()).then_some((out, z))
}

/// Round a geometry already clipped to tile `(zoom, col, row)` onto that tile's
/// integer grid, with the z of each stored vertex. `None` when nothing survives
/// rounding. Multi-part geometries keep their type; a line that was split by
/// clipping arrives here as a `MultiLineString` already.
pub(super) fn to_tile(
    geom: &Geom,
    zoom: u8,
    col: u32,
    row: u32,
) -> Option<(Geometry<i32>, Vec<i32>)> {
    let extent = f64::from(EXTENT);
    let scale = f64::from(1_u32 << zoom) * extent;
    let origin = (f64::from(col) * extent, f64::from(row) * extent);
    Some(match geom {
        Geom::Point(v) => {
            let (c, z) = round_vertex(v, scale, origin);
            (Geometry::Point(Point(c)), vec![z])
        }
        Geom::MultiPoint(points) => {
            let (coords, z): (Vec<Point<i32>>, Vec<i32>) = points
                .iter()
                .map(|v| {
                    let (c, z) = round_vertex(v, scale, origin);
                    (Point(c), z)
                })
                .unzip();
            (Geometry::MultiPoint(MultiPoint::new(coords)), z)
        }
        Geom::LineString(line) => {
            let (line, z) = round_line(line, scale, origin)?;
            (Geometry::LineString(line), z)
        }
        Geom::MultiLineString(parts) => {
            let (lines, z) = gather(parts.iter().map(|p| round_line(p, scale, origin)))?;
            (Geometry::MultiLineString(MultiLineString::new(lines)), z)
        }
        Geom::Polygon(rings) => {
            let (polygon, z) = round_polygon(rings, scale, origin)?;
            (Geometry::Polygon(polygon), z)
        }
        Geom::MultiPolygon(polygons) => {
            let (polygons, z) = gather(polygons.iter().map(|p| round_polygon(p, scale, origin)))?;
            (Geometry::MultiPolygon(MultiPolygon::new(polygons)), z)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64, z: f64) -> Vertex {
        Vertex { x, y, z }
    }

    const ORIGIN: (f64, f64) = (0.0, 0.0);

    #[test]
    fn a_ring_keeps_its_stored_vertices_and_their_z() {
        let open = [
            v(0.2, 0.0, 1.0),
            v(10.0, 0.0, 2.0),
            v(10.0, 10.0, 3.0),
            v(0.0, 10.0, 4.4),
        ];
        let (ring, z) = round_ring(&open, 1.0, ORIGIN).unwrap();
        assert_eq!(ring.0.len(), 5);
        assert_eq!(ring.0[0], ring.0[4]);
        assert_eq!(z, [1, 2, 3, 4]);
    }

    #[test]
    fn a_ring_collapsing_under_rounding_is_dropped() {
        let open = [v(0.1, 0.1, 0.0), v(0.2, 0.2, 0.0), v(0.3, 0.1, 0.0)];
        assert!(round_ring(&open, 1.0, ORIGIN).is_none());
    }

    #[test]
    fn a_closed_input_ring_is_not_double_closed() {
        let open = [
            v(0.0, 0.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(0.0, 0.0, 0.0),
        ];
        let (ring, z) = round_ring(&open, 1.0, ORIGIN).unwrap();
        assert_eq!(ring.0.len(), 4);
        assert_eq!(z.len(), 3);
    }

    #[test]
    fn to_tile_rounds_onto_the_tile_grid() {
        let geom = Geom::LineString(vec![v(0.25, 0.5, 0.4), v(0.5, 0.5, 100.6)]);
        let (geometry, z) = to_tile(&geom, 1, 0, 0).unwrap();
        let Geometry::LineString(line) = geometry else {
            panic!("a line string, got {geometry:?}");
        };
        assert_eq!(
            line.0,
            [Coord { x: 2048, y: 4096 }, Coord { x: 4096, y: 4096 }]
        );
        assert_eq!(z, [0, 101]);
    }

    #[test]
    fn a_multi_line_string_losing_every_part_to_rounding_is_dropped() {
        let geom = Geom::MultiLineString(vec![vec![v(0.5, 0.5, 0.0), v(0.500_000_01, 0.5, 1.0)]]);
        assert!(to_tile(&geom, 0, 0, 0).is_none());
    }

    #[test]
    fn a_polygon_whose_hole_collapses_keeps_only_the_shell() {
        let rings = vec![
            vec![
                v(0.0, 0.0, 1.0),
                v(10.0, 0.0, 1.0),
                v(10.0, 10.0, 1.0),
                v(0.0, 10.0, 1.0),
            ],
            vec![v(5.0, 5.0, 2.0), v(5.1, 5.0, 2.0), v(5.1, 5.1, 2.0)],
        ];
        let (polygon, z) = round_polygon(&rings, 1.0, ORIGIN).unwrap();
        assert_eq!(polygon.interiors(), &[] as &[LineString<i32>]);
        assert_eq!(z, [1; 4]);
    }
}
