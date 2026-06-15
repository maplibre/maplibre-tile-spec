use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use super::ContainerFormat;
use super::common::{
    EncodeCache, EncodedTile, TileStats, encode_tile, make_encode_cache, make_progress_bar,
};
use anyhow::{Result as AnyResult, bail};
use bytes::Bytes;
use futures::TryStreamExt;
use martin_tile_utils::{Encoding, Format};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{
    AsyncPmTilesReader, Compression, HashMapCache, MmapBackend, PmTilesWriter, TileCoord, TileId,
    TileType,
};

/// Re-encode a `.pmtiles` input (MVT) into the requested container.
pub async fn convert(
    input: &Path,
    output: (&Path, ContainerFormat),
    cfg: EncoderConfig,
) -> AnyResult<()> {
    match output {
        (output, ContainerFormat::Pmtiles) => convert_pmtiles_to_pmtiles(input, output, cfg).await,
        (output, _) => bail!(
            "Output must be a .pmtiles file when input is a .pmtiles file, got: {}",
            output.display()
        ),
    }
}

/// An mmap-backed reader over a local `.pmtiles` file. A [`HashMapCache`] keeps
/// decoded leaf directories resident, so converting many tiles doesn't re-walk
/// and re-decompress the same directories on every `get_tile`.
type PmReader = AsyncPmTilesReader<MmapBackend, HashMapCache>;

/// Map a `PMTiles` tile compression to the [`Encoding`] that `encode_one`
/// understands. `PMTiles` has no zlib/deflate variant.
fn compression_to_encoding(compression: Compression) -> AnyResult<Encoding> {
    match compression {
        Compression::None => Ok(Encoding::Uncompressed),
        Compression::Gzip => Ok(Encoding::Gzip),
        Compression::Brotli => Ok(Encoding::Brotli),
        Compression::Zstd => Ok(Encoding::Zstd),
        Compression::Unknown => bail!("input .pmtiles uses an unknown tile compression"),
    }
}

/// Copy the source metadata JSON, overriding `format` to MLT.
fn mlt_pmtiles_metadata(metadata: &str) -> AnyResult<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(metadata).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "format".into(),
            serde_json::Value::String(Format::Mlt.metadata_format_value().into()),
        );
    }
    Ok(serde_json::to_string(&value)?)
}

/// Open a local `.pmtiles`, require MVT tiles, and report its tile [`Encoding`].
async fn open_mvt_pmtiles(input: &Path) -> AnyResult<(Arc<PmReader>, Encoding)> {
    let reader =
        Arc::new(AsyncPmTilesReader::new_with_cached_path(HashMapCache::default(), input).await?);
    let header = reader.get_header();
    if header.tile_type != TileType::Mvt {
        bail!(
            "Expected MVT tiles, got {:?} in {}",
            header.tile_type,
            input.display()
        );
    }
    let encoding = compression_to_encoding(header.tile_compression)?;
    Ok((reader, encoding))
}

/// Flatten the archive's run-length data entries into individual tile ids.
async fn collect_pmtiles_ids(reader: &Arc<PmReader>) -> AnyResult<Vec<TileId>> {
    let mut ids = Vec::new();
    let mut entries = reader.clone().entries();
    while let Some(entry) = entries.try_next().await? {
        ids.extend(entry.iter_coords());
    }
    Ok(ids)
}

/// How many tiles may be in flight (read, encoding, or buffered awaiting
/// in-order emission) per CPU. Bounds memory on huge archives while keeping
/// every core fed. Tile encode times vary by orders of magnitude (a dense city
/// tile vs. an empty ocean tile), so a deep window is needed to keep all cores
/// busy while a few slow tiles are in flight ahead of in-order emission.
const PIPELINE_DEPTH_PER_CORE: usize = 32;

/// How often to emit a plain-text progress line when the live bar is hidden
/// (non-terminal stderr). Large enough to stay quiet in logs, small enough to
/// give a useful ETA on multi-hour runs.
const PROGRESS_LOG_EVERY: u64 = 5_000_000;

/// Emit one plain-text progress line (used when the live bar is hidden).
#[expect(
    clippy::cast_precision_loss,
    reason = "tile counts and rates are approximate progress reporting"
)]
fn log_progress_line(done: u64, total: u64, elapsed: std::time::Duration) {
    let rate = done as f64 / elapsed.as_secs_f64().max(f64::EPSILON);
    let eta_min = total.saturating_sub(done) as f64 / rate / 60.0;
    eprintln!("  {done}/{total} tiles ({rate:.0}/s, eta {eta_min:.0} min)");
}

/// Re-encode every tile MVT → MLT, deduplicating through `cache`, and deliver
/// the results **in ascending tile-id order** so a [`PmTilesWriter`] can keep
/// the archive clustered and run-length encoded.
///
/// The work runs on cooperating threads, none of which is the caller's async
/// runtime: a reader walks the (already ascending) `ids` and pulls raw MVT off
/// the mmap; a pool of `parallelism` worker threads drains those (via a
/// lock-free MPMC channel) and encodes in parallel; and an emitter reorders the
/// unordered encode results back into id order. A pool of `cap` permits flows
/// backpressure from the consumer all the way to the reader, so at most `cap`
/// tiles are ever resident.
fn spawn_encode_pipeline(
    reader: Arc<PmReader>,
    ids: Vec<TileId>,
    encoding: Encoding,
    cfg: EncoderConfig,
    cache: EncodeCache,
) -> tokio::sync::mpsc::Receiver<AnyResult<EncodedTile>> {
    let parallelism = thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let cap = (parallelism * PIPELINE_DEPTH_PER_CORE).max(8);

    // Ordered output to the (async) consumer. The real backpressure is the
    // permit pool below, so this buffer only needs to cover one wave of cores.
    let (out_tx, out_rx) = tokio::sync::mpsc::channel(parallelism);

    thread::spawn(move || {
        // `cap` permits cap how far the reader runs ahead of in-order emission.
        let (tok_tx, tok_rx) = mpsc::channel::<()>();
        for _ in 0..cap {
            tok_tx.send(()).expect("permit receiver alive");
        }
        // Raw tiles: reader → encoder worker pool. A lock-free MPMC channel lets
        // every worker pull independently; `par_bridge` instead funnels all
        // pulls through one mutex, which caps useful parallelism well below the
        // core count once per-tile encode work is non-trivial.
        let (raw_tx, raw_rx) = crossbeam_channel::unbounded::<(usize, TileCoord, Bytes)>();
        // Encoded tiles: encoders → the in-order emitter (this thread).
        let (res_tx, res_rx) = mpsc::channel::<AnyResult<(usize, EncodedTile)>>();

        // Reader: one sequential pass over the ascending ids. A permit is taken
        // only for tiles that exist, so `seq` stays gap-free and skipped ids
        // never stall the emitter.
        let reader_thread = {
            let res_tx = res_tx.clone();
            thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = res_tx.send(Err(e.into()));
                        return;
                    }
                };
                rt.block_on(async move {
                    let mut seq = 0usize;
                    for id in ids {
                        let coord: TileCoord = id.into();
                        match reader.get_tile(id).await {
                            Ok(Some(data)) => {
                                // Throttle before handing the tile downstream.
                                if tok_rx.recv().is_err() {
                                    return; // consumer gone
                                }
                                if raw_tx.send((seq, coord, data)).is_err() {
                                    return;
                                }
                                seq += 1;
                            }
                            Ok(None) => {} // no tile for this id; nothing to emit
                            Err(e) => {
                                let _ = res_tx.send(Err(e.into()));
                                return;
                            }
                        }
                    }
                });
            })
        };

        // Encoders: a fixed pool of `parallelism` workers, each draining raw
        // tiles from the shared MPMC channel and deduping the small ones.
        let encoder_threads: Vec<_> = (0..parallelism)
            .map(|_| {
                let raw_rx = raw_rx.clone();
                let res_tx = res_tx.clone();
                let cache = cache.clone();
                thread::spawn(move || {
                    for (seq, coord, data) in raw_rx {
                        let bytes_in = data.len() as u64;
                        let result =
                            encode_tile(&cache, &data, encoding, cfg).map(|(data, hit)| {
                                (
                                    seq,
                                    EncodedTile {
                                        coord,
                                        data,
                                        bytes_in,
                                        hit,
                                    },
                                )
                            });
                        if res_tx.send(result).is_err() {
                            break; // emitter gone
                        }
                    }
                })
            })
            .collect();
        // Only the workers' clones should keep the raw channel open.
        drop(raw_rx);
        // Drop the engine's own sender so `res_rx` closes once the reader and
        // every encoder worker have dropped their clones.
        drop(res_tx);

        // Emitter: reorder by `seq` and forward in ascending order, returning a
        // permit for every tile sent on so the reader can advance.
        let mut next = 0usize;
        let mut buffer: BTreeMap<usize, EncodedTile> = BTreeMap::new();
        for msg in res_rx {
            let send = match msg {
                Ok((seq, tile)) => {
                    buffer.insert(seq, tile);
                    let mut consumer_gone = false;
                    while let Some(tile) = buffer.remove(&next) {
                        if out_tx.blocking_send(Ok(tile)).is_err() {
                            consumer_gone = true;
                            break;
                        }
                        let _ = tok_tx.send(());
                        next += 1;
                    }
                    !consumer_gone
                }
                Err(e) => out_tx.blocking_send(Err(e)).is_ok(),
            };
            if !send {
                break; // consumer dropped the receiver; tear down
            }
        }

        // Release the reader (it may be parked on a permit) before joining.
        drop(tok_tx);
        let _ = reader_thread.join();
        for t in encoder_threads {
            let _ = t.join();
        }
    });

    out_rx
}

async fn convert_pmtiles_to_pmtiles(
    input: &Path,
    output: &Path,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (reader, encoding) = open_mvt_pmtiles(input).await?;
    let ids = collect_pmtiles_ids(&reader).await?;

    eprintln!("{} -> {} (pmtiles):", input.display(), output.display());
    let start = Instant::now();
    let bar = make_progress_bar(ids.len() as u64);

    let metadata_str = mlt_pmtiles_metadata(&reader.get_metadata().await?)?;
    let file = std::fs::File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mlt)
        .metadata(&metadata_str)
        .create(file)?;

    let mut tiles = spawn_encode_pipeline(reader, ids, encoding, cfg, make_encode_cache());
    let mut stats = TileStats::default();
    // When stderr isn't a terminal (e.g. output redirected to a log) the bar
    // renders nothing, so emit a periodic plain-text progress line instead —
    // long full-planet runs to a log file still need a visible ETA.
    let log_progress = bar.is_hidden();
    let mut done: u64 = 0;
    // Tiles arrive in ascending id order, so the writer stays clustered.
    while let Some(tile) = tiles.recv().await {
        let EncodedTile {
            coord,
            data,
            bytes_in,
            hit,
        } = tile?;
        writer.add_tile(coord, &data)?;
        stats.record(data.len() as u64, bytes_in, hit);
        bar.inc(1);
        done += 1;
        if log_progress && done.is_multiple_of(PROGRESS_LOG_EVERY) {
            log_progress_line(done, bar.length().unwrap_or(done), start.elapsed());
        }
    }
    writer.finalize()?;
    bar.finish_and_clear();
    stats.print_summary(start);

    Ok(())
}
