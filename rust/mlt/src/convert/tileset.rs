use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result as AnyResult, anyhow, bail};
use indicatif::{ProgressBar, ProgressStyle};
use martin_tile_utils::Format;
use mbtiles::{Mbtiles, MbtilesTranscoder};
use mlt_core::encoder::EncoderConfig;
use size_format::SizeFormatterSI;

use super::{MbtFormat, encode_one, whole_rate_per_sec};

#[derive(Default)]
struct EncodeSizes {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
}

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

    let count_table = match src_type.normalized_schema() {
        Some(schema) => schema.content_table(),
        None if matches!(src_type, mbtiles::MbtType::FlatWithHash) => "tiles_with_hash",
        None => "tiles",
    };
    #[expect(clippy::cast_sign_loss, reason = "COUNT(*) is always non-negative")]
    let total: u64 =
        sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {count_table}"))
            .fetch_one(&mut src_conn)
            .await? as u64;

    // The transcoder opens its own connection.
    drop(src_conn);

    eprintln!("{} → {} ({mbt_type}):", input.display(), output.display());

    let start = Instant::now();
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "  {bar:40.cyan/blue} {pos}/{len} tiles [{rate}, eta {eta}]",
            )
            .expect("invalid bar template")
            .with_key("rate", whole_rate_per_sec),
    );
    bar.enable_steady_tick(Duration::from_millis(200));

    let bar_ref = bar.clone();
    let sizes = Arc::new(EncodeSizes::default());
    let sizes_ref = Arc::clone(&sizes);

    let mut transcoder = MbtilesTranscoder::new(input, output, move |data| {
        sizes_ref.bytes_in.fetch_add(data.len() as u64, Ordering::Relaxed);
        let result = encode_one(data, encoding, cfg)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() });
        if let Ok(ref encoded) = result {
            sizes_ref.bytes_out.fetch_add(encoded.len() as u64, Ordering::Relaxed);
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
