//! Polygon ring winding in tile coordinates (x right, y down).
//!
//! MVT defines an exterior ring as one with a positive area by the surveyor's
//! formula and a hole as one with a negative area. Rings are named here by that
//! sign rather than as clockwise or counter-clockwise, which flips with the
//! direction of the y axis: a positive ring looks clockwise with y down, as the
//! MVT spec draws it, but counter-clockwise where y is drawn upwards, as on the
//! `mlt ui` map.

use mlt_core::geo_types::Coord;

/// Twice the signed area of a ring of `(x, y)` points by the surveyor's formula:
/// positive for an MVT exterior. The ring may be open or closed, since a closing
/// point adds nothing. Each term is measured from the first point, so a small
/// ring far from the origin keeps its sign.
pub(crate) fn signed_area2(ring: impl IntoIterator<Item = (f64, f64)>) -> f64 {
    let mut points = ring.into_iter();
    let Some((x0, y0)) = points.next() else {
        return 0.0;
    };
    let mut area2 = 0.0;
    let mut prev = (0.0, 0.0);
    for (x, y) in points {
        let cur = (x - x0, y - y0);
        area2 += prev.0 * cur.1 - cur.0 * prev.1;
        prev = cur;
    }
    area2
}

/// [`signed_area2`] of a ring of integer tile coordinates.
pub(crate) fn coords_area2(ring: &[Coord<i32>]) -> f64 {
    signed_area2(ring.iter().map(|c| (f64::from(c.x), f64::from(c.y))))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [(f64, f64); 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];

    #[test]
    fn a_ring_clockwise_with_y_down_is_positive_and_its_reverse_negative() {
        assert_eq!(signed_area2(SQUARE), 200.0);
        assert_eq!(signed_area2(SQUARE.into_iter().rev()), -200.0);
    }

    #[test]
    fn a_closing_point_adds_nothing() {
        assert_eq!(signed_area2(SQUARE.into_iter().chain([SQUARE[0]])), 200.0);
    }

    #[test]
    fn degenerate_rings_have_no_area() {
        assert_eq!(signed_area2([]), 0.0);
        assert_eq!(signed_area2([(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]), 0.0);
    }

    #[test]
    fn a_tiny_ring_far_from_the_origin_keeps_its_sign() {
        let (x, y, d) = (0.7, 0.3, 1e-9);
        let ring = [(x, y), (x + d, y), (x + d, y + d), (x, y + d)];
        assert!(signed_area2(ring) > 0.0);
        assert!(signed_area2(ring.into_iter().rev()) < 0.0);
    }
}
