//! What a client pays for a layer: fetching its bytes, and getting the triangles of its polygons.

use usize_cast::{FromUsize as _, IntoUsize as _};

use crate::decoder::{GeometryType, GeometryValues};
use crate::encoder::{EncoderConfig, Network};

/// Nanoseconds a client spends decoding one stored triangle.
const DECODE_NS_PER_TRIANGLE: u64 = 20;

/// How many times slower than a Ryzen 9 3900 core the client runs.
const CLIENT_SLOWDOWN: u64 = 4;

/// What a layer costs a client, in nanoseconds of a Ryzen 9 3900 core, and its stored bytes to break a tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost {
    ns: u64,
    bytes: usize,
}

/// Prices layer encodings for the client an [`EncoderConfig`] targets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientCost {
    ns_per_byte: u64,
}

impl ClientCost {
    pub(crate) fn new(cfg: EncoderConfig) -> Self {
        let mbit_per_s = match cfg.network() {
            Network::Offline => return Self { ns_per_byte: 0 },
            Network::Cellular3G => 2,
            Network::Cellular4G => 20,
            Network::Cellular5G => 200,
        };
        Self {
            ns_per_byte: 8_000 / mbit_per_s / CLIENT_SLOWDOWN,
        }
    }

    /// The price of a layer whose staged `geometry` says how the client gets its triangles.
    pub(crate) fn price(self, geometry: &GeometryValues) -> Price {
        Price {
            ns_per_byte: self.ns_per_byte,
            triangles_ns: triangles_ns(geometry),
        }
    }
}

/// What one staged layer costs a client per stored byte, on top of getting its triangles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Price {
    ns_per_byte: u64,
    triangles_ns: u64,
}

impl Price {
    /// The cost of the layer stored in `bytes`.
    pub(crate) fn at(self, bytes: usize) -> Cost {
        Cost {
            ns: self.ns_per_byte * u64::from_usize(bytes) + self.triangles_ns,
            bytes,
        }
    }
}

/// The nanoseconds the client spends on the triangles of `geometry`: decoding the stored ones, or cutting its own.
fn triangles_ns(geometry: &GeometryValues) -> u64 {
    if geometry.triangle_offsets.is_some() {
        let indices = geometry.index_buffer.as_ref().map_or(0, Vec::len);
        DECODE_NS_PER_TRIANGLE * u64::from_usize(indices / 3)
    } else {
        earcut_ns_of_layer(geometry)
    }
}

/// Nanoseconds `mapbox::earcut` takes on a Ryzen 9 3900 core for one polygon.
///
/// `n` counts its vertices as a client holds them, with each ring's closing point.
/// Fitted per polygon over five basemap and building tilesets, within ±10% on the median layer.
/// Rounding each polygon to whole nanoseconds keeps a layer's sum the same in every feature order.
fn earcut_ns(n: usize, holes: usize) -> u64 {
    #[expect(clippy::cast_precision_loss, reason = "an estimate")]
    let (n, holes) = (n as f64, holes as f64);
    let ns = 135.0 + n * (2.04 * n.min(80.0) - 15.7) + 6.9 * n * holes;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fit is above 100 ns for every n"
    )]
    let ns = ns.round() as u64;
    ns
}

/// The earcut nanoseconds of every polygon in `geometry`.
///
/// Staging gives a multi geometry one geometry offset and a line one ring, so the walk steps through them in feature order.
fn earcut_ns_of_layer(geometry: &GeometryValues) -> u64 {
    let (Some(parts), Some(rings)) = (&geometry.part_offsets, &geometry.ring_offsets) else {
        return 0;
    };
    let geoms = geometry.geometry_offsets.as_deref().unwrap_or(&[]);
    let span = |level: &[u32], at: usize| -> usize {
        match (level.get(at), level.get(at + 1)) {
            (Some(&start), Some(&end)) => end.saturating_sub(start).into_usize(),
            _ => 0,
        }
    };
    let (mut geom, mut part, mut ring) = (0, 0, 0);
    let mut total = 0;
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
            total += earcut_ns(vertices + ring_count, ring_count.saturating_sub(1));
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

    use super::{ClientCost, Cost, triangles_ns};
    use crate::decoder::GeometryValues;
    use crate::encoder::{EncoderConfig, Network};

    fn square(x: i32, size: i32) -> LineString<i32> {
        LineString::from(vec![
            (x, x),
            (x + size, x),
            (x + size, x + size),
            (x, x + size),
            (x, x),
        ])
    }

    fn staged(geoms: &[Geometry<i32>], mut geometry: GeometryValues) -> GeometryValues {
        for geom in geoms {
            geometry.push_geom(geom);
        }
        geometry
    }

    fn saving(geoms: &[Geometry<i32>]) -> String {
        let plain = triangles_ns(&staged(geoms, GeometryValues::default()));
        let tessellated = triangles_ns(&staged(geoms, GeometryValues::new_tessellated()));
        format!("{}", i128::from(plain) - i128::from(tessellated))
    }

    #[test]
    fn a_square_saves_its_earcut_less_two_triangle_decodes() {
        assert_snapshot!(saving(&[Geometry::Polygon(Polygon::new(square(0, 10), vec![]))]), @"68");
    }

    #[test]
    fn a_multi_polygon_saves_the_earcut_of_each_part() {
        let parts = vec![
            Polygon::new(square(0, 10), vec![]),
            Polygon::new(square(20, 10), vec![]),
        ];
        assert_snapshot!(saving(&[Geometry::MultiPolygon(MultiPolygon(parts))]), @"136");
    }

    #[test]
    fn a_hole_costs_a_scan_of_the_polygon() {
        let holed = Polygon::new(square(0, 30), vec![square(10, 10)]);
        assert_snapshot!(saving(&[Geometry::Polygon(holed)]), @"91");
    }

    #[test]
    fn points_and_lines_save_nothing() {
        let line = Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)]));
        assert_snapshot!(saving(&[Geometry::Point(Point::new(1, 2)), line]), @"0");
    }

    #[test]
    fn a_line_beside_a_square_saves_only_the_square() {
        let line = Geometry::LineString(LineString::from(vec![(0, 0), (5, 5), (9, 0)]));
        let square = Geometry::Polygon(Polygon::new(square(0, 10), vec![]));
        assert_snapshot!(saving(&[line, square]), @"68");
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
        ]), @"295");
    }

    #[test]
    fn a_stored_byte_costs_its_transfer_on_a_client_4x_slower_than_the_fit() {
        let rows: Vec<String> = [
            Network::Offline,
            Network::Cellular3G,
            Network::Cellular4G,
            Network::Cellular5G,
        ]
        .into_iter()
        .map(|network| {
            let client = ClientCost::new(EncoderConfig::default().with_network(network));
            let price = client.price(&GeometryValues::default());
            format!("{network:?}: {:?}", price.at(1))
        })
        .collect();
        assert_snapshot!(rows.join("\n"), @"
        Offline: Cost { ns: 0, bytes: 1 }
        Cellular3G: Cost { ns: 1000, bytes: 1 }
        Cellular4G: Cost { ns: 100, bytes: 1 }
        Cellular5G: Cost { ns: 10, bytes: 1 }
        ");
    }

    #[test]
    fn equal_nanoseconds_rank_by_fewer_bytes() {
        let offline = ClientCost::new(EncoderConfig::default().with_network(Network::Offline));
        let price = offline.price(&GeometryValues::default());
        let cheaper: Cost = price.at(10);
        assert!(cheaper < price.at(11));
    }
}
