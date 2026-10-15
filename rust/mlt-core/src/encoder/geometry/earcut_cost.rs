//! The client CPU time stored triangles save, to weigh against the bytes they add.

use usize_cast::IntoUsize as _;

use crate::decoder::{GeometryType, GeometryValues};

/// Nanoseconds a client spends decoding one stored triangle.
const DECODE_NS_PER_TRIANGLE: f64 = 20.0;

/// Nanoseconds `mapbox::earcut` takes on a Ryzen 9 3900 core for one polygon.
///
/// `n` counts its vertices as a client holds them, with each ring's closing point.
/// Fitted per polygon over five basemap and building tilesets, within ±10% on the median layer.
fn earcut_ns(n: f64, holes: f64) -> f64 {
    135.0 + n * (2.04 * n.min(80.0) - 15.7) + 6.9 * n * holes
}

/// The client nanoseconds the stored triangles of `geometry` save, net of decoding them.
///
/// `geometry` must be staged with tessellation, so it holds both the outlines and the triangles.
pub(crate) fn tessellation_saving_ns(geometry: &GeometryValues) -> f64 {
    let triangles = geometry.index_buffer.as_ref().map_or(0, Vec::len) / 3;
    #[expect(clippy::cast_precision_loss, reason = "an estimate")]
    let decode = DECODE_NS_PER_TRIANGLE * triangles as f64;
    earcut_ns_of_layer(geometry) - decode
}

/// The earcut nanoseconds of every polygon in `geometry`, which the client spends without triangles.
///
/// Staging gives a multi geometry one geometry offset and a line one ring, so the walk steps through them in feature order.
fn earcut_ns_of_layer(geometry: &GeometryValues) -> f64 {
    let (Some(parts), Some(rings)) = (&geometry.part_offsets, &geometry.ring_offsets) else {
        return 0.0;
    };
    let geoms = geometry.geometry_offsets.as_deref().unwrap_or(&[]);
    let span = |level: &[u32], at: usize| -> usize {
        match (level.get(at), level.get(at + 1)) {
            (Some(&start), Some(&end)) => end.saturating_sub(start).into_usize(),
            _ => 0,
        }
    };
    let (mut geom, mut part, mut ring) = (0, 0, 0);
    let mut total = 0.0;
    for kind in &geometry.vector_types {
        let polygons = match kind {
            GeometryType::Point => 0,
            GeometryType::LineString => {
                ring += 1;
                0
            }
            GeometryType::Polygon => 1,
            GeometryType::MultiPoint => {
                geom += 1;
                0
            }
            GeometryType::MultiLineString => {
                ring += span(geoms, geom);
                geom += 1;
                0
            }
            GeometryType::MultiPolygon => {
                geom += 1;
                span(geoms, geom - 1)
            }
        };
        for _ in 0..polygons {
            let ring_count = span(parts, part);
            let vertices: usize = (ring..ring + ring_count).map(|r| span(rings, r)).sum();
            #[expect(clippy::cast_precision_loss, reason = "an estimate")]
            let (n, holes) = (
                (vertices + ring_count) as f64,
                ring_count.saturating_sub(1) as f64,
            );
            total += earcut_ns(n, holes);
            part += 1;
            ring += ring_count;
        }
    }
    debug_assert_eq!(
        (part + 1, ring + 1),
        (parts.len(), rings.len()),
        "the walk ends where staging did"
    );
    total
}

#[cfg(test)]
mod tests {
    use geo_types::{
        Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
    };
    use insta::assert_snapshot;

    use super::tessellation_saving_ns;
    use crate::decoder::GeometryValues;

    fn square(x: i32, size: i32) -> LineString<i32> {
        LineString::from(vec![
            (x, x),
            (x + size, x),
            (x + size, x + size),
            (x, x + size),
            (x, x),
        ])
    }

    fn saving(geoms: &[Geometry<i32>]) -> String {
        let mut g = GeometryValues::new_tessellated();
        for geom in geoms {
            g.push_geom(geom);
        }
        format!("{:.1}", tessellation_saving_ns(&g))
    }

    #[test]
    fn a_square_saves_its_earcut_less_two_triangle_decodes() {
        assert_snapshot!(saving(&[Geometry::Polygon(Polygon::new(square(0, 10), vec![]))]), @"67.5");
    }

    #[test]
    fn a_multi_polygon_saves_the_earcut_of_each_part() {
        let parts = vec![
            Polygon::new(square(0, 10), vec![]),
            Polygon::new(square(20, 10), vec![]),
        ];
        assert_snapshot!(saving(&[Geometry::MultiPolygon(MultiPolygon(parts))]), @"135.0");
    }

    #[test]
    fn a_hole_costs_a_scan_of_the_polygon() {
        let holed = Polygon::new(square(0, 30), vec![square(10, 10)]);
        assert_snapshot!(saving(&[Geometry::Polygon(holed)]), @"91.0");
    }

    #[test]
    fn points_and_lines_save_nothing() {
        let line = Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)]));
        assert_snapshot!(saving(&[Geometry::Point(Point::new(1, 2)), line]), @"0.0");
    }

    #[test]
    fn a_line_beside_a_square_saves_only_the_square() {
        let line = Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)]));
        let square = Geometry::Polygon(Polygon::new(square(0, 10), vec![]));
        assert_snapshot!(saving(&[line, square]), @"67.5");
    }

    #[test]
    fn a_mixed_layer_saves_what_its_polygons_save_alone() {
        let line = || Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)]));
        let lines = MultiLineString(vec![
            LineString::from(vec![(0, 0), (1, 1)]),
            LineString::from(vec![(2, 2), (3, 9)]),
        ]);
        let squares = vec![
            Polygon::new(square(0, 10), vec![]),
            Polygon::new(square(20, 10), vec![]),
        ];
        assert_snapshot!(saving(&[
            line(),
            Geometry::MultiLineString(lines.clone()),
            Geometry::Polygon(Polygon::new(square(0, 10), vec![])),
            Geometry::MultiPoint(MultiPoint::from(vec![(1, 1), (2, 2)])),
            line(),
            Geometry::Polygon(Polygon::new(square(0, 30), vec![square(10, 10)])),
            Geometry::MultiLineString(lines),
            Geometry::MultiPolygon(MultiPolygon(squares)),
            Geometry::Point(Point::new(1, 2)),
            Geometry::MultiPolygon(MultiPolygon(vec![])),
            line(),
        ]), @"293.5");
    }
}
