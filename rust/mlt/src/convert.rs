use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result as AnyResult, anyhow, bail};
use clap::{Args, ValueEnum};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use martin_tile_utils::{Encoding, Format, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder, NormalizedSchema};
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig};
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::{Decoder, Layer, Parser};
use moka::sync::Cache;
use rayon::iter::{ParallelBridge as _, ParallelIterator as _};
use size_format::SizeFormatterSI;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use crate::ls::is_mlt_extension;

/// Maximum input-tile size (bytes) for which we consult the dedup cache
/// (file-based conversion path only).
const MAX_TILE_TRACK_SIZE: usize = 1024;

/// Maximum aggregate weight (encoded-tile bytes) held by the dedup cache
/// (file-based conversion path only).
const CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Shared cache type: maps an input-identity key to the encoded MLT bytes.
type EncodedCache = Cache<u128, Arc<Vec<u8>>>;

/// Construct a weighted [`EncodedCache`] bounded by roughly `max_bytes` of
/// encoded-tile payload.
fn make_cache(max_bytes: u64) -> EncodedCache {
    Cache::builder()
        .max_capacity(max_bytes)
        .weigher(|_key, value: &Arc<Vec<u8>>| u32::try_from(value.len()).unwrap_or(u32::MAX))
        .build()
}

/// Counters kept across the encoding stage for end-of-run dedup reporting
/// (file-based conversion path only).
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

/// Format a progress bar's throughput as a whole-number `{rate}/s`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "state.per_sec() is always non-negative and well below 2^63 tiles/sec"
)]
fn whole_rate_per_sec(state: &ProgressState, w: &mut dyn std::fmt::Write) {
    let _ = w.write_fmt(format_args!("{}/s", state.per_sec() as u64));
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
            MbtFormat::Normalized => Self::Normalized {
                hash_view: true,
                schema: NormalizedSchema::DedupId,
            },
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

/// Use `MbtilesTranscoder` to run the 3-stage pipeline that decompresses
/// each MVT tile via `encode_one` and writes the resulting MLT tiles to the
/// destination `.mbtiles` file.
async fn convert_mbtiles_async(args: &ConvertArgs, cfg: EncoderConfig) -> AnyResult<()> {
    // ── Validate source ──────────────────────────────────────────────────────
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

    let src_type = src.detect_type(&mut src_conn).await?;
    let mbt_type = args.mbtiles_format.clone().map_or(src_type, MbtType::from);
    let encoding = tile_info.encoding;

    // Done with the source connection — the transcoder opens its own.
    drop(src_conn);

    eprintln!(
        "{} → {} ({mbt_type}):",
        args.input.display(),
        args.output.display()
    );

    // ── Run transcoder ───────────────────────────────────────────────────────
    let mut transcoder =
        MbtilesTranscoder::new(args.input.clone(), args.output.clone(), move |data| {
            encode_one(data, encoding, cfg)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })
        });
    if mbt_type != src_type {
        transcoder = transcoder.dst_type(mbt_type);
    }

    let stats = transcoder.run().await?;

    // ── Set format metadata to MLT ───────────────────────────────────────────
    let dst = Mbtiles::new(&args.output)?;
    let mut dst_conn = dst.open_or_new().await?;
    dst.set_metadata_value(&mut dst_conn, "format", Format::Mlt.metadata_format_value())
        .await?;

    eprintln!(
        "  done: {} tiles ({} unique encoded, {} cache hits)",
        stats.tiles_written, stats.cache_encoded, stats.cache_hits,
    );

    Ok(())
}
