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

/// Whether positions carry an altitude. An MLT layer is either flat or 3D, so
/// every position of a file must agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dims {
    Xy,
    Xyz,
}

impl Dims {
    /// The dimensions of a position: a third value is an altitude.
    fn of(pos: &[f64]) -> Self {
        if pos.len() > 2 { Self::Xyz } else { Self::Xy }
    }
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

/// Projects `GeoJSON` geometries onto a layer's grid, rejecting a position whose
/// dimensions differ from the first one's so the file is all flat or all 3D.
pub(super) struct Projector {
    z_step: Option<ZStep>,
    /// The first position projected, `None` before any. Every later position
    /// must have its dimensions, and it is quoted when one does not.
    first: Option<Vec<f64>>,
}

impl Projector {
    pub fn new(z_step: Option<ZStep>) -> Self {
        Self {
            z_step,
            first: None,
        }
    }

    /// The dimensions every projected position shares, `None` before any.
    pub fn dims(&self) -> Option<Dims> {
        self.first.as_deref().map(Dims::of)
    }

    /// Project a geometry. An invalid position (see [`Self::project_position`]), a
    /// segment that crosses the antimeridian (see [`reject_antimeridian_crossing`]),
    /// or a `GeometryCollection`, which MLT has no geometry type for, is an error.
    pub fn project(&mut self, value: &GeometryValue) -> AnyResult<Geom> {
        Ok(match value {
            GeometryValue::Point { coordinates } => {
                Geom::Point(self.project_position(coordinates)?)
            }
            GeometryValue::MultiPoint { coordinates } => Geom::MultiPoint(
                coordinates
                    .iter()
                    .map(|p| self.project_position(p))
                    .collect::<AnyResult<_>>()?,
            ),
            GeometryValue::LineString { coordinates } => {
                Geom::LineString(self.project_line(coordinates)?)
            }
            GeometryValue::MultiLineString { coordinates } => Geom::MultiLineString(
                coordinates
                    .iter()
                    .map(|l| self.project_line(l))
                    .collect::<AnyResult<_>>()?,
            ),
            GeometryValue::Polygon { coordinates } => {
                Geom::Polygon(self.project_rings(coordinates)?)
            }
            GeometryValue::MultiPolygon { coordinates } => Geom::MultiPolygon(
                coordinates
                    .iter()
                    .map(|p| self.project_rings(p))
                    .collect::<AnyResult<_>>()?,
            ),
            GeometryValue::GeometryCollection { .. } => {
                bail!("GeometryCollection geometries are not supported by MLT")
            }
        })
    }

    /// Project the positions of a line string, whose consecutive positions are
    /// joined by segments.
    fn project_line(&mut self, line: &[Position]) -> AnyResult<Vec<Vertex>> {
        let vertices = line
            .iter()
            .map(|p| self.project_position(p))
            .collect::<AnyResult<_>>()?;
        for segment in line.windows(2) {
            reject_antimeridian_crossing(&segment[0], &segment[1])?;
        }
        Ok(vertices)
    }

    /// Project the rings of a polygon. A ring that is not closed is joined back
    /// to its first position, so that closing segment is checked too.
    fn project_rings(&mut self, rings: &[Vec<Position>]) -> AnyResult<Vec<Vec<Vertex>>> {
        rings
            .iter()
            .map(|r| {
                let vertices = self.project_line(r)?;
                if let (Some(last), Some(first)) = (r.last(), r.first()) {
                    reject_antimeridian_crossing(last, first)?;
                }
                Ok(vertices)
            })
            .collect()
    }

    /// Project one position. It must hold a longitude in `-180..=180` and a
    /// latitude in `-90..=90`, and have an altitude exactly when the first
    /// position did. An altitude is mapped onto the z grid when the layer has
    /// one and must fit it. Values past the third, such as a historical `m`,
    /// are ignored.
    fn project_position(&mut self, pos: &Position) -> AnyResult<Vertex> {
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
        let dims = Dims::of(s);
        let first = self.first.get_or_insert_with(|| s.to_vec());
        if Dims::of(first) != dims {
            let (this, that) = match dims {
                Dims::Xy => ("has no altitude", "has one"),
                Dims::Xyz => ("has an altitude", "has none"),
            };
            bail!(
                "position {s:?} {this}, but the first position {first:?} {that}; an MLT layer is \
                 either flat or 3D, so make them all [lon, lat] or all [lon, lat, alt]"
            );
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
            // the step is reported by the caller after projection.
            _ => 0.0,
        };
        Ok(Vertex { x, y, z })
    }
}

/// Reject a segment spanning over 180 degrees of longitude: it almost surely means
/// to cross the antimeridian, which RFC 7946 section 3.1.9 says to split. One with
/// both ends at -180 or 180 runs along the map edge and is accepted. The positions
/// must already be validated by [`Projector::project_position`].
#[expect(
    clippy::float_cmp,
    reason = "only a longitude of exactly -180 or 180 lies on the antimeridian"
)]
fn reject_antimeridian_crossing(a: &Position, b: &Position) -> AnyResult<()> {
    let (a, b) = (a.as_slice(), b.as_slice());
    let on_antimeridian = |lon: f64| lon.abs() == 180.0;
    if (a[0] - b[0]).abs() > 180.0 && !(on_antimeridian(a[0]) && on_antimeridian(b[0])) {
        bail!(
            "segment from {a:?} to {b:?} spans more than 180 degrees of longitude, so it \
             crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section \
             3.1.9)"
        );
    }
    Ok(())
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
    fn dims_are_set_by_the_first_position() {
        let mut projector = Projector::new(None);
        assert_eq!(projector.dims(), None);
        projector
            .project(&geometry(
                r#"{"type":"LineString","coordinates":[[1,2,3],[4,5,6]]}"#,
            ))
            .unwrap();
        assert_eq!(projector.dims(), Some(Dims::Xyz));

        let mut projector = Projector::new(None);
        projector
            .project(&geometry(
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#,
            ))
            .unwrap();
        assert_eq!(projector.dims(), Some(Dims::Xy));
    }

    #[test]
    fn mixing_flat_and_3d_positions_is_rejected() {
        let mut projector = Projector::new(None);
        projector
            .project(&geometry(r#"{"type":"Point","coordinates":[1,2]}"#))
            .unwrap();
        let err = projector
            .project(&geometry(
                r#"{"type":"LineString","coordinates":[[1,2,3],[4,5,6]]}"#,
            ))
            .unwrap_err();
        insta::assert_snapshot!(err, @"position [1.0, 2.0, 3.0] has an altitude, but the first position [1.0, 2.0] has none; an MLT layer is either flat or 3D, so make them all [lon, lat] or all [lon, lat, alt]");

        // Within one geometry too.
        let err = Projector::new(None)
            .project(&geometry(
                r#"{"type":"LineString","coordinates":[[1,2,3],[4,5]]}"#,
            ))
            .unwrap_err();
        insta::assert_snapshot!(err, @"position [4.0, 5.0] has no altitude, but the first position [1.0, 2.0, 3.0] has one; an MLT layer is either flat or 3D, so make them all [lon, lat] or all [lon, lat, alt]");
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
        let errors: Vec<String> = [
            r#"{"type":"LineString","coordinates":[[0,0],[139.7]]}"#,
            r#"{"type":"LineString","coordinates":[[0,0],[181.5,10]]}"#,
            r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,-91],[0,0]]]}"#,
            r#"{"type":"Point","coordinates":[0,0,1e12]}"#,
        ]
        .into_iter()
        .map(|json| {
            Projector::new(step(-3))
                .project(&geometry(json))
                .unwrap_err()
                .to_string()
        })
        .collect();
        insta::assert_snapshot!(errors.join("\n"), @"
        position [139.7] has fewer than two coordinates
        position [181.5, 10.0] has a longitude outside -180..=180
        position [1.0, -91.0] has a latitude outside -90..=90
        position [0.0, 0.0, 1000000000000.0] has an altitude outside the z grid of 10^-3 m steps
        ");
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
    fn a_segment_crossing_the_antimeridian_is_rejected() {
        let errors: Vec<String> = [
            r#"{"type":"LineString","coordinates":[[179,10],[-179,10]]}"#,
            // One end on the antimeridian: the short way still crosses it.
            r#"{"type":"LineString","coordinates":[[-180,0],[170,0]]}"#,
            r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]],[[[179,0],[-179,0],[-179,1],[179,0]]]]}"#,
            r#"{"type":"MultiLineString","coordinates":[[[0,0],[1,0]],[[-170,0],[170,0]]]}"#,
            // The implicit closing segment of an unclosed ring.
            r#"{"type":"Polygon","coordinates":[[[-179,0],[-90,0],[0,0],[90,0],[179,1]]]}"#,
        ]
        .into_iter()
        .map(|json| {
            Projector::new(None)
                .project(&geometry(json))
                .unwrap_err()
                .to_string()
        })
        .collect();
        insta::assert_snapshot!(errors.join("\n"), @"
        segment from [179.0, 10.0] to [-179.0, 10.0] spans more than 180 degrees of longitude, so it crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section 3.1.9)
        segment from [-180.0, 0.0] to [170.0, 0.0] spans more than 180 degrees of longitude, so it crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section 3.1.9)
        segment from [179.0, 0.0] to [-179.0, 0.0] spans more than 180 degrees of longitude, so it crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section 3.1.9)
        segment from [-170.0, 0.0] to [170.0, 0.0] spans more than 180 degrees of longitude, so it crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section 3.1.9)
        segment from [179.0, 1.0] to [-179.0, 0.0] spans more than 180 degrees of longitude, so it crosses the antimeridian; split the geometry at longitude 180 (RFC 7946 section 3.1.9)
        ");
    }

    #[test]
    fn segments_spanning_up_to_180_degrees_or_along_the_map_edge_are_accepted() {
        for json in [
            r#"{"type":"LineString","coordinates":[[-90,0],[90,0]]}"#,
            // The world bounding box, and the edge Natural Earth's Antarctica runs along.
            r#"{"type":"Polygon","coordinates":[[[-180,-90],[180,-90],[180,90],[-180,90],[-180,-90]]]}"#,
            r#"{"type":"LineString","coordinates":[[180,-85],[180,-90],[-180,-90],[-180,-85]]}"#,
            r#"{"type":"LineString","coordinates":[[-180,0],[180,0]]}"#,
            r#"{"type":"Polygon","coordinates":[[[-180,0],[0,0],[180,0],[0,10],[-180,0]]]}"#,
            r#"{"type":"MultiPoint","coordinates":[[179,10],[-179,10]]}"#,
        ] {
            Projector::new(None).project(&geometry(json)).unwrap();
        }
    }

    #[test]
    fn a_geometry_collection_does_not_project() {
        let err = Projector::new(None)
            .project(&geometry(
                r#"{"type":"GeometryCollection","geometries":[]}"#,
            ))
            .unwrap_err();
        insta::assert_snapshot!(err, @"GeometryCollection geometries are not supported by MLT");
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
