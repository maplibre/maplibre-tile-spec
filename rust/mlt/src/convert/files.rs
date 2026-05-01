use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result as AnyResult, anyhow, bail};
use indicatif::{ProgressBar, ProgressStyle};
use moka::sync::Cache;
use rayon::iter::{ParallelBridge as _, ParallelIterator as _};
use size_format::SizeFormatterSI;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use super::{EncoderConfig, convert_mlt_buffer, convert_mvt_buffer, whole_rate_per_sec};
use crate::ls::is_mlt_extension;

/// Only tiles below this size are tracked in the dedup cache, because
/// larger tiles almost never repeat across a tileset.
const MAX_TILE_TRACK_SIZE: usize = 1024;

const CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;

type EncodedCache = Cache<u128, Arc<Vec<u8>>>;

fn make_cache(max_bytes: u64) -> EncodedCache {
    Cache::builder()
        .max_capacity(max_bytes)
        .weigher(|_key, value: &Arc<Vec<u8>>| u32::try_from(value.len()).unwrap_or(u32::MAX))
        .build()
}

#[derive(Default)]
struct DedupStats {
    hits: AtomicU64,
    encoded: AtomicU64,
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

fn is_convert_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mlt" | "mvt")
    )
}

pub fn convert_files(input: &Path, output: &Path, cfg: EncoderConfig) -> AnyResult<()> {
    // For a single file, use the parent so `strip_prefix` yields just the filename.
    let base = if input.is_dir() {
        input
    } else {
        input.parent().unwrap_or(Path::new("."))
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

    // `bar.println` is a no-op when hidden (non-TTY), so fall back to stderr.
    let emit = |msg: String| {
        if bar.is_hidden() {
            eprintln!("{msg}");
        } else {
            bar.println(msg);
        }
    };

    WalkDir::new(input)
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
            let result = convert_file(&in_path, base, output, cfg, &cache, &stats);
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
        eprintln!("No .mlt or .mvt files found in {}", input.display());
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
