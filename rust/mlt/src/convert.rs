use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{fs, mem};

use anyhow::{Context as _, Result as AnyResult, anyhow, bail};
use clap::{Args, ValueEnum};
use futures::StreamExt as _;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use martin_tile_utils::{Encoding, Format, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{CopyDuplicateMode, MbtType, Mbtiles, TileCoord, init_mbtiles_schema};
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig};
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::{Decoder, Layer, Parser};
use rayon::iter::{IntoParallelIterator as _, IntoParallelRefIterator as _, ParallelIterator as _};
use sqlx::{Connection as _, SqliteConnection};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::spawn_blocking;

use crate::ls::is_mlt_extension;

/// How many raw tiles to collect before shipping a batch to the compute stage.
const BATCH_SIZE: usize = 500;

/// Raw MVT batch forwarded from the reader to the compute stage.
type RawBatch = Vec<(TileCoord, Vec<u8>)>;
/// Converted MLT batch forwarded from the compute stage to the writer.
type MltBatch = Vec<(TileCoord, Vec<u8>)>;
/// Number of in-flight batches allowed between adjacent pipeline stages (backpressure).
const CHANNEL_BUFFER: usize = 4;
/// Maximum time between forced flushes in the writer, regardless of batch fullness.
/// Keeps data safe on long-running jobs and avoids holding a huge open transaction.
const SAVE_EVERY: Duration = Duration::from_mins(1);
/// Minimum interval between progress log lines in non-interactive (non-TTY) mode.
const PROGRESS_REPORT_EVERY: Duration = Duration::from_secs(2);

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
            // hash_view=false: skip the tiles_with_hash view (not needed for writes)
            MbtFormat::Normalized => Self::Normalized { hash_view: false },
        }
    }
}

/// Which sort strategies to attempt during re-encoding.
///
/// The encoder always encodes with the original feature order as a baseline
/// and keeps whichever encoding produces the smallest output.
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
        return tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?
            .block_on(convert_mbtiles_async(args, cfg));
    }

    let mut files: Vec<PathBuf> = Vec::new();
    collect_tile_files(&args.input, &mut files)?;
    if files.is_empty() {
        eprintln!("No .mlt or .mvt files found in {}", args.input.display());
        return Ok(());
    }

    // Determine the base for computing relative paths.
    let base = if args.input.is_dir() {
        args.input.as_path()
    } else {
        args.input.parent().unwrap_or(Path::new("."))
    };

    let failed = AtomicUsize::new(0);
    files.par_iter().for_each(|file| {
        if let Err(e) = convert_file(file, base, &args.output, cfg) {
            eprintln!("error: {}: {e:#}", file.display());
            failed.fetch_add(1, Ordering::Relaxed);
        }
    });

    let n = failed.into_inner();
    if n > 0 {
        bail!("{n} file(s) failed to convert");
    }
    Ok(())
}

fn convert_file(file: &Path, base: &Path, output: &Path, cfg: EncoderConfig) -> AnyResult<()> {
    let rel = file
        .strip_prefix(base)
        .with_context(|| format!("stripping prefix from {}", file.display()))?;
    let out_path = output.join(rel).with_extension("mlt");

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let buffer = fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let out_bytes = if is_mlt_extension(file) {
        convert_mlt_buffer(&buffer, cfg)
            .with_context(|| format!("converting MLT {}", file.display()))?
    } else {
        convert_mvt_buffer(buffer, cfg)
            .with_context(|| format!("converting MVT {}", file.display()))?
    };

    fs::write(&out_path, &out_bytes).with_context(|| format!("writing {}", out_path.display()))?;

    println!("{} -> {}", file.display(), out_path.display());
    Ok(())
}

/// Recursively collect all `.mlt` and `.mvt` files under `path`.
fn collect_tile_files(path: &Path, files: &mut Vec<PathBuf>) -> AnyResult<()> {
    if path.is_dir() {
        for entry in
            fs::read_dir(path).with_context(|| format!("reading directory {}", path.display()))?
        {
            let child = entry?.path();
            if child.is_file() && is_convert_extension(&child) {
                files.push(child);
            } else if child.is_dir() {
                collect_tile_files(&child, files)?;
            }
        }
    } else if path.is_file() {
        if is_convert_extension(path) {
            files.push(path.to_path_buf());
        }
    } else {
        bail!("path does not exist: {}", path.display());
    }
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
/// Every Tag01 layer is fully decoded to [`mlt_core::TileLayer01`] and then
/// re-encoded via [`mlt_core::TileLayer01::encode`]. Unknown layer tags are
/// passed through unchanged.
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
            _ => {}
        }
    }

    Ok(out)
}

/// Convert an MVT tile to an MLT tile using automatic encoding selection.
///
/// Each MVT layer is converted to a [`mlt_core::TileLayer01`] and encoded
/// via [`mlt_core::TileLayer01::encode`].
fn convert_mvt_buffer(buffer: Vec<u8>, cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for tile in mvt_to_tile_layers(buffer)? {
        out.extend_from_slice(&tile.encode(cfg)?);
    }
    Ok(out)
}

/// Stage 1 of the pipeline: stream raw MVT tiles from the source `.mbtiles` file in
/// `BATCH_SIZE` chunks and forward each batch to the compute stage via `raw_tx`.
///
/// Dropping `raw_tx` at function exit signals EOF to the compute stage.
async fn mbtiles_reader(
    src: Mbtiles,
    mut src_conn: SqliteConnection,
    raw_tx: Sender<RawBatch>,
) -> AnyResult<()> {
    let mut stream = src.stream_tiles(&mut src_conn);
    let mut batch: RawBatch = Vec::with_capacity(BATCH_SIZE);

    while let Some(res) = stream.next().await {
        let (coord, data_opt) = res?;
        if let Some(data) = data_opt {
            batch.push((coord, data));
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

/// Stage 2 of the pipeline: receive raw MVT batches, decompress and convert each
/// tile on a blocking thread pool (rayon), then forward converted MLT batches to
/// the writer stage via `mlt_tx`.
///
/// Per-tile errors are logged as warnings; the rest of the batch is still forwarded.
/// Dropping `mlt_tx` at function exit signals EOF to the writer stage.
async fn mbtiles_compute(
    mut raw_rx: Receiver<RawBatch>,
    mlt_tx: Sender<MltBatch>,
    encoding: Encoding,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    while let Some(batch) = raw_rx.recv().await {
        let results = spawn_blocking(move || {
            batch
                .into_par_iter()
                .map(|(coord, data)| -> AnyResult<(TileCoord, Vec<u8>)> {
                    // Decompress using the encoding detected at DB-open time.
                    // This avoids per-tile magic-byte sniffing (and double decompression).
                    let mvt = match encoding {
                        Encoding::Gzip => decode_gzip(&data)?,
                        Encoding::Zlib => decode_zlib(&data)?,
                        Encoding::Brotli => decode_brotli(&data)?,
                        Encoding::Zstd => decode_zstd(&data)?,
                        Encoding::Uncompressed | Encoding::Internal => data,
                    };
                    let mlt = convert_mvt_buffer(mvt, cfg)?;
                    Ok((coord, mlt))
                })
                .collect::<Vec<_>>()
        })
        .await?;

        // Separate successes from per-tile failures (don't abort the whole run).
        let mut mlt_batch: MltBatch = Vec::with_capacity(results.len());
        for r in results {
            match r {
                Ok(tile) => mlt_batch.push(tile),
                Err(e) => eprintln!("warning: skipping tile: {e:#}"),
            }
        }

        if !mlt_batch.is_empty() {
            mlt_tx
                .send(mlt_batch)
                .await
                .map_err(|_| anyhow!("writer stage closed unexpectedly"))?;
        }
    }
    Ok(())
}

/// Flush `pending` tiles to `dst` in a single `SQLite` transaction.
///
/// Returns the number of tiles flushed (0 if `pending` was empty).
/// The caller is responsible for resetting the save timer and updating the progress bar.
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
    // insert_tiles expects (z, x, y, data); drain moves tile bytes without cloning.
    let flat: Vec<(u8, u32, u32, Vec<u8>)> =
        pending.drain(..).map(|(c, d)| (c.z, c.x, c.y, d)).collect();
    let n = flat.len();
    dst.insert_tiles(dst_conn, mbt_type, CopyDuplicateMode::Override, &flat)
        .await?;
    *total += n;
    Ok(n)
}

/// Stage 3 of the pipeline: accumulate converted MLT tiles and flush them to the
/// destination `.mbtiles` file whenever `BATCH_SIZE` tiles are pending or `SAVE_EVERY`
/// seconds have elapsed (matches martin-cp's dual-trigger strategy).
///
/// Each flush is wrapped in one `SQLite` transaction by `insert_tiles`.
/// Returns the total number of tiles written.
async fn mbtiles_writer(
    dst: Mbtiles,
    mut dst_conn: SqliteConnection,
    mut mlt_rx: Receiver<MltBatch>,
    mbt_type: MbtType,
    bar: ProgressBar,
) -> AnyResult<usize> {
    // WAL mode: readers don't block writers; much better throughput for bulk inserts.
    // The writer checkpoints + truncates the WAL before closing so no sidecar files remain.
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

    // Checkpoint + truncate WAL so no -wal/-shm sidecar files remain after close.
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

    // Use detect_format (inspects actual tile bytes, not just the metadata string)
    // to learn the tile format AND the compression encoding used in this file.
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
    // Encoding is Copy; captured by value in the compute stage.

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
    // Record that tiles are MLT (not the source MVT format).
    dst.set_metadata_value(&mut dst_conn, "format", Format::Mlt.metadata_format_value())
        .await?;

    // ── progress bar (mirrors martin-cp) ──────────────────────────────────────
    let bar = ProgressBar::new(tile_count);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "{elapsed_precise} -> eta: {eta} [{bar:40.cyan/blue} {percent}%] \
                 {human_pos}/{human_len} ({per_sec}) | {msg}",
            )
            .expect("invalid progress bar template")
            .progress_chars("█▓▒░ "),
    );
    // Use bar.println so the header line is coordinated with bar redraws on TTY.
    bar.println(format!(
        "{} → {} ({mbt_type}):",
        args.input.display(),
        args.output.display()
    ));

    // ── pipeline ──────────────────────────────────────────────────────────────
    let (raw_tx, raw_rx) =
        hotpath::channel!(channel::<RawBatch>(CHANNEL_BUFFER), label = "raw_mvt");
    let (mlt_tx, mlt_rx) =
        hotpath::channel!(channel::<MltBatch>(CHANNEL_BUFFER), label = "encoded_mlt");

    let ((), (), total) = tokio::try_join!(
        mbtiles_reader(src, src_conn, raw_tx),
        mbtiles_compute(raw_rx, mlt_tx, tile_info.encoding, cfg),
        mbtiles_writer(dst, dst_conn, mlt_rx, mbt_type, bar.clone()),
    )?;

    // In non-interactive mode the bar is hidden; print the summary line explicitly.
    if bar.is_hidden() {
        bar.println(format!(
            "  done: {total} tiles in {}",
            HumanDuration(bar.elapsed())
        ));
    }
    bar.finish_with_message(format!("done, {total} tiles"));
    Ok(())
}
