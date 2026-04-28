use std::{fs::File, path::Path};

use anyhow::{Ok, Result as AnyResult, anyhow, bail};
use futures::StreamExt;
use martin_tile_utils::{Encoding, Format};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{PmTilesWriter, TileType};

use crate::convert::{ConvertArgs, SortMode, TileFormat};

use super::{MbtFormat, encode_one};

async fn get_encoding_type(input: &Path) -> AnyResult<(Encoding, MbtType)> {
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
    Ok((tile_info.encoding, src_type))
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
    let (encoding, src_type) = get_encoding_type(input).await?;
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
    let (encoding, _) = get_encoding_type(input).await?;
    let mbt = Mbtiles::new(input)?;
    let mut conn: sqlx::SqliteConnection = mbt.open_readonly().await?;
    let mut stream = mbt.stream_tiles(&mut conn);

    let file = File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mlt).create(file).unwrap();
    while let Some(tile) = stream.next().await {
        let (coord, data) = tile?;

        if let Some(data) = data {
            let coord = pmtiles::TileCoord::new(coord.z, coord.x, coord.y).unwrap();
            let enc_data = encode_one(data, encoding, cfg)?;
            writer.add_tile(coord, &enc_data[..])?;
        }
    }
    writer.finalize()?;

    Ok(())
}
