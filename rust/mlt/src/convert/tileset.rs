use std::path::Path;

use anyhow::{Result as AnyResult, anyhow, bail};
use martin_tile_utils::Format;
use mbtiles::{Mbtiles, MbtilesTranscoder};
use mlt_core::encoder::EncoderConfig;

use super::{MbtFormat, encode_one};

pub async fn convert_mbtiles(
    input: &Path,
    output: &Path,
    mbtiles_format: Option<MbtFormat>,
    cfg: EncoderConfig,
) -> AnyResult<()> {
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
    let mbt_type = mbtiles_format.map_or(src_type, Into::into);
    let encoding = tile_info.encoding;

    // The transcoder opens its own connection.
    drop(src_conn);

    eprintln!("{} → {} ({mbt_type}):", input.display(), output.display());

    let mut transcoder =
        MbtilesTranscoder::new(input, output, move |data| {
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
