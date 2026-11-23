//! Planar polygons with holes, as `CityGML` stores surfaces, cut into triangles.

use martin_tile_utils::EARTH_CIRCUMFERENCE;

/// Below this length of the summed cross products, in squared metres, a ring has no area.
const MIN_NORMAL: f64 = 1e-9;

/// Metres per degree of latitude, on a sphere of the WGS84 semi-major axis.
const METRES_PER_DEGREE: f64 = EARTH_CIRCUMFERENCE / 360.0;

/// The scratch buffers that cutting polygons into triangles reuses.
#[derive(Default)]
pub struct Triangulator {
    earcut: earcut::Earcut<f64>,
    indices: Vec<u32>,
    holes: Vec<u32>,
    local: Vec<[f64; 3]>,
}

impl Triangulator {
    /// Cuts the polygon whose rings end at `ends` in `points`, the exterior first, into triangles appended to `out`.
    /// `lat_lon` says the points are latitude and longitude in degrees and metres up, rather than metres east, north and up.
    /// The triangles wind like the exterior ring.
    /// Returns the polygon's normal in metres east, north and up.
    /// `None`, adding nothing, if the exterior has no area or the points are more than a `u32` indexes.
    pub fn polygon(
        &mut self,
        points: &[[f64; 3]],
        ends: &[usize],
        lat_lon: bool,
        out: &mut Vec<[u32; 3]>,
    ) -> Option<[f64; 3]> {
        let exterior = *ends.first()?;
        let points = if lat_lon {
            let [lat0, lon0, _] = *points.first()?;
            let east = METRES_PER_DEGREE * lat0.to_radians().cos();
            self.local.clear();
            self.local.extend(points.iter().map(|&[lat, lon, up]| {
                [(lon - lon0) * east, (lat - lat0) * METRES_PER_DEGREE, up]
            }));
            &self.local
        } else {
            points
        };
        let origin = *points.first()?;
        let normal = newell(&points[..exterior], origin);
        if dot(normal, normal) < MIN_NORMAL * MIN_NORMAL {
            return None;
        }
        let abs = normal.map(f64::abs);
        // The two axes left in right-handed order when the normal's largest component is dropped.
        let (across, along, dropped) = if abs[2] >= abs[0] && abs[2] >= abs[1] {
            (0, 1, 2)
        } else if abs[0] >= abs[1] {
            (1, 2, 0)
        } else {
            (2, 0, 1)
        };
        let flat = points
            .iter()
            .map(|p| [p[across] - origin[across], p[along] - origin[along]]);
        self.holes.clear();
        for &e in &ends[..ends.len() - 1] {
            self.holes.push(u32::try_from(e).ok()?);
        }
        self.earcut.earcut(flat, &self.holes, &mut self.indices);
        let start = out.len();
        // Each triangle winds like the polygon's normal along the dropped axis.
        for &[a, b, c] in self.indices.as_chunks::<3>().0 {
            let [pa, pb, pc] = [a, b, c].map(|i| points[i as usize]);
            let doubled_area = (pb[across] - pa[across]) * (pc[along] - pa[along])
                - (pb[along] - pa[along]) * (pc[across] - pa[across]);
            out.push(if doubled_area * normal[dropped] < 0.0 {
                [a, c, b]
            } else {
                [a, b, c]
            });
        }
        (out.len() > start).then_some(normal)
    }
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

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
