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
use usize_cast::FromUsize as _;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use super::{EncoderConfig, TileFormat, convert_buffer, whole_rate_per_sec};

/// Only tiles below this size are cached; larger tiles rarely repeat across a tileset.
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
        self.bytes_saved
            .fetch_add(u64::from_usize(size), Ordering::Relaxed);
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
        Some("mlt" | "mvt" | "pbf")
    )
}

/// Per-walk shared state passed to [`convert_file`].
struct WalkCtx<'a> {
    base: &'a Path,
    output: &'a Path,
    cfg: EncoderConfig,
    to: TileFormat,
    cache: &'a EncodedCache,
    stats: &'a DedupStats,
}

pub fn convert(input: &Path, output: &Path, cfg: EncoderConfig, to: TileFormat) -> AnyResult<()> {
    // For a single file, use the parent so `strip_prefix` yields just the filename.
    let base = if input.is_dir() {
        input
    } else {
        input.parent().unwrap_or(Path::new("."))
    };

    let cache: EncodedCache = make_cache(CACHE_MAX_BYTES);
    let stats = DedupStats::default();
    let failed = AtomicUsize::new(0);
    let ctx = WalkCtx {
        base,
        output,
        cfg,
        to,
        cache: &cache,
        stats: &stats,
    };

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
            let result = convert_file(&in_path, &ctx);
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
        eprintln!("No .mlt, .mvt, or .pbf files found in {}", input.display());
        return Ok(());
    }
    eprintln!("{}", format_dedup_line(&stats, &cache));
    Ok(())
}

fn convert_file(file: &Path, ctx: &WalkCtx<'_>) -> AnyResult<()> {
    let rel = file
        .strip_prefix(ctx.base)
        .with_context(|| format!("stripping prefix from {}", file.display()))?;
    let out_path = ctx.output.join(rel).with_extension(ctx.to.extension());

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let buffer = fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let from = TileFormat::from_path(file);
    let err_ctx = || {
        format!(
            "converting {} {}",
            from.extension().to_uppercase(),
            file.display()
        )
    };

    if buffer.len() > MAX_TILE_TRACK_SIZE {
        let out_bytes = convert_buffer(buffer, from, ctx.to, ctx.cfg).with_context(err_ctx)?;
        ctx.stats.record_encode();
        fs::write(&out_path, &out_bytes)
            .with_context(|| format!("writing {}", out_path.display()))?;
        return Ok(());
    }

    let key = xxh3_128(&buffer);
    let entry = ctx
        .cache
        .entry(key)
        .or_try_insert_with(|| -> AnyResult<Arc<Vec<u8>>> {
            let out_bytes = convert_buffer(buffer, from, ctx.to, ctx.cfg).with_context(err_ctx)?;
            Ok(Arc::new(out_bytes))
        })
        .map_err(|e: Arc<anyhow::Error>| anyhow!("{e:#}"))?;

    let is_fresh = entry.is_fresh();
    let out_arc = entry.into_value();
    if is_fresh {
        ctx.stats.record_encode();
    } else {
        ctx.stats.record_hit(out_arc.len());
    }

    fs::write(&out_path, out_arc.as_slice())
        .with_context(|| format!("writing {}", out_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mlt_core::Parser;

    use super::*;

    const POINT_MVT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/simple/point-boolean.mvt"
    );
    const LINE_MVT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/simple/line-boolean.mvt"
    );
    const OMT_MVT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/omt/0_0_0.mvt"
    );

    // Tile field 3 (layers) holding an empty Layer, which has no name.
    const MVT_WITH_AN_UNNAMED_LAYER: &[u8] = &[0x1a, 0x00];

    static NEXT_TREE_ID: AtomicU64 = AtomicU64::new(0);

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let id = NEXT_TREE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("mlt-from-files-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&root).expect("temp root is creatable");
            Self(root)
        }

        fn join(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }

        fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
            let path = self.join(rel);
            fs::create_dir_all(path.parent().expect("temp path has a parent"))
                .expect("temp subdirectory is creatable");
            fs::write(&path, bytes).expect("temp file is writable");
            path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn output_paths(root: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                e.path()
                    .strip_prefix(root)
                    .expect("walked entry is below the root")
                    .to_path_buf()
            })
            .collect();
        paths.sort();
        paths
    }

    fn read_fixture(path: &str) -> Vec<u8> {
        fs::read(path).expect("fixture reads")
    }

    fn layer_count(tile: &[u8]) -> usize {
        Parser::default()
            .parse_layers(tile)
            .expect("converted tile parses")
            .len()
    }

    #[test]
    fn a_fresh_cache_and_stats_report_no_dedup_work() {
        let cache = make_cache(CACHE_MAX_BYTES);
        let stats = DedupStats::default();

        assert_eq!(
            format_dedup_line(&stats, &cache),
            "  dedup: 0 unique encoded, 0 cached (0.0% hit rate, ~0B of encode work skipped); \
             cache weight 0B"
        );
    }

    #[test]
    fn the_dedup_line_reports_the_hit_rate_the_saved_bytes_and_the_cache_weight() {
        let cache = make_cache(CACHE_MAX_BYTES);
        cache.insert(1, Arc::new(vec![0u8; 2000]));
        let stats = DedupStats::default();
        stats.record_encode();
        stats.record_encode();
        stats.record_encode();
        stats.record_hit(1500);

        assert_eq!(
            format_dedup_line(&stats, &cache),
            "  dedup: 3 unique encoded, 1 cached (25.0% hit rate, ~1.5kB of encode work skipped); \
             cache weight 2.0kB"
        );
    }

    #[test]
    fn the_cache_weighs_every_entry_by_its_byte_length() {
        let cache = make_cache(CACHE_MAX_BYTES);
        cache.insert(1, Arc::new(vec![0u8; 10]));
        cache.insert(2, Arc::new(vec![0u8; 30]));
        cache.run_pending_tasks();

        assert_eq!(cache.entry_count(), 2);
        assert_eq!(cache.weighted_size(), 40);
    }

    #[test]
    fn a_repeated_small_tile_is_encoded_once_and_served_from_the_cache() {
        let tree = TempTree::new();
        let tile = read_fixture(POINT_MVT);
        let first = tree.write("in/a.mvt", &tile);
        let second = tree.write("in/nested/b.mvt", &tile);
        let base = tree.join("in");
        let output = tree.join("out");
        let cache = make_cache(CACHE_MAX_BYTES);
        let stats = DedupStats::default();
        let ctx = WalkCtx {
            base: &base,
            output: &output,
            cfg: EncoderConfig::default(),
            to: TileFormat::Mlt,
            cache: &cache,
            stats: &stats,
        };

        convert_file(&first, &ctx).expect("the first tile converts");
        convert_file(&second, &ctx).expect("the second tile converts");

        assert_eq!(stats.encoded.load(Ordering::Relaxed), 1);
        assert_eq!(stats.hits.load(Ordering::Relaxed), 1);
        assert_eq!(
            output_paths(&output),
            [PathBuf::from("a.mlt"), Path::new("nested").join("b.mlt")]
        );
        let written = fs::read(output.join("a.mlt")).expect("output reads");
        assert_eq!(
            fs::read(tree.join("out/nested/b.mlt")).expect("output reads"),
            written
        );
        assert_eq!(
            stats.bytes_saved.load(Ordering::Relaxed),
            u64::from_usize(written.len())
        );
    }

    #[test]
    fn a_repeated_tile_above_the_track_size_is_converted_again_instead_of_cached() {
        let tree = TempTree::new();
        let tile = read_fixture(OMT_MVT);
        assert!(tile.len() > MAX_TILE_TRACK_SIZE);
        let first = tree.write("in/a.mvt", &tile);
        let second = tree.write("in/b.mvt", &tile);
        let base = tree.join("in");
        let output = tree.join("out");
        let cache = make_cache(CACHE_MAX_BYTES);
        let stats = DedupStats::default();
        let ctx = WalkCtx {
            base: &base,
            output: &output,
            cfg: EncoderConfig::default(),
            to: TileFormat::Mvt,
            cache: &cache,
            stats: &stats,
        };

        convert_file(&first, &ctx).expect("the first tile converts");
        convert_file(&second, &ctx).expect("the second tile converts");
        cache.run_pending_tasks();

        assert_eq!(stats.encoded.load(Ordering::Relaxed), 2);
        assert_eq!(stats.hits.load(Ordering::Relaxed), 0);
        assert_eq!(stats.bytes_saved.load(Ordering::Relaxed), 0);
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(
            output_paths(&output),
            [PathBuf::from("a.mvt"), PathBuf::from("b.mvt")]
        );
        assert_eq!(fs::read(output.join("a.mvt")).expect("output reads"), tile);
    }

    #[test]
    fn a_file_outside_the_walk_base_is_rejected_before_anything_is_written() {
        let tree = TempTree::new();
        let stray = tree.write("elsewhere/a.mvt", &read_fixture(POINT_MVT));
        let base = tree.join("in");
        let output = tree.join("out");
        let cache = make_cache(CACHE_MAX_BYTES);
        let stats = DedupStats::default();
        let ctx = WalkCtx {
            base: &base,
            output: &output,
            cfg: EncoderConfig::default(),
            to: TileFormat::Mlt,
            cache: &cache,
            stats: &stats,
        };

        let err = convert_file(&stray, &ctx).expect_err("a file outside the base cannot be mapped");

        assert_eq!(
            err.to_string(),
            format!("stripping prefix from {}", stray.display())
        );
        assert_eq!(output_paths(&output), Vec::<PathBuf>::new());
    }

    #[test]
    fn converting_a_directory_mirrors_its_layout_and_leaves_other_extensions_alone() {
        let tree = TempTree::new();
        tree.write("in/point.mvt", &read_fixture(POINT_MVT));
        tree.write("in/nested/line.pbf", &read_fixture(LINE_MVT));
        tree.write("in/notes.txt", b"not a tile");
        let input = tree.join("in");
        let output = tree.join("out");

        convert(&input, &output, EncoderConfig::default(), TileFormat::Mlt)
            .expect("the directory converts");

        assert_eq!(
            output_paths(&output),
            [
                Path::new("nested").join("line.mlt"),
                PathBuf::from("point.mlt")
            ]
        );
        assert_eq!(
            layer_count(&fs::read(output.join("point.mlt")).expect("output reads")),
            1
        );
        assert_eq!(
            layer_count(&fs::read(tree.join("out/nested/line.mlt")).expect("output reads")),
            1
        );
    }

    #[test]
    fn converting_a_single_file_writes_one_tile_into_the_output_directory() {
        let tree = TempTree::new();
        let input = tree.write("in/point.mvt", &read_fixture(POINT_MVT));
        let output = tree.join("out");

        convert(&input, &output, EncoderConfig::default(), TileFormat::Mlt)
            .expect("the file converts");

        assert_eq!(output_paths(&output), [PathBuf::from("point.mlt")]);
        assert_eq!(
            layer_count(&fs::read(output.join("point.mlt")).expect("output reads")),
            1
        );
    }

    #[test]
    fn converting_a_directory_holding_no_tiles_writes_nothing_and_succeeds() {
        let tree = TempTree::new();
        tree.write("in/notes.txt", b"not a tile");
        let input = tree.join("in");
        let output = tree.join("out");

        convert(&input, &output, EncoderConfig::default(), TileFormat::Mlt)
            .expect("a directory without tiles is not an error");

        assert_eq!(output_paths(&output), Vec::<PathBuf>::new());
    }

    #[test]
    fn a_tile_that_fails_to_convert_is_counted_and_fails_the_whole_walk() {
        let tree = TempTree::new();
        tree.write("in/broken.mvt", MVT_WITH_AN_UNNAMED_LAYER);
        tree.write("in/point.mvt", &read_fixture(POINT_MVT));
        let input = tree.join("in");
        let output = tree.join("out");

        let err = convert(&input, &output, EncoderConfig::default(), TileFormat::Mlt)
            .expect_err("a broken tile fails the walk");

        assert_eq!(err.to_string(), "1 file(s) failed to convert");
        assert_eq!(output_paths(&output), [PathBuf::from("point.mlt")]);
    }
}
