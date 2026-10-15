//! What a client pays for a layer: fetching its bytes, and the work it does on them before drawing.

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

    /// The price of a layer on which the client does `work`.
    pub(crate) fn price(self, work: &impl ClientWork) -> Price {
        Price {
            ns_per_byte: self.ns_per_byte,
            work_ns: work.client_ns(),
        }
    }
}

/// Work a client does on a layer after fetching it and before drawing it.
pub(crate) trait ClientWork {
    /// The nanoseconds this work takes on a Ryzen 9 3900 core.
    fn client_ns(&self) -> u64;
}

/// Getting the triangles of the polygons: decoding the stored ones, or cutting its own.
impl ClientWork for GeometryValues {
    fn client_ns(&self) -> u64 {
        if self.triangle_offsets.is_some() {
            let indices = self.index_buffer.as_ref().map_or(0, Vec::len);
            DECODE_NS_PER_TRIANGLE * u64::from_usize(indices / 3)
        } else {
            earcut_ns_of_layer(self)
        }
    }
}

/// What one layer costs a client per stored byte, on top of the work it does on them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Price {
    ns_per_byte: u64,
    work_ns: u64,
}

impl Price {
    /// The cost of the layer stored in `bytes`.
    pub(crate) fn at(self, bytes: usize) -> Cost {
        Cost {
            ns: self.ns_per_byte * u64::from_usize(bytes) + self.work_ns,
            bytes,
        }
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
