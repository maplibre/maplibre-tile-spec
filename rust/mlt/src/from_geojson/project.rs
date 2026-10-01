//! Pure projection math: WGS84 lon/lat/alt into the Web Mercator unit square,
//! with the altitude mapped onto the layer's z grid.
//!
//! Everything here is deterministic and side-effect free, so it is unit tested
//! directly against known map points.

use anyhow::{Result as AnyResult, bail};
use geojson::{GeometryValue, Position};
use martin_tile_utils::{EARTH_CIRCUMFERENCE, wgs84_to_webmercator};
use mlt_core::ZStep;

/// Latitude bound of Web Mercator. Latitudes are clamped here so the projection
/// never produces infinities at the poles.
const MAX_LAT: f64 = 85.051_128_779_806_59;

/// A projected position: `x`/`y` in the unit square, `z` in grid units of the
/// layer's [`ZStep`], or `0` on a flat layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Vertex {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vertex {
    /// The point a fraction `t` of the way from `self` to `other`, z included.
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self {
            x: (other.x - self.x).mul_add(t, self.x),
            y: (other.y - self.y).mul_add(t, self.y),
            z: (other.z - self.z).mul_add(t, self.z),
        }
    }
}

/// How many positions came with and without an altitude.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Dims {
    pub flat: usize,
    pub with_z: usize,
}

/// A geometry in projected `f64` coordinates. Rings are stored as given (closed
/// or not); the clipper and the tile builder handle closing.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Geom {
    Point(Vertex),
    MultiPoint(Vec<Vertex>),
    LineString(Vec<Vertex>),
    MultiLineString(Vec<Vec<Vertex>>),
    Polygon(Vec<Vec<Vertex>>),
    MultiPolygon(Vec<Vec<Vec<Vertex>>>),
}

impl Geom {
    /// Every vertex of the geometry, in storage order.
    pub fn vertices(&self) -> Box<dyn Iterator<Item = &Vertex> + '_> {
        match self {
            Self::Point(p) => Box::new(std::iter::once(p)),
            Self::MultiPoint(ps) | Self::LineString(ps) => Box::new(ps.iter()),
            Self::MultiLineString(ls) | Self::Polygon(ls) => Box::new(ls.iter().flatten()),
            Self::MultiPolygon(ps) => Box::new(ps.iter().flatten().flatten()),
        }
    }

    /// `[min_x, min_y, max_x, max_y]` of the geometry, or `None` when it has no vertices.
    pub fn bbox(&self) -> Option<[f64; 4]> {
        self.vertices().fold(None, |acc, v| {
            Some(match acc {
                None => [v.x, v.y, v.x, v.y],
                Some([x0, y0, x1, y1]) => [x0.min(v.x), y0.min(v.y), x1.max(v.x), y1.max(v.y)],
            })
        })
    }
}

/// Projects `GeoJSON` geometries onto a layer's grid, counting the dimensions of
/// the positions it sees so the caller can check the file is all flat or all 3D.
pub(super) struct Projector {
    z_step: Option<ZStep>,
    pub dims: Dims,
}

impl Projector {
    pub fn new(z_step: Option<ZStep>) -> Self {
        Self {
            z_step,
            dims: Dims::default(),
        }
    }

    /// Project a geometry. An invalid position (see [`Self::position`]) or a
    /// `GeometryCollection`, which MLT has no geometry type for, is an error.
    pub fn project(&mut self, value: &GeometryValue) -> AnyResult<Geom> {
        Ok(match value {
            GeometryValue::Point { coordinates } => Geom::Point(self.position(coordinates)?),
            GeometryValue::MultiPoint { coordinates } => Geom::MultiPoint(self.line(coordinates)?),
            GeometryValue::LineString { coordinates } => Geom::LineString(self.line(coordinates)?),
            GeometryValue::MultiLineString { coordinates } => Geom::MultiLineString(
                coordinates
                    .iter()
                    .map(|l| self.line(l))
                    .collect::<AnyResult<_>>()?,
            ),
            GeometryValue::Polygon { coordinates } => Geom::Polygon(self.rings(coordinates)?),
            GeometryValue::MultiPolygon { coordinates } => Geom::MultiPolygon(
                coordinates
                    .iter()
                    .map(|p| self.rings(p))
                    .collect::<AnyResult<_>>()?,
            ),
            GeometryValue::GeometryCollection { .. } => {
                bail!("GeometryCollection geometries are not supported by MLT")
            }
        })
    }

    fn line(&mut self, line: &[Position]) -> AnyResult<Vec<Vertex>> {
        line.iter().map(|p| self.position(p)).collect()
    }

    fn rings(&mut self, rings: &[Vec<Position>]) -> AnyResult<Vec<Vec<Vertex>>> {
        rings.iter().map(|r| self.line(r)).collect()
    }

    /// Project one position. It must hold a longitude in `-180..=180` and a
    /// latitude in `-90..=90`. An altitude is mapped onto the z grid when the
    /// layer has one and must fit it; a fourth (`m`) value is ignored.
    fn position(&mut self, pos: &Position) -> AnyResult<Vertex> {
        let s = pos.as_slice();
        let [lon, lat, rest @ ..] = s else {
            bail!("position {s:?} has fewer than two coordinates");
        };
        if !(-180.0..=180.0).contains(lon) {
            bail!("position {s:?} has a longitude outside -180..=180");
        }
        if !(-90.0..=90.0).contains(lat) {
            bail!("position {s:?} has a latitude outside -90..=90");
        }
        let (x, y) = mercator_unit(*lon, *lat);
        let altitude = rest.first().copied();
        if altitude.is_some() {
            self.dims.with_z += 1;
        } else {
            self.dims.flat += 1;
        }
        let z = match (self.z_step, altitude) {
            (Some(step), Some(metres)) => {
                if step.z(metres).is_none() {
                    bail!(
                        "position {s:?} has an altitude outside the z grid of 10^{} m steps",
                        step.exponent()
                    );
                }
                grid_z(step, metres)
            }
            // A flat layer stores no z; a mismatch between the altitudes and
            // the step is reported by the caller once every position is counted.
            _ => 0.0,
        };
        Ok(Vertex { x, y, z })
    }
}

/// The exact (unrounded) grid value of an elevation, with the arithmetic order
/// of [`ZStep::z`] so that rounding it later lands on the same integer.
fn grid_z(step: ZStep, metres: f64) -> f64 {
    let power = 10_f64.powi(i32::from(step.exponent().unsigned_abs()));
    if step.exponent() < 0 {
        metres * power - ZStep::BASE_METRES * power
    } else {
        (metres - ZStep::BASE_METRES) / power
    }
}

/// Project lon/lat into the unit square: `x` grows east from the antimeridian,
/// `y` grows south from the northern Mercator bound, both in `0..=1`. A latitude
/// past the Mercator bound sits on the edge, as the projection has no room for it.
pub(super) fn mercator_unit(lon: f64, lat: f64) -> (f64, f64) {
    let lat = lat.clamp(-MAX_LAT, MAX_LAT);
    let (x, y) = wgs84_to_webmercator(lon, lat);
    let half = EARTH_CIRCUMFERENCE / 2.0;
    (
        ((x + half) / EARTH_CIRCUMFERENCE).clamp(0.0, 1.0),
        ((half - y) / EARTH_CIRCUMFERENCE).clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_origin_is_the_centre_of_the_unit_square() {
        let (x, y) = mercator_unit(0.0, 0.0);
        assert!(close(x, 0.5) && close(y, 0.5), "{x} {y}");
    }

    #[test]
    fn the_mercator_bounds_map_to_the_square_edges() {
        let (x, y) = mercator_unit(-180.0, MAX_LAT);
        assert!(close(x, 0.0) && close(y, 0.0), "{x} {y}");
        let (x, y) = mercator_unit(180.0, -MAX_LAT);
        assert!(close(x, 1.0) && close(y, 1.0), "{x} {y}");
    }

    #[test]
    fn latitudes_beyond_the_bounds_are_clamped() {
        let (_, north) = mercator_unit(0.0, 90.0);
        let (_, south) = mercator_unit(0.0, -90.0);
        assert!(close(north, 0.0) && close(south, 1.0), "{north} {south}");
    }

    #[test]
    fn y_grows_southwards() {
        let (_, north) = mercator_unit(0.0, 60.0);
        let (_, south) = mercator_unit(0.0, -60.0);
        assert!(north < 0.5 && 0.5 < south);
        assert!(close(north, 1.0 - south));
    }

    #[test]
    fn the_white_house_lands_in_its_z14_tile() {
        let (x, y) = mercator_unit(-77.036_560, 38.897_957);
        let scale = f64::from(1_u32 << 14) * 4096.0;
        let (wx, wy) = (x * scale, y * scale);
        assert_eq!((wx / 4096.0).floor(), 4685.0);
        assert_eq!((wy / 4096.0).floor(), 6267.0);
        assert_eq!((wx - 4685.0 * 4096.0).round(), 4016.0);
        assert_eq!((wy - 6267.0 * 4096.0).round(), 2438.0);
    }

    #[test]
    fn lerp_interpolates_all_three_axes() {
        let a = Vertex {
            x: 0.0,
            y: 10.0,
            z: 100.0,
        };
        let b = Vertex {
            x: 4.0,
            y: 2.0,
            z: 300.0,
        };
        assert_eq!(
            a.lerp(b, 0.25),
            Vertex {
                x: 1.0,
                y: 8.0,
                z: 150.0
            }
        );
    }

    /// Parse a geometry the way the command does, through `GeoJson::from_str`,
    /// which accepts positions that serde's `Geometry` deserializer rejects.
    fn geometry(json: &str) -> GeometryValue {
        match json.parse::<geojson::GeoJson>().unwrap() {
            geojson::GeoJson::Geometry(g) => g.value,
            geojson::GeoJson::Feature(_) | geojson::GeoJson::FeatureCollection(_) => {
                panic!("a geometry")
            }
        }
    }

    fn step(exponent: i8) -> Option<ZStep> {
        ZStep::new(exponent).ok()
    }

    #[test]
    fn dims_count_flat_and_z_positions() {
        let mut projector = Projector::new(None);
        projector
            .project(&geometry(r#"{"type":"Point","coordinates":[1,2]}"#))
            .unwrap();
        projector
            .project(&geometry(
                r#"{"type":"LineString","coordinates":[[1,2,3],[4,5,6]]}"#,
            ))
            .unwrap();
        projector
            .project(&geometry(
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#,
            ))
            .unwrap();
        assert_eq!(projector.dims, Dims { flat: 5, with_z: 2 });
    }

    fn point_z(z_step: Option<ZStep>, json: &str) -> f64 {
        let Geom::Point(p) = Projector::new(z_step).project(&geometry(json)).unwrap() else {
            panic!("a point")
        };
        p.z
    }

    #[test]
    fn an_altitude_is_mapped_onto_the_z_grid_and_an_m_value_is_ignored() {
        // Terrain-RGB's decimetre grid: 12.3 m is (12.3 + 10000) * 10.
        assert!(close(
            point_z(step(-1), r#"{"type":"Point","coordinates":[0,0,12.3,7]}"#),
            100_123.0
        ));
        assert!(close(
            point_z(step(2), r#"{"type":"Point","coordinates":[0,0,250]}"#),
            102.5
        ));
    }

    #[test]
    fn without_a_z_step_or_an_altitude_z_is_zero() {
        assert_eq!(
            point_z(None, r#"{"type":"Point","coordinates":[0,0,12.3]}"#),
            0.0
        );
        assert_eq!(
            point_z(step(0), r#"{"type":"Point","coordinates":[0,0]}"#),
            0.0
        );
    }

    #[test]
    fn invalid_positions_are_rejected() {
        for (json, message) in [
            (
                r#"{"type":"LineString","coordinates":[[0,0],[139.7]]}"#,
                "position [139.7] has fewer than two coordinates",
            ),
            (
                r#"{"type":"LineString","coordinates":[[0,0],[181.5,10]]}"#,
                "position [181.5, 10.0] has a longitude outside -180..=180",
            ),
            (
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,-91],[0,0]]]}"#,
                "position [1.0, -91.0] has a latitude outside -90..=90",
            ),
            (
                r#"{"type":"Point","coordinates":[0,0,1e12]}"#,
                "position [0.0, 0.0, 1000000000000.0] has an altitude outside the z grid of 10^-3 m steps",
            ),
        ] {
            let err = Projector::new(step(-3))
                .project(&geometry(json))
                .unwrap_err();
            assert_eq!(err.to_string(), message);
        }
    }

    #[test]
    fn the_antimeridian_and_the_poles_are_accepted() {
        let Geom::LineString(line) = Projector::new(None)
            .project(&geometry(
                r#"{"type":"LineString","coordinates":[[-180,90],[180,-90]]}"#,
            ))
            .unwrap()
        else {
            panic!("a line")
        };
        assert!(close(line[0].x, 0.0) && close(line[0].y, 0.0), "{line:?}");
        assert!(close(line[1].x, 1.0) && close(line[1].y, 1.0), "{line:?}");
    }

    #[test]
    fn a_geometry_collection_does_not_project() {
        let err = Projector::new(None)
            .project(&geometry(
                r#"{"type":"GeometryCollection","geometries":[]}"#,
            ))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "GeometryCollection geometries are not supported by MLT"
        );
    }

    #[test]
    fn bbox_spans_every_ring_of_a_multi_polygon() {
        let g = Projector::new(None)
            .project(&geometry(
                r#"{"type":"MultiPolygon","coordinates":[
                    [[[0,0],[10,0],[10,10],[0,0]]],
                    [[[-20,-20],[-10,-20],[-10,-10],[-20,-20]]]
                ]}"#,
            ))
            .unwrap();
        let [x0, y0, x1, y1] = g.bbox().unwrap();
        assert!(
            close(x0, 160.0 / 360.0) && close(x1, 190.0 / 360.0),
            "{x0} {x1}"
        );
        assert!(y0 < 0.5 && y1 > 0.5);
        assert_eq!(Geom::MultiPoint(vec![]).bbox(), None);
    }
}
