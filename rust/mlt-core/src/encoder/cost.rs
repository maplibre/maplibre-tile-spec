//! What a client pays for a layer: fetching its bytes, and the work it does on them before drawing.

use usize_cast::FromUsize as _;

use crate::decoder::GeometryValues;
use crate::encoder::source::LayerSource;
use crate::encoder::{EncoderConfig, Network};
use crate::tile::stored_len;

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

/// Decoding the stored triangles of the polygons.
impl ClientWork for GeometryValues {
    fn client_ns(&self) -> u64 {
        let indices = self.index_buffer.as_ref().map_or(0, Vec::len);
        DECODE_NS_PER_TRIANGLE * u64::from_usize(indices / 3)
    }
}

/// Cutting every polygon of a layer into triangles with `mapbox::earcut`, which no feature order changes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Earcut {
    ns: u64,
}

impl Earcut {
    pub(crate) fn of(source: &impl LayerSource) -> Self {
        let ns = (0..source.feature_count())
            .flat_map(|feature| source.polygons(feature))
            .map(|rings| {
                let (mut vertices, mut ring_count) = (0, 0);
                for ring in rings {
                    vertices += stored_len(ring);
                    ring_count += 1;
                }
                polygon_earcut_ns(vertices + ring_count, ring_count.saturating_sub(1))
            })
            .sum();
        Self { ns }
    }
}

impl ClientWork for Earcut {
    fn client_ns(&self) -> u64 {
        self.ns
    }
}

/// Nanoseconds `mapbox::earcut` takes on a Ryzen 9 3900 core for one polygon.
///
/// `n` counts its vertices as a client holds them, with each ring's closing point.
/// Fitted per polygon over five basemap and building tilesets, within ±10% on the median layer.
fn polygon_earcut_ns(n: usize, holes: usize) -> u64 {
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
