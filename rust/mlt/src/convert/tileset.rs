use std::{path::Path, sync::Arc, thread};

use anyhow::{Result as AnyResult, anyhow, bail};
use bytes::Bytes;
use futures::StreamExt;
use martin_tile_utils::{Encoding, Format};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder, Metadata};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{PmTilesWriter, TileCoord, TileType};

use crate::convert::{ConvertArgs, SortMode, TileFormat};

use super::{MbtFormat, encode_one};

async fn get_metadata(input: &Path) -> AnyResult<(Encoding, MbtType, Metadata)> {
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
    Ok((tile_info.encoding, src_type, meta))
}

pub async fn convert_tiles(args: &ConvertArgs) -> AnyResult<()> {
    let cfg = EncoderConfig {
        tessellate: args.tessellate,
        try_spatial_morton_sort: matches!(args.sort, SortMode::Auto | SortMode::Morton),
        try_spatial_hilbert_sort: matches!(args.sort, SortMode::Auto | SortMode::Hilbert),
        try_id_sort: matches!(args.sort, SortMode::Auto | SortMode::Id),
        allow_shared_dict: !args.no_shared_dict,
        ..Default::default()
    };

    match (args.input_format(), args.output_format()) {
        (Some(TileFormat::Mbtiles), Some(TileFormat::Mbtiles)) => {
            convert_mbtiles_to_mbtiles(&args.input, &args.output, args.mbtiles_format.clone(), cfg)
                .await?;
        }
        (Some(TileFormat::Mbtiles), Some(TileFormat::Pmtiles)) => {
            convert_mbtiles_to_pmtiles(&args.input, &args.output, cfg).await?;
        }
        _ => bail!("Conversion formats not supported yet"),
    }

    Ok(())
}

async fn convert_mbtiles_to_mbtiles(
    input: &Path,
    output: &Path,
    mbtiles_format: Option<MbtFormat>,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (encoding, src_type, _) = get_metadata(input).await?;
    let mbt_type = mbtiles_format.map_or(src_type, Into::into);

    eprintln!("{} → {} ({mbt_type}):", input.display(), output.display());

    let mut transcoder = MbtilesTranscoder::new(input, output, move |data| {
        encode_one(data, encoding, cfg)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })
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

    // The transcoder copies source metadata; override `format` to MLT.
    let dst = Mbtiles::new(output)?;
    let mut dst_conn = dst.open_or_new().await?;
    dst.set_metadata_value(&mut dst_conn, "format", Format::Mlt.metadata_format_value())
        .await?;

    eprintln!(
        "  done: {} tiles ({} unique encoded, {} cache hits)",
        stats.tiles_written, stats.cache_encoded, stats.cache_hits,
    );

    Ok(())
}

async fn convert_mbtiles_to_pmtiles(
    input: &Path,
    output: &Path,
    cfg: EncoderConfig,
) -> AnyResult<()> {
    let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<(TileCoord, Vec<u8>)>(500);
    let (final_tx, mut final_rx) = tokio::sync::mpsc::channel::<(TileCoord, Bytes)>(500);

    // Read from .mbtiles
    let mbt = Mbtiles::new(input)?;
    let mut conn = mbt.open_readonly().await?;
    tokio::spawn(async move {
        let mut stream = mbt.stream_tiles(&mut conn);
        while let Some(tile_result) = stream.next().await {
            match tile_result {
                Ok((coord, Some(data))) => {
                    if let Ok(c) = TileCoord::new(coord.z, coord.x, coord.y) {
                        if raw_tx.send((c, data)).await.is_err() {
                            break;
                        }
                    }
                }
                Ok((_, None)) => continue,
                Err(e) => {
                    eprintln!("Database stream error: {}", e);
                    break;
                }
            }
        }
    });

    // Encode to mlt
    let (encoding, _, mut metadata) = get_metadata(input).await?;
    let raw_rx = Arc::new(tokio::sync::Mutex::new(raw_rx));
    let num_cpus = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    for _ in 0..num_cpus {
        let input = raw_rx.clone();
        let output = final_tx.clone();

        tokio::spawn(async move {
            while let Some((coord, data)) = {
                let mut lock = input.lock().await;
                lock.recv().await
            } {
                if let Ok(enc_data) = encode_one(data, encoding, cfg) {
                    output.send((coord, enc_data)).await.ok();
                }
            }
        });
    }
    drop(final_tx);

    // Write to .pmtiles
    metadata.tilejson.other.insert(
        "format".into(),
        serde_json::Value::String(Format::Mlt.metadata_format_value().into()),
    );
    let file = std::fs::File::create(output)?;
    let metadata_str = serde_json::to_string(&metadata.tilejson)?;
    let mut stream_writer = PmTilesWriter::new(TileType::Mlt)
        .metadata(&metadata_str)
        .create(file)?;
    while let Some((coord, data)) = final_rx.recv().await {
        stream_writer.add_tile(coord, &data)?;
    }

    stream_writer.finalize()?;

    Ok(())
}
