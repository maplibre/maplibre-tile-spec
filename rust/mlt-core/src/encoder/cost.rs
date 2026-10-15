//! What a client pays for a layer: fetching its bytes, and the work it does on them before drawing.

use std::time::Duration;

use usize_cast::FromUsize as _;

use crate::decoder::GeometryValues;
use crate::encoder::source::LayerSource;
use crate::encoder::{EncoderConfig, Network};
use crate::tile::stored_len;

/// How long a client takes to decode one stored triangle.
const DECODE_PER_TRIANGLE: Duration = Duration::from_nanos(20);

/// How many times slower than a Ryzen 9 3900 core the client runs.
const CLIENT_SLOWDOWN: u32 = 4;

/// What a layer costs a client, in time on a Ryzen 9 3900 core, and its stored bytes to break a tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost {
    time: Duration,
    bytes: usize,
}

/// Prices layer encodings for the client an [`EncoderConfig`] targets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientCost {
    /// One byte's transfer, in the time a Ryzen 9 3900 core does the same share of a slower client's work.
    per_byte: Duration,
}

impl ClientCost {
    pub(crate) fn new(cfg: EncoderConfig) -> Self {
        // How long 8 bits take to arrive at each network's rate.
        let transfer = match cfg.network() {
            Network::Offline => Duration::ZERO,
            Network::Cellular3G => Duration::from_micros(4),
            Network::Cellular4G => Duration::from_nanos(400),
            Network::Cellular5G => Duration::from_nanos(40),
        };
        Self {
            per_byte: transfer / CLIENT_SLOWDOWN,
        }
    }

    /// The price of a layer on which the client does `work`.
    pub(crate) fn price(self, work: &impl ClientWork) -> Price {
        Price {
            per_byte: self.per_byte,
            work: work.client_time(),
        }
    }
}

/// Work a client does on a layer after fetching it and before drawing it.
pub(crate) trait ClientWork {
    /// How long this work takes on a Ryzen 9 3900 core.
    fn client_time(&self) -> Duration;
}

/// Decoding the stored triangles of the polygons.
impl ClientWork for GeometryValues {
    fn client_time(&self) -> Duration {
        let indices = self.index_buffer.as_ref().map_or(0, Vec::len);
        let triangles =
            u32::try_from(indices / 3).expect("a layer stores fewer triangles than its u32 bytes");
        DECODE_PER_TRIANGLE * triangles
    }
}

/// Cutting every polygon of a layer into triangles with `mapbox::earcut`, which no feature order changes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Earcut {
    time: Duration,
}

impl Earcut {
    pub(crate) fn of(source: &impl LayerSource) -> Self {
        let time = (0..source.feature_count())
            .flat_map(|feature| source.polygons(feature))
            .map(|rings| {
                let (mut vertices, mut ring_count) = (0, 0);
                for ring in rings {
                    vertices += stored_len(ring);
                    ring_count += 1;
                }
                polygon_earcut(vertices + ring_count, ring_count.saturating_sub(1))
            })
            .sum();
        Self { time }
    }
}

impl ClientWork for Earcut {
    fn client_time(&self) -> Duration {
        self.time
    }
}

/// How long `mapbox::earcut` takes on a Ryzen 9 3900 core for one polygon.
///
/// `n` counts its vertices as a client holds them, with each ring's closing point.
/// Fitted per polygon over five basemap and building tilesets, within 10% either way on the median layer.
fn polygon_earcut(n: usize, holes: usize) -> Duration {
    let (n, holes) = (u64::from_usize(n), u64::from_usize(holes));
    // `135 + n * (2.04 * min(n, 80) - 15.7) + 6.9 * n * holes` ns, in hundredths of a nanosecond.
    let centi_ns = 13_500 + 204 * n * n.min(80) + 690 * n * holes - 1_570 * n;
    Duration::from_nanos(centi_ns / 100)
}

/// What one layer costs a client per stored byte, on top of the work it does on them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Price {
    per_byte: Duration,
    work: Duration,
}

impl Price {
    /// The cost of the layer stored in `bytes`.
    pub(crate) fn at(self, bytes: usize) -> Cost {
        let stored = u32::try_from(bytes).expect("a layer fits in u32 bytes");
        Cost {
            time: self.per_byte * stored + self.work,
            bytes,
        }
    }
}
