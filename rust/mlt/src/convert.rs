use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{fs, mem};

use anyhow::{Context as _, Result as AnyResult, anyhow, bail};
use clap::{Args, ValueEnum};
use futures::{StreamExt as _, TryStreamExt as _};
use indicatif::{HumanDuration, ProgressBar, ProgressState, ProgressStyle};
use martin_tile_utils::{Encoding, Format, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{
    CopyDuplicateMode, MbtType, Mbtiles, TileCoord, init_mbtiles_schema, invert_y_value,
};
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig};
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::{Decoder, Layer, Parser};
use moka::sync::Cache;
use rayon::iter::{IntoParallelIterator as _, ParallelBridge as _, ParallelIterator as _};
use size_format::SizeFormatterSI;
use sqlx::sqlite::SqliteRow;
use sqlx::{Connection as _, Row as _, SqliteConnection};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::spawn_blocking;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use crate::ls::is_mlt_extension;

/// How many raw tiles to collect before shipping a batch to the compute stage.
const BATCH_SIZE: usize = 500;

/// Maximum input-tile size (bytes) for which we consult the dedup cache.
///
/// Only small tiles (empty-ocean quads, land backgrounds) tend to repeat in
/// real tilesets, so tracking anything larger burns cache weight for a
/// near-zero hit rate.
const MAX_TILE_TRACK_SIZE: usize = 1024;

/// Maximum aggregate weight (encoded-tile bytes) held by the dedup cache.
const CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Raw MVT batch forwarded from the reader to the compute stage. The middle
/// element is the schema-provided MD5 (as `u128`) when the source file carries
/// one, or `None` for Flat sources (in which case the compute stage will
/// content-hash the bytes itself).
type RawBatch = Vec<(TileCoord, Option<u128>, Vec<u8>)>;
/// Converted MLT batch forwarded from the compute stage to the writer.
type MltBatch = Vec<(TileCoord, Vec<u8>)>;
/// Number of in-flight batches allowed between adjacent pipeline stages (backpressure).
const CHANNEL_BUFFER: usize = 4;
/// Maximum time between forced flushes in the writer, regardless of batch fullness.
/// Keeps data safe on long-running jobs and avoids holding a huge open transaction.
const SAVE_EVERY: Duration = Duration::from_secs(60);
/// Minimum interval between progress log lines in non-interactive (non-TTY) mode.
const PROGRESS_REPORT_EVERY: Duration = Duration::from_secs(2);

/// Format a progress bar's throughput as a whole-number `{rate}/s`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "state.per_sec() is always non-negative and well below 2^63 tiles/sec"
)]
fn whole_rate_per_sec(state: &ProgressState, w: &mut dyn std::fmt::Write) {
    let _ = w.write_fmt(format_args!("{}/s", state.per_sec() as u64));
}

/// Shared cache type: maps an input-identity key (MD5 of source tile data,
/// or xxh3 of it for schemas without stored hashes) to the encoded MLT bytes.
type EncodedCache = Cache<u128, Arc<Vec<u8>>>;

/// Construct a weighted [`EncodedCache`] bounded by roughly `max_bytes` of
/// encoded-tile payload.
fn make_cache(max_bytes: u64) -> EncodedCache {
    Cache::builder()
        .max_capacity(max_bytes)
        .weigher(|_key, value: &Arc<Vec<u8>>| u32::try_from(value.len()).unwrap_or(u32::MAX))
        .build()
}

/// Parse a 32-character hex MD5 string to a `u128`. Returns `None` on malformed input.
fn hex_md5_to_u128(s: &str) -> Option<u128> {
    if s.len() != 32 {
        return None;
    }
    u128::from_str_radix(s, 16).ok()
}

/// Counters kept across the encoding stage for end-of-run dedup reporting.
#[derive(Default)]
struct DedupStats {
    hits: AtomicU64,
    encoded: AtomicU64,
    /// Cumulative size (bytes) of cache-hit encoded tiles — encode work skipped.
    bytes_saved: AtomicU64,
}

impl DedupStats {
    fn record_hit(&self, size: usize) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.bytes_saved.fetch_add(size as u64, Ordering::Relaxed);
    }
    fn record_encode(&self) {
        self.encoded.fetch_add(1, Ordering::Relaxed);
    }
}

/// Print a one-line dedup summary alongside the "done" message.
#[expect(
    clippy::cast_precision_loss,
    reason = "hit/miss counts are well below 2^52 for realistic tilesets"
)]
fn format_dedup_line(stats: &DedupStats, cache: &EncodedCache) -> String {
    cache.run_pending_tasks();
    let hits = stats.hits.load(Ordering::Relaxed);
    let encoded = stats.encoded.load(Ordering::Relaxed);
    let bytes_saved = stats.bytes_saved.load(Ordering::Relaxed);
    let total = hits + encoded;
    let hit_rate = if total == 0 {
        0.0
    } else {
        (hits as f64 * 100.0) / (total as f64)
    };
    format!(
        "  dedup: {encoded} unique encoded, {hits} cached ({hit_rate:.1}% hit rate, \
         ~{:.1}B of encode work skipped); cache weight {:.1}B",
        SizeFormatterSI::new(bytes_saved),
        SizeFormatterSI::new(cache.weighted_size()),
    )
}

/// Output `.mbtiles` schema variant.
///
/// Mirrors [`MbtType`] but without the `hash_view` detail, keeping the CLI simple.
#[derive(Clone, Default, ValueEnum)]
enum MbtFormat {
    /// Single table with all tiles; no deduplication (smallest overhead)
    #[default]
    Flat,
    /// Single table with tiles and `MD5` hashes
    #[value(name = "flat-with-hash")]
    FlatWithHash,
    /// Separate `images` / `map` tables; identical tiles stored only once
    Normalized,
}

impl From<MbtFormat> for MbtType {
    fn from(f: MbtFormat) -> Self {
        match f {
            MbtFormat::Flat => Self::Flat,
            MbtFormat::FlatWithHash => Self::FlatWithHash,
            MbtFormat::Normalized => Self::Normalized { hash_view: false },
        }
    }
}

/// Which sort strategies to attempt during re-encoding.
#[derive(Clone, Default, ValueEnum)]
enum SortMode {
    /// Try all sort strategies and keep the smallest result
    #[default]
    Auto,
    /// Do not reorder features (original order only)
    None,
    /// Only try Z-order (Morton) curve sort
    Morton,
    /// Only try Hilbert curve sort
    Hilbert,
    /// Only try feature-ID ascending sort
    Id,
}

#[derive(Args)]
pub struct ConvertArgs {
    /// Input: a directory with .mlt/.mvt tiles, a single tile file, or an .mbtiles database
    input: PathBuf,
    /// Output: a directory for re-encoded .mlt files, or an .mbtiles database (required when input is .mbtiles)
    output: PathBuf,
    /// Add tessellation
    #[clap(short, long, default_value = "false")]
    tessellate: bool,
    /// Sort strategy to try when re-encoding (encoder keeps the smallest result)
    #[clap(long, default_value = "auto")]
    sort: SortMode,
    /// Schema type for the output `.mbtiles` file; defaults to the input file's schema
    #[clap(long)]
    mbtiles_format: Option<MbtFormat>,
}

pub fn convert(args: &ConvertArgs) -> AnyResult<()> {
    let cfg = EncoderConfig {
        tessellate: args.tessellate,
        try_spatial_morton_sort: matches!(args.sort, SortMode::Auto | SortMode::Morton),
        try_spatial_hilbert_sort: matches!(args.sort, SortMode::Auto | SortMode::Hilbert),
        try_id_sort: matches!(args.sort, SortMode::Auto | SortMode::Id),
        ..Default::default()
    };

    if is_mbtiles_extension(&args.input) {
        if !is_mbtiles_extension(&args.output) {
            bail!(
                "Output must be an .mbtiles file when input is an .mbtiles file, got: {}",
                args.output.display()
            );
        }
        if args.output.exists() {
            bail!(
                "Output {} already exists; refusing to append. \
                 Delete it first or choose a different path.",
                args.output.display()
            );
        }
        return tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?
            .block_on(convert_mbtiles_async(args, cfg));
    }

    // Determine the base for computing relative paths. For a single-file
    // input, the "base" is the parent directory so `strip_prefix` just yields
    // the filename.
    let base = if args.input.is_dir() {
        args.input.as_path()
    } else {
        args.input.parent().unwrap_or(Path::new("."))
    };

    let cache: EncodedCache = make_cache(CACHE_MAX_BYTES);
    let stats = DedupStats::default();
    let failed = AtomicUsize::new(0);

    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {elapsed_precise} [{pos} files, {rate}] {msg}")
            .expect("invalid spinner template")
            .with_key("rate", whole_rate_per_sec),
    );
    bar.enable_steady_tick(Duration::from_millis(100));

    // `bar.println` is a no-op when hidden (non-TTY), so fall through to `eprintln!`.
    let emit = |msg: String| {
        if bar.is_hidden() {
            eprintln!("{msg}");
        } else {
            bar.println(msg);
        }
    };

    WalkDir::new(&args.input)
        .into_iter()
        .filter_map(|r| match r {
            Ok(e) => Some(e),
            Err(e) => {
                emit(format!("warning: walkdir: {e}"));
                failed.fetch_add(1, Ordering::Relaxed);
                None
            }
        })
        .filter(|e| e.file_type().is_file() && is_convert_extension(e.path()))
        .par_bridge()
        .for_each(|entry| {
            let in_path = entry.into_path();
            let result = convert_file(&in_path, base, &args.output, cfg, &cache, &stats);
            bar.inc(1);
            if let Err(e) = result {
                emit(format!("error: {}: {e:#}", in_path.display()));
                failed.fetch_add(1, Ordering::Relaxed);
            }
        });

    bar.finish_and_clear();

    let n = failed.into_inner();
    if n > 0 {
        bail!("{n} file(s) failed to convert");
    }

    let processed = stats.hits.load(Ordering::Relaxed) + stats.encoded.load(Ordering::Relaxed);
    if processed == 0 {
        eprintln!("No .mlt or .mvt files found in {}", args.input.display());
        return Ok(());
    }
    eprintln!("{}", format_dedup_line(&stats, &cache));
    Ok(())
}

fn convert_file(
    file: &Path,
    base: &Path,
    output: &Path,
    cfg: EncoderConfig,
    cache: &EncodedCache,
    stats: &DedupStats,
) -> AnyResult<()> {
    let rel = file
        .strip_prefix(base)
        .with_context(|| format!("stripping prefix from {}", file.display()))?;
    let out_path = output.join(rel).with_extension("mlt");

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let buffer = fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let is_mlt = is_mlt_extension(file);
    let file_display = file.display().to_string();

    // Skip cache for large tiles — they are almost certainly unique.
    if buffer.len() > MAX_TILE_TRACK_SIZE {
        let out_bytes = if is_mlt {
            convert_mlt_buffer(&buffer, cfg)
                .with_context(|| format!("converting MLT {file_display}"))?
        } else {
            convert_mvt_buffer(buffer, cfg)
                .with_context(|| format!("converting MVT {file_display}"))?
        };
        stats.record_encode();
        fs::write(&out_path, &out_bytes)
            .with_context(|| format!("writing {}", out_path.display()))?;
        return Ok(());
    }

    let key = xxh3_128(&buffer);
    let entry = cache
        .entry(key)
        .or_try_insert_with(|| -> AnyResult<Arc<Vec<u8>>> {
            let out_bytes = if is_mlt {
                convert_mlt_buffer(&buffer, cfg)
                    .with_context(|| format!("converting MLT {file_display}"))?
            } else {
                convert_mvt_buffer(buffer, cfg)
                    .with_context(|| format!("converting MVT {file_display}"))?
            };
            Ok(Arc::new(out_bytes))
        })
        .map_err(|e: Arc<anyhow::Error>| anyhow!("{e:#}"))?;

    let is_fresh = entry.is_fresh();
    let out_arc = entry.into_value();
    if is_fresh {
        stats.record_encode();
    } else {
        stats.record_hit(out_arc.len());
    }

    fs::write(&out_path, out_arc.as_slice())
        .with_context(|| format!("writing {}", out_path.display()))?;

    Ok(())
}

fn is_convert_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mlt" | "mvt")
    )
}

fn is_mbtiles_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(OsStr::to_str), Some("mbtiles"))
}

/// Re-encode an MLT tile using automatic encoding selection.
///
/// Every Tag01 layer is fully decoded to [`TileLayer01`] and then re-encoded
/// via [`encode_tile_layer`].  Unknown layer tags are passed through unchanged.
fn convert_mlt_buffer(buffer: &[u8], cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut out: Vec<u8> = Vec::new();

    for layer in layers {
        match layer {
            Layer::Tag01(l) => {
                let tile = l.into_tile(&mut dec)?;
                out.extend_from_slice(&tile.encode(cfg)?);
            }
            Layer::Unknown(u) => {
                out.extend(EncodedUnknown::from(u).write_to(Encoder::default())?.data);
            }
        }
    }

    Ok(out)
}

/// Convert an MVT tile to an MLT tile using automatic encoding selection.
///
/// Each MVT layer is converted to a [`mlt_core::TileLayer01`] and encoded
/// via [`encode_tile_layer`].
fn convert_mvt_buffer(buffer: Vec<u8>, cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for tile in mvt_to_tile_layers(buffer)? {
        out.extend_from_slice(&tile.encode(cfg)?);
    }
    Ok(out)
}

/// Stage 1: stream raw MVT tiles from the source `.mbtiles` in batches.
///
/// Dispatches to a schema-aware reader so that `FlatWithHash` / `Normalized`
/// sources pass their stored content hash as a cache key, while `Flat`
/// sources fall back to content hashing.
#[hotpath::measure]
async fn mbtiles_reader(
    src: Mbtiles,
    mut src_conn: SqliteConnection,
    src_type: MbtType,
    raw_tx: Sender<RawBatch>,
) -> AnyResult<()> {
    match src_type {
        MbtType::Flat => read_flat(&src, &mut src_conn, &raw_tx).await,
        MbtType::FlatWithHash => {
            read_with_hash(
                &mut src_conn,
                "SELECT zoom_level, tile_column, tile_row, tile_data, tile_hash \
                 FROM tiles_with_hash",
                &raw_tx,
            )
            .await
        }
        MbtType::Normalized { .. } => {
            read_with_hash(
                &mut src_conn,
                "SELECT zoom_level, tile_column, tile_row, tile_data, \
                        map.tile_id AS tile_hash \
                 FROM map JOIN images ON map.tile_id = images.tile_id",
                &raw_tx,
            )
            .await
        }
    }
}

/// Reader for `Flat` schemas (no stored hash).
#[hotpath::measure]
async fn read_flat(
    src: &Mbtiles,
    src_conn: &mut SqliteConnection,
    raw_tx: &Sender<RawBatch>,
) -> AnyResult<()> {
    let mut stream = src.stream_tiles(src_conn);
    let mut batch: RawBatch = Vec::with_capacity(BATCH_SIZE);

    while let Some(res) = stream.next().await {
        let (coord, data_opt) = res?;
        if let Some(data) = data_opt {
            batch.push((coord, None, data));
            if batch.len() >= BATCH_SIZE {
                let full = mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE));
                raw_tx
                    .send(full)
                    .await
                    .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
            }
        }
    }
    if !batch.is_empty() {
        raw_tx
            .send(batch)
            .await
            .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
    }
    Ok(())
}

/// Reader for schemas that carry a hex-encoded content hash alongside the tile data.
#[hotpath::measure]
async fn read_with_hash(
    src_conn: &mut SqliteConnection,
    sql: &'static str,
    raw_tx: &Sender<RawBatch>,
) -> AnyResult<()> {
    let mut stream = sqlx::query(sql).fetch(src_conn);
    let mut batch: RawBatch = Vec::with_capacity(BATCH_SIZE);

    while let Some(row) = stream.try_next().await? {
        let Some(coord) = row_to_coord(&row)? else {
            continue;
        };
        let data: Option<Vec<u8>> = row.try_get("tile_data")?;
        let Some(data) = data else {
            continue;
        };
        let hash: Option<String> = row.try_get("tile_hash")?;
        // Malformed/missing hash falls back to content hashing in the compute stage.
        let key = hash.as_deref().and_then(hex_md5_to_u128);
        batch.push((coord, key, data));
        if batch.len() >= BATCH_SIZE {
            let full = mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE));
            raw_tx
                .send(full)
                .await
                .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
        }
    }
    if !batch.is_empty() {
        raw_tx
            .send(batch)
            .await
            .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
    }
    Ok(())
}

/// Parse zoom/col/row from a sqlite row into an XYZ-space [`TileCoord`].
/// Returns `Ok(None)` for rows whose coordinates are NULL or out of range.
fn row_to_coord(row: &SqliteRow) -> AnyResult<Option<TileCoord>> {
    let z: Option<i64> = row.try_get("zoom_level")?;
    let x: Option<i64> = row.try_get("tile_column")?;
    let y: Option<i64> = row.try_get("tile_row")?;
    let (Some(z), Some(x), Some(y)) = (z, x, y) else {
        return Ok(None);
    };
    let Ok(z) = u8::try_from(z) else {
        return Ok(None);
    };
    let Ok(x) = u32::try_from(x) else {
        return Ok(None);
    };
    let Ok(y) = u32::try_from(y) else {
        return Ok(None);
    };
    if !TileCoord::is_possible_on_zoom_level(z, x, y) {
        return Ok(None);
    }
    // mbtiles stores TMS-oriented y; flip to XYZ.
    Ok(Some(TileCoord::new_unchecked(z, x, invert_y_value(z, y))))
}

/// Stage 2: decompress and convert each tile on the rayon pool, then forward
/// converted MLT batches to the writer. Per-tile errors are logged as warnings.
#[hotpath::measure]
async fn mbtiles_compute(
    mut raw_rx: Receiver<RawBatch>,
    mlt_tx: Sender<MltBatch>,
    encoding: Encoding,
    cfg: EncoderConfig,
    cache: EncodedCache,
    stats: Arc<DedupStats>,
) -> AnyResult<()> {
    while let Some(batch) = raw_rx.recv().await {
        let cache = cache.clone();
        let stats = Arc::clone(&stats);
        let mlt_batch: MltBatch = spawn_blocking(move || {
            batch
                .into_par_iter()
                .filter_map(|(coord, key, data)| {
                    encode_cached(coord, key, data, encoding, cfg, &cache, &stats)
                })
                .collect()
        })
        .await?;

        if !mlt_batch.is_empty() {
            mlt_tx
                .send(mlt_batch)
                .await
                .map_err(|_| anyhow!("writer stage closed unexpectedly"))?;
        }
    }
    Ok(())
}

/// Resolve one tile against the dedup cache, encoding it only on a miss.
///
/// Returns `None` (with a warning logged) if the per-tile encode fails so the
/// rest of the batch is still forwarded.
fn encode_cached(
    coord: TileCoord,
    key: Option<u128>,
    data: Vec<u8>,
    encoding: Encoding,
    cfg: EncoderConfig,
    cache: &EncodedCache,
    stats: &DedupStats,
) -> Option<(TileCoord, Vec<u8>)> {
    // Skip cache for large tiles — they are almost certainly unique.
    if data.len() > MAX_TILE_TRACK_SIZE {
        return match encode_one(data, encoding, cfg) {
            Ok(mlt) => {
                stats.record_encode();
                Some((coord, mlt))
            }
            Err(e) => {
                eprintln!("warning: skipping tile: {e:#}");
                None
            }
        };
    }

    // For Flat sources (no stored hash) compute an xxh3_128 content hash;
    // otherwise reuse the schema-provided MD5.
    let key = key.unwrap_or_else(|| xxh3_128(&data));

    let entry = cache
        .entry(key)
        .or_try_insert_with(|| -> AnyResult<Arc<Vec<u8>>> {
            Ok(Arc::new(encode_one(data, encoding, cfg)?))
        })
        .inspect_err(|e: &Arc<anyhow::Error>| eprintln!("warning: skipping tile: {e:#}"))
        .ok()?;

    let is_fresh = entry.is_fresh();
    let arc = entry.into_value();
    if is_fresh {
        stats.record_encode();
    } else {
        stats.record_hit(arc.len());
    }
    Some((coord, (*arc).clone()))
}

/// Decompress one raw tile and convert it from MVT to MLT.
fn encode_one(data: Vec<u8>, encoding: Encoding, cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let mvt = match encoding {
        Encoding::Gzip => decode_gzip(&data)?,
        Encoding::Zlib => decode_zlib(&data)?,
        Encoding::Brotli => decode_brotli(&data)?,
        Encoding::Zstd => decode_zstd(&data)?,
        Encoding::Uncompressed | Encoding::Internal => data,
    };
    convert_mvt_buffer(mvt, cfg)
}

/// Flush `pending` tiles to `dst` in a single `SQLite` transaction.
/// Returns the number of tiles flushed (0 if `pending` was empty).
#[hotpath::measure]
async fn flush_pending(
    dst: &Mbtiles,
    dst_conn: &mut SqliteConnection,
    pending: &mut MltBatch,
    total: &mut usize,
    mbt_type: MbtType,
) -> AnyResult<usize> {
    if pending.is_empty() {
        return Ok(0);
    }
    let flat: Vec<(u8, u32, u32, Vec<u8>)> =
        pending.drain(..).map(|(c, d)| (c.z, c.x, c.y, d)).collect();
    let n = flat.len();
    dst.insert_tiles(dst_conn, mbt_type, CopyDuplicateMode::Override, &flat)
        .await?;
    *total += n;
    Ok(n)
}

/// Stage 3: accumulate converted MLT tiles and flush to the destination `.mbtiles`
/// whenever `BATCH_SIZE` tiles are pending or `SAVE_EVERY` has elapsed.
/// Returns the total number of tiles written.
#[hotpath::measure]
async fn mbtiles_writer(
    dst: Mbtiles,
    mut dst_conn: SqliteConnection,
    mut mlt_rx: Receiver<MltBatch>,
    mbt_type: MbtType,
    bar: ProgressBar,
) -> AnyResult<usize> {
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&mut dst_conn)
        .await?;
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(&mut dst_conn)
        .await?;

    let mut total = 0usize;
    let mut pending: MltBatch = Vec::with_capacity(BATCH_SIZE);
    let mut last_saved = Instant::now();
    let mut last_logged = Instant::now();

    while let Some(batch) = mlt_rx.recv().await {
        pending.extend(batch);
        if pending.len() >= BATCH_SIZE || last_saved.elapsed() >= SAVE_EVERY {
            let flushed =
                flush_pending(&dst, &mut dst_conn, &mut pending, &mut total, mbt_type).await?;
            if flushed > 0 {
                bar.inc(flushed as u64);
                last_saved = Instant::now();
                // In non-interactive mode the bar is hidden; emit periodic log lines instead.
                if bar.is_hidden() && last_logged.elapsed() >= PROGRESS_REPORT_EVERY {
                    bar.println(format!("  {total} tiles converted"));
                    last_logged = Instant::now();
                }
            }
        }
    }
    // Final flush after the channel closes.
    let flushed = flush_pending(&dst, &mut dst_conn, &mut pending, &mut total, mbt_type).await?;
    if flushed > 0 {
        bar.inc(flushed as u64);
    }

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&mut dst_conn)
        .await?;
    dst_conn.close().await?;

    Ok(total)
}

/// Fan-out / fan-in pipeline:
///
/// ```text
///  [SQLite reader]  ──batch──▶  [spawn_blocking + rayon]  ──batch──▶  [SQLite writer]
/// ```
///
/// WAL mode is enabled on the destination file for better write throughput.
async fn convert_mbtiles_async(args: &ConvertArgs, cfg: EncoderConfig) -> AnyResult<()> {
    // ── source ────────────────────────────────────────────────────────────────
    let src = Mbtiles::new(&args.input)?;
    let mut src_conn = src.open_readonly().await?;

    let meta = src.get_metadata(&mut src_conn).await?;
    let tile_info = src
        .detect_format(&meta.tilejson, &mut src_conn)
        .await?
        .ok_or_else(|| anyhow!("{} appears to be empty", args.input.display()))?;

    if tile_info.format != Format::Mvt {
        bail!(
            "Expected MVT tiles, got {} in {}",
            tile_info.format,
            args.input.display()
        );
    }
    // Detect the source schema type and use it as default for the output.
    let src_type = src.detect_type(&mut src_conn).await?;
    let mbt_type = args.mbtiles_format.clone().map_or(src_type, MbtType::from);

    // Count source tiles so the progress bar can show percentage and ETA.
    let count_sql = if src_type.is_normalized() {
        "SELECT COUNT(*) FROM map"
    } else {
        "SELECT COUNT(*) FROM tiles"
    };
    let tile_count: u64 = sqlx::query_scalar::<_, i64>(count_sql)
        .fetch_one(&mut src_conn)
        .await
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    // ── destination ───────────────────────────────────────────────────────────
    let dst = Mbtiles::new(&args.output)?;
    let mut dst_conn = dst.open_or_new().await?;
    init_mbtiles_schema(&mut dst_conn, mbt_type).await?;

    // Copy metadata from source; `format` is overridden to `mlt` afterwards.
    copy_metadata(&mut src_conn, &mut dst_conn).await?;
    dst.set_metadata_value(&mut dst_conn, "format", Format::Mlt.metadata_format_value())
        .await?;

    // ── progress bar (mirrors martin-cp) ──────────────────────────────────────
    let bar = ProgressBar::new(tile_count);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "{elapsed_precise} -> eta: {eta} [{bar:40.cyan/blue} {percent}%] {pos}/{len} ({rate}) | {msg}",
            )
            .expect("invalid progress bar template")
            .progress_chars("█▓▒░ ")
            .with_key("rate", whole_rate_per_sec),
    );
    bar.println(format!(
        "{} → {} ({mbt_type}):",
        args.input.display(),
        args.output.display()
    ));

    // ── Normalized → Normalized fast path ────────────────────────────────────
    // The `images` table already holds one row per unique payload and `map`
    // carries coordinate → tile_id references that don't need re-encoding.
    // Stream-encode `images`, then bulk-copy `map` via ATTACH DATABASE.
    if matches!(src_type, MbtType::Normalized { .. })
        && matches!(mbt_type, MbtType::Normalized { .. })
    {
        let total = normalized_encode_and_copy(
            &args.input,
            &mut src_conn,
            &mut dst_conn,
            tile_info.encoding,
            cfg,
            &bar,
        )
        .await?;

        dst_conn.close().await?;

        if bar.is_hidden() {
            eprintln!("  done: {total} tiles in {}", HumanDuration(bar.elapsed()));
        }
        bar.finish_with_message(format!("done, {total} tiles"));
        return Ok(());
    }

    let cache: EncodedCache = make_cache(CACHE_MAX_BYTES);
    let stats = Arc::new(DedupStats::default());

    // ── pipeline ──────────────────────────────────────────────────────────────
    let (raw_tx, raw_rx) =
        hotpath::channel!(channel::<RawBatch>(CHANNEL_BUFFER), label = "raw_mvt");
    let (mlt_tx, mlt_rx) =
        hotpath::channel!(channel::<MltBatch>(CHANNEL_BUFFER), label = "encoded_mlt");

    let ((), (), total) = tokio::try_join!(
        mbtiles_reader(src, src_conn, src_type, raw_tx),
        mbtiles_compute(
            raw_rx,
            mlt_tx,
            tile_info.encoding,
            cfg,
            cache.clone(),
            Arc::clone(&stats),
        ),
        mbtiles_writer(dst, dst_conn, mlt_rx, mbt_type, bar.clone()),
    )?;

    let dedup_line = format_dedup_line(&stats, &cache);

    if bar.is_hidden() {
        eprintln!("  done: {total} tiles in {}", HumanDuration(bar.elapsed()));
        eprintln!("{dedup_line}");
    } else {
        bar.println(dedup_line);
    }
    bar.finish_with_message(format!("done, {total} tiles"));
    Ok(())
}

/// Raw images batch: `(tile_id_hex, mvt_bytes)`.
type NormRawBatch = Vec<(String, Vec<u8>)>;
/// Encoded images batch: `(tile_id_hex, mlt_bytes)`.
type NormEncBatch = Vec<(String, Vec<u8>)>;

#[hotpath::measure]
async fn normalized_encode_and_copy(
    src_path: &Path,
    src_conn: &mut SqliteConnection,
    dst_conn: &mut SqliteConnection,
    encoding: Encoding,
    cfg: EncoderConfig,
    bar: &ProgressBar,
) -> AnyResult<usize> {
    // Retarget the bar to the `images` row count (the rate-limiting phase).
    let image_count: u64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM images")
        .fetch_one(&mut *src_conn)
        .await
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);
    bar.set_length(image_count);
    bar.set_message("encoding images");

    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&mut *dst_conn)
        .await?;
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(&mut *dst_conn)
        .await?;

    // ── Phase 1: stream-encode images ────────────────────────────────────
    let (raw_tx, raw_rx) = hotpath::channel!(
        channel::<NormRawBatch>(CHANNEL_BUFFER),
        label = "raw_images"
    );
    let (enc_tx, enc_rx) = hotpath::channel!(
        channel::<NormEncBatch>(CHANNEL_BUFFER),
        label = "encoded_images"
    );

    let ((), (), total) = tokio::try_join!(
        normalized_read_images(src_conn, raw_tx),
        normalized_compute(raw_rx, enc_tx, encoding, cfg),
        normalized_write_images(dst_conn, enc_rx, bar),
    )?;

    // ── Phase 2: bulk copy `map` via ATTACH ──────────────────────────────
    bar.set_message("copying map");
    copy_map_via_attach(dst_conn, src_path).await?;

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&mut *dst_conn)
        .await?;

    Ok(total)
}

/// Stream `images` rows into batches on the compute channel.
#[hotpath::measure]
async fn normalized_read_images(
    src_conn: &mut SqliteConnection,
    raw_tx: Sender<NormRawBatch>,
) -> AnyResult<()> {
    let mut stream = sqlx::query("SELECT tile_id, tile_data FROM images").fetch(src_conn);
    let mut batch: NormRawBatch = Vec::with_capacity(BATCH_SIZE);

    while let Some(row) = stream.try_next().await? {
        let tile_id: String = row.try_get("tile_id")?;
        let data: Option<Vec<u8>> = row.try_get("tile_data")?;
        let Some(data) = data else {
            continue;
        };
        batch.push((tile_id, data));
        if batch.len() >= BATCH_SIZE {
            let full = mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE));
            raw_tx
                .send(full)
                .await
                .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
        }
    }
    if !batch.is_empty() {
        raw_tx
            .send(batch)
            .await
            .map_err(|_| anyhow!("compute stage closed unexpectedly"))?;
    }
    Ok(())
}

/// Stage 2 (Normalized): decompress + re-encode each image on the rayon pool.
#[hotpath::measure]
async fn normalized_compute(
    mut raw_rx: Receiver<NormRawBatch>,
    enc_tx: Sender<NormEncBatch>,
    encoding: Encoding,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    while let Some(batch) = raw_rx.recv().await {
        let enc_batch: NormEncBatch = spawn_blocking(move || {
            batch
                .into_par_iter()
                .filter_map(|(tile_id, data)| match encode_one(data, encoding, cfg) {
                    Ok(mlt) => Some((tile_id, mlt)),
                    Err(e) => {
                        eprintln!("warning: skipping image {tile_id}: {e:#}");
                        None
                    }
                })
                .collect()
        })
        .await?;

        if !enc_batch.is_empty() {
            enc_tx
                .send(enc_batch)
                .await
                .map_err(|_| anyhow!("writer stage closed unexpectedly"))?;
        }
    }
    Ok(())
}

/// Stage 3 (Normalized): bulk-insert encoded images into `dst.images`.
#[hotpath::measure]
async fn normalized_write_images(
    dst_conn: &mut SqliteConnection,
    mut enc_rx: Receiver<NormEncBatch>,
    bar: &ProgressBar,
) -> AnyResult<usize> {
    let mut total = 0usize;

    while let Some(batch) = enc_rx.recv().await {
        let n = batch.len();
        let mut tx = dst_conn.begin().await?;
        for (tile_id, mlt) in batch {
            sqlx::query("INSERT OR REPLACE INTO images (tile_id, tile_data) VALUES (?, ?)")
                .bind(tile_id)
                .bind(mlt)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        total += n;
        bar.inc(n as u64);
    }

    Ok(total)
}

/// Copy all metadata rows from source to destination, replacing existing keys.
#[hotpath::measure]
async fn copy_metadata(
    src_conn: &mut SqliteConnection,
    dst_conn: &mut SqliteConnection,
) -> AnyResult<()> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as("SELECT name, value FROM metadata")
        .fetch_all(&mut *src_conn)
        .await?;

    for (name, value) in rows {
        sqlx::query("INSERT OR REPLACE INTO metadata (name, value) VALUES (?, ?)")
            .bind(name)
            .bind(value)
            .execute(&mut *dst_conn)
            .await?;
    }
    Ok(())
}

/// Bulk-copy `src.map` into `dst.map` using sqlite's ATTACH DATABASE.
#[hotpath::measure]
async fn copy_map_via_attach(dst_conn: &mut SqliteConnection, src_path: &Path) -> AnyResult<()> {
    let src_path_str = src_path
        .to_str()
        .ok_or_else(|| anyhow!("source path is not valid UTF-8: {}", src_path.display()))?;

    sqlx::query("ATTACH DATABASE ? AS norm_src")
        .bind(src_path_str)
        .execute(&mut *dst_conn)
        .await?;

    sqlx::query(
        "INSERT OR REPLACE INTO main.map (zoom_level, tile_column, tile_row, tile_id) \
         SELECT zoom_level, tile_column, tile_row, tile_id FROM norm_src.map",
    )
    .execute(&mut *dst_conn)
    .await?;

    sqlx::query("DETACH DATABASE norm_src")
        .execute(&mut *dst_conn)
        .await?;

    Ok(())
}
