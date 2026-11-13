//! Planar polygons with holes, as `CityGML` stores surfaces, cut into triangles.

use martin_tile_utils::EARTH_CIRCUMFERENCE;

/// Below this length of the summed cross products, in squared metres, a ring has no area.
const MIN_NORMAL: f64 = 1e-9;

/// Metres per degree of latitude, on a sphere of the WGS84 semi-major axis.
const METRES_PER_DEGREE: f64 = EARTH_CIRCUMFERENCE / 360.0;

/// Cuts the polygon whose rings are `points[..ends[0]]`, `points[ends[0]..ends[1]]` and so on,
/// the exterior first, into triangles appended to `out` as indices into `points`.
/// `lat_lon` says the points are latitude and longitude in degrees and metres up, rather than
/// metres east, north and up.
///
/// The triangles wind like the exterior ring. Returns the polygon's normal in metres east,
/// north and up, or `None`, adding nothing, if the exterior has no area or the points are more
/// than a `u32` indexes.
pub fn polygon(
    points: &[[f64; 3]],
    ends: &[usize],
    lat_lon: bool,
    out: &mut Vec<[u32; 3]>,
) -> Option<[f64; 3]> {
    let exterior = *ends.first()?;
    let local: Vec<[f64; 3]>;
    let points = if lat_lon {
        // Near enough to metres for a facing and a plane to cut in.
        let [lat0, lon0, _] = *points.first()?;
        let east = METRES_PER_DEGREE * lat0.to_radians().cos();
        local = points
            .iter()
            .map(|&[lat, lon, up]| [(lon - lon0) * east, (lat - lat0) * METRES_PER_DEGREE, up])
            .collect();
        &local
    } else {
        points
    };
    let origin = *points.first()?;
    let normal = newell(&points[..exterior], origin);
    if dot(normal, normal) < MIN_NORMAL * MIN_NORMAL {
        return None;
    }
    let abs = normal.map(f64::abs);
    // The two axes that remain when the normal's largest component is dropped.
    let (across, along) = if abs[2] >= abs[0] && abs[2] >= abs[1] {
        (0, 1)
    } else if abs[0] >= abs[1] {
        (1, 2)
    } else {
        (2, 0)
    };
    let flat: Vec<f64> = points
        .iter()
        .flat_map(|p| [p[across] - origin[across], p[along] - origin[along]])
        .collect();
    let indices = earcutr::earcut(&flat, &ends[..ends.len() - 1], 2).ok()?;
    let start = out.len();
    for t in indices.as_chunks::<3>().0 {
        let [Ok(a), Ok(b), Ok(c)] = t.map(u32::try_from) else {
            out.truncate(start);
            return None;
        };
        let facing = dot(
            cross(
                sub(points[t[1]], points[t[0]]),
                sub(points[t[2]], points[t[0]]),
            ),
            normal,
        );
        out.push(if facing < 0.0 { [a, c, b] } else { [a, b, c] });
    }
    (out.len() > start).then_some(normal)
}

/// The ring's normal by Newell's method, relative to `origin` for precision with large coordinates.
fn newell(ring: &[[f64; 3]], origin: [f64; 3]) -> [f64; 3] {
    let mut n = [0.0; 3];
    for (i, &p) in ring.iter().enumerate() {
        let a = sub(p, origin);
        let b = sub(ring[(i + 1) % ring.len()], origin);
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    n
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[[f64; 3]], triangles: &[[u32; 3]], normal: [f64; 3]) -> f64 {
        let len = dot(normal, normal).sqrt();
        triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| points[i as usize]);
                dot(cross(sub(b, a), sub(c, a)), normal) / len / 2.0
            })
            .sum()
    }

    #[test]
    fn a_vertical_wall_with_a_window_hole_keeps_its_area_and_faces_its_normal() {
        // A 10 x 5 m wall in the x-z plane of a UTM-sized coordinate, with a 2 x 1 m hole.
        let (x, y) = (570_000.0, 5_936_000.0);
        let points = [
            [x, y, 0.0],
            [x + 10.0, y, 0.0],
            [x + 10.0, y, 5.0],
            [x, y, 5.0],
            [x + 2.0, y, 2.0],
            [x + 2.0, y, 3.0],
            [x + 4.0, y, 3.0],
            [x + 4.0, y, 2.0],
        ];
        let mut out = Vec::new();
        let normal = polygon(&points, &[4, 8], false, &mut out).unwrap();
        assert!(normal[1] < 0.0, "{normal:?}");
        assert!((area(&points, &out, normal) - 48.0).abs() < 1e-6);
        assert!(out.iter().flatten().any(|&i| i >= 4));
    }

    #[test]
    fn a_ring_without_area_is_skipped() {
        let points = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]];
        let mut out = Vec::new();
        assert!(polygon(&points, &[3], false, &mut out).is_none());
        assert!(polygon(&points, &[], false, &mut out).is_none());
        assert_eq!(out, [] as [[u32; 3]; 0]);
    }

    #[test]
    fn degrees_are_measured_in_metres_east_north_and_up() {
        // A flat square metre, counter-clockwise from above, latitude first.
        let (lat, lon): (f64, f64) = (35.42, 136.76);
        let (north, east) = (
            1.0 / METRES_PER_DEGREE,
            1.0 / (METRES_PER_DEGREE * lat.to_radians().cos()),
        );
        let points = [
            [lat, lon, 10.0],
            [lat, lon + east, 10.0],
            [lat + north, lon + east, 10.0],
            [lat + north, lon, 10.0],
        ];
        let mut out = Vec::new();
        let normal = polygon(&points, &[4], true, &mut out).unwrap();
        assert!((normal[2] - 2.0).abs() < 1e-6, "{normal:?}");
        assert_eq!(out.len(), 2);
        // Read as metres, the same square has no area.
        assert!(polygon(&points, &[4], false, &mut out).is_none());
    }
}
