use std::time::{Duration, Instant};

use anyhow::{Result as AnyResult, anyhow};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use martin_tile_utils::Encoding;
use mlt_core::encoder::EncoderConfig;
use moka::sync::Cache;
use pmtiles::{Compression, PmTilesWriter, TileCoord};
use size_format::SizeFormatterSI;
use xxhash_rust::xxh3::Xxh3Builder;

use super::{encode_one, whole_rate_per_sec};

/// Geographic fields that can be carried into a new `PMTiles` archive.
///
/// Optional fields support metadata sources such as `MBTiles`, where individual
/// values may be absent. Unspecified values retain the writer's defaults.
#[derive(Debug, Default, PartialEq)]
pub struct PmTilesGeography {
    pub min_zoom: Option<u8>,
    pub max_zoom: Option<u8>,
    pub bounds: Option<(f64, f64, f64, f64)>,
    pub center: Option<(f64, f64, u8)>,
}

impl PmTilesGeography {
    #[must_use]
    pub fn apply(self, mut writer: PmTilesWriter) -> PmTilesWriter {
        if let Some(min_zoom) = self.min_zoom {
            writer = writer.min_zoom(min_zoom);
        }
        if let Some(max_zoom) = self.max_zoom {
            writer = writer.max_zoom(max_zoom);
        }
        if let Some((min_lon, min_lat, max_lon, max_lat)) = self.bounds {
            writer = writer.bounds(min_lon, min_lat, max_lon, max_lat);
        }
        if let Some((longitude, latitude, zoom)) = self.center {
            writer = writer.center_zoom(zoom).center(longitude, latitude);
        }
        writer
    }
}

/// Cap on the encoding cache (which encoded `Bytes` to keep around).
pub const ENCODE_CACHE_BYTES: u64 = 512 * 1024 * 1024;
/// Cap on the tile cache track size (in bytes).
pub const MAX_TILE_CACHE_TRACK_SIZE_BYTES: usize = 1024;
const PROGRESS_BAR_TEMPLATE: &str = "  {bar:40.cyan/blue} {pos}/{len} tiles [{rate}, eta {eta}]";

/// The encode dedup cache, keyed on the raw (small) tile bytes.
pub type EncodeCache = Cache<Vec<u8>, (Bytes, u64), Xxh3Builder>;

pub fn make_progress_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(PROGRESS_BAR_TEMPLATE)
            .expect("invalid bar template")
            .with_key("rate", whole_rate_per_sec),
    );
    bar.enable_steady_tick(Duration::from_millis(200));
    bar
}

/// Build the bounded, byte-weighted cache used to dedup encoded tiles.
pub fn make_encode_cache() -> EncodeCache {
    Cache::builder()
        .max_capacity(ENCODE_CACHE_BYTES)
        .weigher(|_, v: &(Bytes, u64)| u32::try_from(v.0.len()).unwrap_or(u32::MAX))
        .build_with_hasher(Xxh3Builder::default())
}

/// Encode one source tile MVT -> MLT, deduplicating through `cache`.
///
/// Returns the encoded bytes, the raw MVT size, and whether the result came from the cache.
///
/// Only small tiles (ocean, empty land, ...) actually repeat across a tileset.
/// Big city tiles are essentially unique, so tiles over [`MAX_TILE_CACHE_TRACK_SIZE_BYTES`] skip the cache.
/// Any rare repeat still dedups when the container is written.
pub fn encode_tile(
    cache: &EncodeCache,
    data: &[u8],
    encoding: Encoding,
    cfg: EncoderConfig,
) -> AnyResult<(Bytes, u64, bool)> {
    if data.len() > MAX_TILE_CACHE_TRACK_SIZE_BYTES {
        let (encoded, raw_mvt_size) = encode_one(data.to_vec(), encoding, cfg)?;
        return Ok((encoded, raw_mvt_size, false));
    }
    let mut hit = true;
    let encoded = cache
        .try_get_with_by_ref(data, || {
            hit = false;
            encode_one(data.to_vec(), encoding, cfg)
        })
        .map_err(|e| anyhow!("{e}"))?;
    Ok((encoded.0, encoded.1, hit))
}

/// Running totals for a container-to-container conversion, matching the
/// summary line printed by every tileset conversion.
#[derive(Default)]
pub struct TileStats {
    written: u64,
    cache_hits: u64,
    cache_encoded: u64,
    raw_mvt_bytes: u64,
    raw_mlt_bytes: u64,
}

impl TileStats {
    pub fn record(&mut self, encoded_len: u64, raw_mvt_size: u64, hit: bool) {
        self.written += 1;
        if hit {
            self.cache_hits += 1;
        } else {
            self.cache_encoded += 1;
            self.raw_mvt_bytes += raw_mvt_size;
            self.raw_mlt_bytes += encoded_len;
        }
    }

    pub fn print_summary(
        &self,
        start: Instant,
        input_archive_size: u64,
        output_archive_size: u64,
        source_encoding: Encoding,
        tile_compression: Compression,
    ) {
        let source_encoding = source_encoding.compression().unwrap_or("none");
        let tile_compression = tile_compression.content_encoding().unwrap_or("none");
        eprintln!(
            "  converted {} tiles ({} unique encoded, {} cache hits) in {:.1?}",
            self.written,
            self.cache_encoded,
            self.cache_hits,
            start.elapsed(),
        );
        eprintln!(
            "  size raw/archive: MVT({source_encoding}) {:.1}B/{:.1}B -> MLT({tile_compression}) {:.1}B/{:.1}B",
            SizeFormatterSI::new(self.raw_mvt_bytes),
            SizeFormatterSI::new(input_archive_size),
            SizeFormatterSI::new(self.raw_mlt_bytes),
            SizeFormatterSI::new(output_archive_size),
        );
    }
}

/// One encoded tile leaving the pipeline: its coordinate, the MLT bytes, the
/// source MVT size, and whether the encode was served from the dedup cache.
pub struct EncodedTile {
    pub coord: TileCoord,
    pub data: Bytes,
    pub raw_mvt_size: u64,
    pub hit: bool,
}
