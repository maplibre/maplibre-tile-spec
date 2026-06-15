use std::time::{Duration, Instant};

use anyhow::{Result as AnyResult, anyhow};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use martin_tile_utils::Encoding;
use mlt_core::encoder::EncoderConfig;
use moka::sync::Cache;
use pmtiles::TileCoord;
use size_format::SizeFormatterSI;
use xxhash_rust::xxh3::Xxh3Builder;

use super::{encode_one, whole_rate_per_sec};

/// Cap on the encoding cache (which encoded `Bytes` to keep around).
pub const ENCODE_CACHE_BYTES: u64 = 512 * 1024 * 1024;
/// Cap on the tile cache track size (in bytes).
pub const MAX_TILE_CACHE_TRACK_SIZE_BYTES: usize = 1024;
const PROGRESS_BAR_TEMPLATE: &str = "  {bar:40.cyan/blue} {pos}/{len} tiles [{rate}, eta {eta}]";

/// The encode dedup cache, keyed on the raw (small) tile bytes.
///
/// The key is the tile content itself rather than a 64-bit digest of it, so two
/// distinct tiles can never collide onto one cache slot.
/// `xxh3` still does the bucket hashing, just over the bytes moka already holds.
/// The owned key is a `Vec<u8>` because that is what `[u8]::to_owned` yields,
/// which is what [`Cache::try_get_with_by_ref`] needs to build the key on a miss.
pub type EncodeCache = Cache<Vec<u8>, Bytes, Xxh3Builder>;

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
        .weigher(|_, v: &Bytes| u32::try_from(v.len()).unwrap_or(u32::MAX))
        .build_with_hasher(Xxh3Builder::default())
}

/// Encode one source tile MVT → MLT, deduplicating through `cache`.
///
/// Only small tiles (ocean, empty land, ...) actually repeat across a tileset;
/// big city tiles are essentially unique, so tiles over
/// [`MAX_TILE_CACHE_TRACK_SIZE_BYTES`] skip the cache entirely (any rare repeat
/// still dedups when the container is written). Small tiles look up by their raw
/// bytes; `try_get_with_by_ref` only materializes the owned key on a miss, so
/// cache hits never allocate. Returns the encoded bytes and whether they came
/// from the cache. Mirrors `files.rs` and the mbtiles transcoder.
pub fn encode_tile(
    cache: &EncodeCache,
    data: &[u8],
    encoding: Encoding,
    cfg: EncoderConfig,
) -> AnyResult<(Bytes, bool)> {
    if data.len() > MAX_TILE_CACHE_TRACK_SIZE_BYTES {
        return Ok((encode_one(data.to_vec(), encoding, cfg)?, false));
    }
    let mut hit = true;
    let encoded = cache
        .try_get_with_by_ref(data, || {
            hit = false;
            encode_one(data.to_vec(), encoding, cfg)
        })
        .map_err(|e| anyhow!("{e}"))?;
    Ok((encoded, hit))
}

/// Running totals for a container-to-container conversion, matching the
/// summary line printed by every tileset conversion.
#[derive(Default)]
pub struct TileStats {
    written: u64,
    cache_hits: u64,
    cache_encoded: u64,
    bytes_in: u64,
    bytes_out: u64,
}

impl TileStats {
    pub fn record(&mut self, encoded_len: u64, bytes_in: u64, hit: bool) {
        self.written += 1;
        if hit {
            self.cache_hits += 1;
        } else {
            self.cache_encoded += 1;
            self.bytes_in += bytes_in;
            self.bytes_out += encoded_len;
        }
    }

    pub fn print_summary(&self, start: Instant) {
        eprintln!(
            "  converted {} tiles ({} unique encoded, {} cache hits, {:.1}B -> {:.1}B) in {:.1?}",
            self.written,
            self.cache_encoded,
            self.cache_hits,
            SizeFormatterSI::new(self.bytes_in),
            SizeFormatterSI::new(self.bytes_out),
            start.elapsed(),
        );
    }
}

/// One encoded tile leaving the pipeline: its coordinate, the MLT bytes, the
/// source MVT size, and whether the encode was served from the dedup cache.
pub struct EncodedTile {
    pub coord: TileCoord,
    pub data: Bytes,
    pub bytes_in: u64,
    pub hit: bool,
}
