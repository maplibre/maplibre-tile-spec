//! Triangulation of one planar-ish face with holes, keeping the face's winding.

use earcut::Earcut;

/// The face normal by Newell's method, which tolerates slightly non-planar rings.
fn newell(ring: &[[f64; 3]]) -> [f64; 3] {
    let mut n = [0.0; 3];
    for (i, a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    n
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ]
}

/// Whether a ring encloses no area, measured against the square of its extent.
/// `earcut` can loop forever on such a ring.
fn is_flat(ring: &[[f64; 3]], normal: [f64; 3]) -> bool {
    let mut span = 0.0_f64;
    for axis in 0..3 {
        let (lo, hi) = ring
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v[axis]), hi.max(v[axis]))
            });
        span = span.max(hi - lo);
    }
    dot(normal, normal).sqrt() <= 1e-9 * span * span
}

/// Triangles of the face with these `rings`, exterior first, in a right-handed metric frame.
///
/// Corners index into the rings laid end to end.
/// Each triangle winds as the exterior ring does, so it faces the same way.
/// A face or hole enclosing no area is left out.
pub(super) fn triangulate(rings: &[Vec<[f64; 3]>]) -> Vec<[usize; 3]> {
    let Some(exterior) = rings.first() else {
        return Vec::new();
    };
    let normal = newell(exterior);
    if exterior.len() < 3 || is_flat(exterior, normal) {
        return Vec::new();
    }
    let drop = (0..3)
        .max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
        .expect("three axes");
    let keep = [(drop + 1) % 3, (drop + 2) % 3];

    let mut points = Vec::new();
    let mut original = Vec::new();
    let mut holes = Vec::new();
    let mut start = 0;
    for (r, ring) in rings.iter().enumerate() {
        let skip = r > 0 && (ring.len() < 3 || is_flat(ring, newell(ring)));
        if !skip {
            if r > 0 {
                holes.push(points.len());
            }
            for (i, v) in ring.iter().enumerate() {
                points.push([v[keep[0]], v[keep[1]]]);
                original.push(start + i);
            }
        }
        start += ring.len();
    }
    let flat: Vec<[f64; 3]> = rings.iter().flatten().copied().collect();
    let mut indices = Vec::new();
    Earcut::new().earcut(points, &holes, &mut indices);
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| original[i]);
            let facing = dot(cross(sub(flat[b], flat[a]), sub(flat[c], flat[a])), normal);
            if facing < 0.0 { [a, c, b] } else { [a, b, c] }
        })
        .collect()
}
