use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result as AnyResult, anyhow, bail};
use bytes::Bytes;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use martin_tile_utils::{Encoding, Format};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder, Metadata};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use size_format::SizeFormatterSI;

use super::{MbtFormat, encode_one};
use crate::convert::{TileFormat, whole_rate_per_sec};

const PROGRESS_BAR_TEMPLATE: &str =
    "  {bar:40.cyan/blue} {pos}/{len} tiles [{rate}, eta {eta}]";

fn make_progress_bar(total: u64) -> ProgressBar {
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

#[derive(Default)]
struct EncodeSizes {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
}

async fn get_metadata(input: &Path) -> AnyResult<(Encoding, MbtType, Metadata, u64)> {
    let src = Mbtiles::new(input)?;
    let mut src_conn = src.open_readonly().await?;

    let meta = src.get_metadata(&mut src_conn).await?;
    let tile_info = src
        .detect_format(&meta.tilejson, &mut src_conn)
        .await?
        .ok_or_else(|| anyhow!("{} appears to be empty", input.display()))?;

    if tile_info.format != Format::Mvt {
        bail!(
            "Expected MVT tiles, got {} in {}",
            tile_info.format,
            input.display()
        );
    }

    let src_type = src.detect_type(&mut src_conn).await?;
    let count_table = match src_type.normalized_schema() {
        Some(schema) => schema.content_table(),
        None if matches!(src_type, MbtType::FlatWithHash) => "tiles_with_hash",
        None => "tiles",
    };
    #[expect(clippy::cast_sign_loss, reason = "COUNT(*) is always non-negative")]
    let total: u64 = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {count_table}"))
        .fetch_one(&mut src_conn)
        .await? as u64;
    Ok((tile_info.encoding, src_type, meta, total))
}

pub async fn convert_tiles(
    input: (&Path, TileFormat),
    output: (&Path, TileFormat),
    cfg: EncoderConfig,
    mbtiles_format: Option<MbtFormat>,
) -> AnyResult<()> {
    match (input, output) {
        ((input, TileFormat::Mbtiles), (output, TileFormat::Mbtiles)) => {
            convert_mbtiles_to_mbtiles(input, output, mbtiles_format, cfg).await?;
        }
        ((input, TileFormat::Mbtiles), (output, TileFormat::Pmtiles)) => {
            convert_mbtiles_to_pmtiles(input, output, cfg).await?;
        }
        ((_, from), (_, to)) => bail!("Converting from {from:?} to {to:?} not supported yet"),
    }

    Ok(())
}

async fn convert_mbtiles_to_mbtiles(
    input: &Path,
    output: &Path,
    mbtiles_format: Option<MbtFormat>,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (encoding, src_type, _, total) = get_metadata(input).await?;
    let mbt_type = mbtiles_format.map_or(src_type, Into::into);

    eprintln!("{} → {} ({mbt_type}):", input.display(), output.display());

    let start = Instant::now();
    let bar = make_progress_bar(total);

    let bar_ref = bar.clone();
    let sizes = Arc::new(EncodeSizes::default());
    let sizes_ref = Arc::clone(&sizes);

    let mut transcoder = MbtilesTranscoder::new(input, output, move |data| {
        sizes_ref
            .bytes_in
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        let result = encode_one(data, encoding, cfg)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() });
        if let Ok(ref encoded) = result {
            sizes_ref
                .bytes_out
                .fetch_add(encoded.len() as u64, Ordering::Relaxed);
        }
        bar_ref.inc(1);
        result
    })
    .batch_size(500)
    .cache_max_bytes(512 * 1024 * 1024)
    .max_tile_track_size(1024)
    .copy_metadata(true)
    .channel_buffer(4);
    if mbt_type != src_type {
        transcoder = transcoder.dst_type(mbt_type);
    }

    let stats = transcoder.run().await?;

    bar.finish_and_clear();

    // The transcoder copies source metadata; override `format` to MLT.
    let dst = Mbtiles::new(output)?;
    let mut dst_conn = dst.open_or_new().await?;
    dst.set_metadata_value(&mut dst_conn, "format", Format::Mlt.metadata_format_value())
        .await?;

    let in_bytes = sizes.bytes_in.load(Ordering::Relaxed);
    let out_bytes = sizes.bytes_out.load(Ordering::Relaxed);
    eprintln!(
        "  converted {} tiles ({} unique encoded, {} cache hits, {:.1}B → {:.1}B) in {:.1?}",
        stats.tiles_written,
        stats.cache_encoded,
        stats.cache_hits,
        SizeFormatterSI::new(in_bytes),
        SizeFormatterSI::new(out_bytes),
        start.elapsed(),
    );

    Ok(())
}

async fn convert_mbtiles_to_pmtiles(
    input: &Path,
    output: &Path,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (encoding, _, mut metadata, total) = get_metadata(input).await?;

    eprintln!("{} → {} (pmtiles):", input.display(), output.display());

    let start = Instant::now();
    let bar = make_progress_bar(total);

    metadata.tilejson.other.insert(
        "format".into(),
        serde_json::Value::String(Format::Mlt.metadata_format_value().into()),
    );
    let file = std::fs::File::create(output)?;
    let metadata_str = serde_json::to_string(&metadata.tilejson)?;
    let mut stream_writer = PmTilesWriter::new(TileType::Mlt)
        .metadata(&metadata_str)
        .create(file)?;

    let parallelism = thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);

    let mbt = Mbtiles::new(input)?;
    let mut conn = mbt.open_readonly().await?;
    // FIXME: If the input is a normalised .mbtiles, we can save work by encoding
    // each unique blob only once (use the `images` table directly).
    let encoded = mbt
        .stream_tiles(&mut conn)
        .filter_map(|r| async move {
            match r {
                Ok((coord, Some(data))) => TileCoord::new(coord.z, coord.x, coord.y)
                    .ok()
                    .map(|c| (c, data)),
                Ok((_, None)) => None,
                Err(e) => {
                    eprintln!("Database stream error: {e}");
                    None
                }
            }
        })
        .map(|(coord, data)| {
            // Encoding is CPU-bound; offload so the single-threaded runtime
            // can actually run multiple encodes in parallel.
            tokio::task::spawn_blocking(move || -> AnyResult<(TileCoord, Bytes, u64)> {
                let bytes_in = data.len() as u64;
                let encoded = encode_one(data, encoding, cfg)?;
                Ok((coord, encoded, bytes_in))
            })
        })
        .buffer_unordered(parallelism);
    tokio::pin!(encoded);

    let mut written: u64 = 0;
    let mut bytes_in: u64 = 0;
    let mut bytes_out: u64 = 0;
    while let Some(joined) = encoded.next().await {
        let (coord, data, in_size) = joined??;
        bytes_in += in_size;
        bytes_out += data.len() as u64;
        stream_writer.add_tile(coord, &data)?;
        written += 1;
        bar.inc(1);
    }

    stream_writer.finalize()?;
    bar.finish_and_clear();

    eprintln!(
        "  converted {} tiles ({:.1}B → {:.1}B) in {:.1?}",
        written,
        SizeFormatterSI::new(bytes_in),
        SizeFormatterSI::new(bytes_out),
        start.elapsed(),
    );

    Ok(())
}
