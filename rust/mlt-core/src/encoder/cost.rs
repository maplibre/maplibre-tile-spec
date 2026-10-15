//! What a client pays for a layer: fetching its bytes, and the work it does on them before drawing.

use std::time::Duration;

use usize_cast::FromUsize as _;

use crate::decoder::GeometryValues;
use crate::encoder::source::LayerSource;
use crate::encoder::{Client, Network};
use crate::tile::stored_len;

/// What a layer costs a client, in time on a Ryzen 9 3900 core, and its stored bytes to break a tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost {
    time: Duration,
    bytes: usize,
}

/// Prices layer encodings for a client that fetches them over a [`Network`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientCost {
    client: Client,
    /// Transfer time per byte, divided by the client's slowdown.
    per_byte: Duration,
}

impl ClientCost {
    pub(crate) fn new(client: Client, network: Network) -> Self {
        // How long 8 bits take to arrive at each network's rate.
        let transfer = match network {
            Network::Offline => Duration::ZERO,
            Network::Cellular3G => Duration::from_micros(4),
            Network::Cellular4G => Duration::from_nanos(400),
            Network::Cellular5G => Duration::from_nanos(40),
        };
        Self {
            client,
            per_byte: transfer / client.slowdown(),
        }
    }

    /// The price of a layer on which the client does `work`.
    pub(crate) fn price(self, work: &impl ClientWork) -> Price {
        Price {
            per_byte: self.per_byte,
            work: work.client_time(self.client),
        }
    }
}

impl Client {
    /// How many times slower than a Ryzen 9 3900 core the client runs.
    fn slowdown(self) -> u32 {
        match self {
            // A laptop or desktop.
            Self::GlJs => 1,
            // A mid-range phone or car head unit.
            Self::Native => 2,
        }
    }

    /// How long the client takes to decode one stored triangle.
    fn decode_per_triangle(self) -> Duration {
        match self {
            // The Rust decoder in wasm.
            Self::GlJs => Duration::from_nanos(30),
            Self::Native => Duration::from_nanos(20),
        }
    }
}

/// Work a client does on a layer after fetching it and before drawing it.
pub(crate) trait ClientWork {
    /// How long `client` takes for this work on a Ryzen 9 3900 core.
    fn client_time(&self, client: Client) -> Duration;
}

/// Decoding the stored triangles of the polygons.
impl ClientWork for GeometryValues {
    fn client_time(&self, client: Client) -> Duration {
        let indices = self.index_buffer.as_ref().map_or(0, Vec::len);
        let triangles =
            u32::try_from(indices / 3).expect("a layer stores fewer triangles than its u32 bytes");
        client.decode_per_triangle() * triangles
    }
}

/// Cutting every polygon of a layer into triangles on the client, which no feature order changes.
///
/// `n` is a polygon's vertex count, closing points included.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Earcut {
    polygons: u64,
    /// The sum of `n`.
    vertices: u64,
    /// The sum of `n * min(n, 80)`.
    quadratic: u64,
    /// The sum of `n * holes`.
    hole_vertices: u64,
}

impl Earcut {
    pub(crate) fn of(source: &impl LayerSource) -> Self {
        let mut earcut = Self::default();
        for rings in (0..source.feature_count()).flat_map(|feature| source.polygons(feature)) {
            let (mut vertices, mut ring_count) = (0, 0);
            for ring in rings {
                vertices += stored_len(ring);
                ring_count += 1;
            }
            let n = u64::from_usize(vertices + ring_count);
            let holes = u64::from_usize(ring_count.saturating_sub(1));
            earcut.polygons += 1;
            earcut.vertices += n;
            earcut.quadratic += n * n.min(80);
            earcut.hole_vertices += n * holes;
        }
        earcut
    }
}

impl ClientWork for Earcut {
    fn client_time(&self, client: Client) -> Duration {
        let Self {
            polygons,
            vertices,
            quadratic,
            hole_vertices,
        } = *self;
        // Fitted per polygon on five tilesets, in hundredths of a nanosecond.
        let centi_ns = match client {
            // `126 + n * (3.46 * min(n, 80) + 22.2) + 3.6 * n * holes` ns, `earcut` 3.2.4 in Chromium.
            Client::GlJs => {
                12_600 * polygons + 346 * quadratic + 2_220 * vertices + 360 * hole_vertices
            }
            // `135 + n * (2.04 * min(n, 80) - 15.7) + 6.9 * n * holes` ns, `mapbox::earcut`.
            Client::Native => {
                13_500 * polygons + 204 * quadratic + 690 * hole_vertices - 1_570 * vertices
            }
        };
        Duration::from_nanos(centi_ns / 100)
    }
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
