use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

use anyhow::{Result as AnyResult, anyhow, bail};
use futures::StreamExt;
use martin_tile_utils::{Encoding, Format};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder, Metadata};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use size_format::SizeFormatterSI;
use usize_cast::FromUsize as _;

use super::common::{
    ENCODE_CACHE_BYTES, EncodedTile, MAX_TILE_CACHE_TRACK_SIZE_BYTES, TileStats, encode_tile,
    make_encode_cache, make_progress_bar,
};
use super::{
    ContainerFormat, MbtFormat, PmtilesTileCompression, encode_one, update_mlt_pmtiles_metadata,
};

/// Re-encode an `.mbtiles` input (MVT) into the requested container.
pub async fn convert(
    input: &Path,
    output: (&Path, ContainerFormat),
    cfg: EncoderConfig,
    mbtiles_format: Option<MbtFormat>,
    tile_compression: PmtilesTileCompression,
) -> AnyResult<()> {
    match output {
        (output, ContainerFormat::Mbtiles) => {
            convert_mbtiles_to_mbtiles(input, output, mbtiles_format, cfg).await
        }
        (output, ContainerFormat::Pmtiles) => {
            convert_mbtiles_to_pmtiles(input, output, cfg, tile_compression).await
        }
        (output, ContainerFormat::Files) => bail!(
            "Output must be either an .mbtiles or a .pmtiles file when input is an .mbtiles file, got: {}",
            output.display()
        ),
    }
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

async fn convert_mbtiles_to_mbtiles(
    input: &Path,
    output: &Path,
    mbtiles_format: Option<MbtFormat>,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (encoding, src_type, _, total) = get_metadata(input).await?;
    let mbt_type = mbtiles_format.map_or(src_type, Into::into);

    eprintln!("{} -> {} ({mbt_type}):", input.display(), output.display());

    let start = Instant::now();
    let bar = make_progress_bar(total);

    let bar_ref = bar.clone();
    let sizes = Arc::new(EncodeSizes::default());
    let sizes_ref = Arc::clone(&sizes);

    let mut transcoder = MbtilesTranscoder::new(input, output, move |data| {
        sizes_ref
            .bytes_in
            .fetch_add(u64::from_usize(data.len()), Ordering::Relaxed);
        let result = encode_one(data, encoding, cfg)
            .map(|(data, _raw_mvt_size)| data)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() });
        if let Ok(ref encoded) = result {
            sizes_ref
                .bytes_out
                .fetch_add(u64::from_usize(encoded.len()), Ordering::Relaxed);
        }
        bar_ref.inc(1);
        result
    })
    .batch_size(500)
    .cache_max_bytes(ENCODE_CACHE_BYTES)
    .max_tile_track_size(MAX_TILE_CACHE_TRACK_SIZE_BYTES)
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
        "  converted {} tiles ({} unique encoded, {} cache hits, {:.1}B -> {:.1}B) in {:.1?}",
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
    tile_compression: PmtilesTileCompression,
) -> AnyResult<()> {
    // FIXME: add a fastpath for normalised schemas. We don't need to cache them
    let (encoding, _, metadata, total) = get_metadata(input).await?;
    let tile_compression = tile_compression.resolve(encoding)?;
    let input_archive_size = std::fs::metadata(input)?.len();

    eprintln!("{} -> {} (pmtiles):", input.display(), output.display());

    let start = Instant::now();
    let bar = make_progress_bar(total);

    let file = std::fs::File::create(output)?;
    let mut metadata_json = serde_json::to_value(&metadata.tilejson)?;
    let metadata_obj = metadata_json
        .as_object_mut()
        .ok_or_else(|| anyhow!("MBTiles metadata must serialize to a JSON object"))?;
    update_mlt_pmtiles_metadata(metadata_obj, tile_compression);
    let metadata_str = serde_json::to_string(&metadata_json)?;
    let mut stream_writer = PmTilesWriter::new(TileType::Mlt)
        .tile_compression(tile_compression)
        .metadata(&metadata_str)
        .create(file)?;

    let parallelism = thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);

    let cache = make_encode_cache();

    let mbt = Mbtiles::new(input)?;
    let mut conn = mbt.open_readonly().await?;
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
            let cache = cache.clone();
            tokio::task::spawn_blocking(move || -> AnyResult<EncodedTile> {
                let (data, raw_mvt_size, hit) = encode_tile(&cache, &data, encoding, cfg)?;
                Ok(EncodedTile {
                    coord,
                    data,
                    raw_mvt_size,
                    hit,
                })
            })
        })
        .buffer_unordered((parallelism - 1).max(1));
    tokio::pin!(encoded);

    let mut stats = TileStats::default();
    while let Some(joined) = encoded.next().await {
        let EncodedTile {
            coord,
            data,
            raw_mvt_size,
            hit,
        } = joined??;
        stream_writer.add_tile(coord, &data)?;
        stats.record(data.len() as u64, raw_mvt_size, hit);
        bar.inc(1);
    }

    stream_writer.finalize()?;
    let output_archive_size = std::fs::metadata(output)?.len();
    bar.finish_and_clear();
    stats.print_summary(
        start,
        input_archive_size,
        output_archive_size,
        encoding,
        tile_compression,
    );

    Ok(())
}
