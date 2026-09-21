use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

use anyhow::{Result as AnyResult, anyhow, bail};
use futures::StreamExt;
use martin_tile_utils::{Encoding, Format};
use mbtiles::{MbtType, Mbtiles, MbtilesTranscoder, Metadata, detach_db, init_mbtiles_schema};
use mlt_core::encoder::EncoderConfig;
use pmtiles::{Compression, PmTilesWriter, TileCoord, TileType};
use size_format::SizeFormatterSI;
use tilejson::{Bounds, TileJSON};
use usize_cast::FromUsize as _;

use super::bbox::{BboxFilter, clip_bounds, clip_center};
use super::common::{
    ENCODE_CACHE_BYTES, EncodedTile, MAX_TILE_CACHE_TRACK_SIZE_BYTES, PmTilesGeography, TileStats,
    encode_tile, make_encode_cache, make_progress_bar,
};
use super::{ContainerFormat, encode_one, update_mlt_pmtiles_metadata};

/// Narrows a tileset's geography to the box a `--bbox` conversion kept.
fn clip_tilejson(tilejson: &mut TileJSON, clip: Bounds) {
    let bounds = tilejson
        .bounds
        .map_or(clip, |bounds| clip_bounds(bounds, clip));
    tilejson.center = tilejson.center.map(|center| clip_center(center, bounds));
    tilejson.bounds = Some(bounds);
}

fn geography_from_metadata(metadata: &Metadata) -> PmTilesGeography {
    let tilejson = &metadata.tilejson;
    PmTilesGeography {
        min_zoom: tilejson.minzoom,
        max_zoom: tilejson.maxzoom,
        bounds: tilejson.bounds,
        center: tilejson.center,
    }
}

/// Re-encode an `.mbtiles` input (MVT) into the requested container.
pub async fn convert(
    input: &Path,
    output: (&Path, ContainerFormat),
    cfg: EncoderConfig,
    dst_type: Option<MbtType>,
    tile_compression: Compression,
    clip: Option<Bounds>,
) -> AnyResult<()> {
    match output {
        (output, ContainerFormat::Mbtiles) => {
            convert_mbtiles_to_mbtiles(input, output, dst_type, cfg, clip).await
        }
        (output, ContainerFormat::Pmtiles) => {
            convert_mbtiles_to_pmtiles(input, output, cfg, tile_compression, clip).await
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
    let total: u64 = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM {count_table}"
    )))
    .fetch_one(&mut src_conn)
    .await? as u64;
    Ok((tile_info.encoding, src_type, meta, total))
}

async fn convert_mbtiles_to_mbtiles(
    input: &Path,
    output: &Path,
    dst_type: Option<MbtType>,
    cfg: EncoderConfig,
    clip: Option<Bounds>,
) -> AnyResult<()> {
    let (encoding, src_type, mut metadata, total) = get_metadata(input).await?;
    let mbt_type = dst_type.unwrap_or(src_type);

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
    // The copied geography still describes the whole source archive.
    if let Some(clip) = clip {
        clip_tilejson(&mut metadata.tilejson, clip);
        if let Some(bounds) = metadata.tilejson.bounds {
            dst.set_metadata_value(&mut dst_conn, "bounds", bounds.to_string())
                .await?;
        }
        if let Some(center) = metadata.tilejson.center {
            dst.set_metadata_value(&mut dst_conn, "center", center.to_string())
                .await?;
        }
    }

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
    tile_compression: Compression,
    clip: Option<Bounds>,
) -> AnyResult<()> {
    // FIXME: add a fastpath for normalised schemas. We don't need to cache them
    let (encoding, _, mut metadata, total) = get_metadata(input).await?;
    let input_archive_size = std::fs::metadata(input)?.len();

    eprintln!("{} -> {} (pmtiles):", input.display(), output.display());

    let start = Instant::now();
    let bar = make_progress_bar(total);

    // The source geography still describes every tile the `--bbox` dropped.
    if let Some(clip) = clip {
        clip_tilejson(&mut metadata.tilejson, clip);
    }
    let geography = geography_from_metadata(&metadata);
    let file = std::fs::File::create(output)?;
    let mut metadata_json = serde_json::to_value(&metadata.tilejson)?;
    let metadata_obj = metadata_json
        .as_object_mut()
        .ok_or_else(|| anyhow!("MBTiles metadata must serialize to a JSON object"))?;
    update_mlt_pmtiles_metadata(metadata_obj, tile_compression);
    let metadata_str = serde_json::to_string(&metadata_json)?;
    let mut stream_writer = geography
        .apply(PmTilesWriter::new(TileType::Mlt))
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

/// A temporary `.mbtiles` holding only the tiles a `--bbox` keeps, deleted when dropped.
///
/// Filtering in `SQLite` keeps the conversion from reading tile payloads it would discard.
pub struct BboxExtract {
    path: PathBuf,
    /// Schema of the archive the tiles came from, which the conversion keeps writing.
    pub source_type: MbtType,
}

impl BboxExtract {
    /// Copies every source tile overlapping `filter` into a temporary archive beside `output`.
    pub async fn create(input: &Path, output: &Path, filter: &BboxFilter) -> AnyResult<Self> {
        let path = output.with_extension("bbox-extract.mbtiles");
        if path.exists() {
            bail!(
                "Temporary bbox extract {} already exists; delete it first",
                path.display()
            );
        }
        let src = Mbtiles::new(input)?;
        let mut src_conn = src.open_readonly().await?;
        let source_type = src.detect_type(&mut src_conn).await?;
        drop(src_conn);

        let start = Instant::now();
        // Constructed before the work so that a failure cleans the partial file up.
        let extract = Self { path, source_type };
        let dst = Mbtiles::new(&extract.path)?;
        let mut conn = dst.open_or_new().await?;
        init_mbtiles_schema(&mut conn, MbtType::Flat, false).await?;
        src.attach_to(&mut conn, "src").await?;
        sqlx::query(
            "INSERT OR REPLACE INTO metadata (name, value) SELECT name, value FROM src.metadata",
        )
        .execute(&mut conn)
        .await?;
        let copied = sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO tiles (zoom_level, tile_column, tile_row, tile_data) \
             SELECT zoom_level, tile_column, tile_row, tile_data FROM src.tiles WHERE {}",
            filter.mbtiles_where()
        )))
        .execute(&mut conn)
        .await?
        .rows_affected();
        detach_db(&mut conn, "src").await?;
        drop(conn);

        if copied == 0 {
            bail!(
                "--bbox {} selected no tiles from {}",
                filter.bounds(),
                input.display()
            );
        }
        eprintln!(
            "{} -> {} (bbox extract):",
            input.display(),
            extract.path.display()
        );
        eprintln!(
            "  extracted {copied} tiles within {} in {:.1?}",
            filter.bounds(),
            start.elapsed()
        );
        Ok(extract)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BboxExtract {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut path = self.path.clone().into_os_string();
            path.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(path));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mlt_core::Parser;
    use pmtiles::{AsyncPmTilesReader, HashMapCache};
    use tilejson::Center;

    use super::*;

    const POINT_MVT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/simple/point-boolean.mvt"
    ));
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempArchive(PathBuf);

    impl TempArchive {
        fn new(extension: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!(
                "mlt-from-mbtiles-test-{}-{id}.{extension}",
                std::process::id()
            )))
        }
    }

    impl Drop for TempArchive {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm", "-journal"] {
                let mut path = self.0.clone().into_os_string();
                path.push(suffix);
                let _ = fs::remove_file(PathBuf::from(path));
            }
        }
    }

    async fn write_source(
        path: &Path,
        mbt_type: MbtType,
        tiles: &[(i64, i64, i64, &[u8])],
        metadata: &[(&str, &str)],
    ) {
        let mbt = Mbtiles::new(path).expect("archive path is usable");
        let mut conn = mbt.open_or_new().await.expect("archive opens");
        init_mbtiles_schema(&mut conn, mbt_type, false)
            .await
            .expect("schema initialises");
        let insert = if matches!(mbt_type, MbtType::FlatWithHash) {
            "INSERT INTO tiles_with_hash (zoom_level, tile_column, tile_row, tile_data, tile_hash) \
             VALUES (?, ?, ?, ?, '')"
        } else {
            "INSERT INTO tiles (zoom_level, tile_column, tile_row, tile_data) VALUES (?, ?, ?, ?)"
        };
        for &(zoom, column, row, data) in tiles {
            sqlx::query(insert)
                .bind(zoom)
                .bind(column)
                .bind(row)
                .bind(data.to_vec())
                .execute(&mut conn)
                .await
                .expect("tile inserts");
        }
        for &(name, value) in metadata {
            mbt.set_metadata_value(&mut conn, name, value)
                .await
                .expect("metadata inserts");
        }
    }

    async fn write_two_tile_source(path: &Path, metadata: &[(&str, &str)]) {
        write_source(
            path,
            MbtType::Flat,
            &[(0, 0, 0, POINT_MVT), (1, 1, 1, POINT_MVT)],
            metadata,
        )
        .await;
    }

    async fn output_tiles(path: &Path) -> Vec<(i64, i64, i64, Vec<u8>)> {
        let mbt = Mbtiles::new(path).expect("output opens");
        let mut conn = mbt.open_readonly().await.expect("output connects");
        sqlx::query_as(
            "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles ORDER BY 1, 2, 3",
        )
        .fetch_all(&mut conn)
        .await
        .expect("output is queryable")
    }

    async fn output_metadata_value(path: &Path, key: &str) -> Option<String> {
        let mbt = Mbtiles::new(path).expect("output opens");
        let mut conn = mbt.open_readonly().await.expect("output connects");
        mbt.get_metadata_value(&mut conn, key)
            .await
            .expect("metadata is readable")
    }

    async fn open_pmtiles(path: &Path) -> AsyncPmTilesReader<pmtiles::MmapBackend, HashMapCache> {
        AsyncPmTilesReader::new_with_cached_path(HashMapCache::default(), path)
            .await
            .expect("output opens")
    }

    #[tokio::test]
    async fn reads_the_encoding_schema_and_tile_count_of_a_flat_archive() {
        let source = TempArchive::new("mbtiles");
        write_two_tile_source(&source.0, &[("name", "tiny"), ("format", "pbf")]).await;

        let (encoding, src_type, metadata, total) =
            get_metadata(&source.0).await.expect("metadata reads");

        assert_eq!(encoding, Encoding::Uncompressed);
        assert_eq!(src_type, MbtType::Flat);
        assert_eq!(metadata.tilejson.name, Some("tiny".to_string()));
        assert_eq!(metadata.tilejson.minzoom, Some(0));
        assert_eq!(metadata.tilejson.maxzoom, Some(1));
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn counts_a_flat_with_hash_archive_through_its_own_table() {
        let source = TempArchive::new("mbtiles");
        write_source(
            &source.0,
            MbtType::FlatWithHash,
            &[(0, 0, 0, POINT_MVT), (1, 1, 1, POINT_MVT)],
            &[("format", "pbf")],
        )
        .await;

        let (_, src_type, _, total) = get_metadata(&source.0).await.expect("metadata reads");

        assert_eq!(src_type, MbtType::FlatWithHash);
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn rejects_an_archive_without_any_tiles() {
        let source = TempArchive::new("mbtiles");
        write_source(&source.0, MbtType::Flat, &[], &[("name", "empty")]).await;

        let err = get_metadata(&source.0)
            .await
            .expect_err("an archive without tiles is rejected");

        assert_eq!(
            err.to_string(),
            format!("{} appears to be empty", source.0.display())
        );
    }

    #[tokio::test]
    async fn rejects_an_archive_whose_tiles_are_not_mvt() {
        let source = TempArchive::new("mbtiles");
        write_source(
            &source.0,
            MbtType::Flat,
            &[(0, 0, 0, PNG_MAGIC)],
            &[("format", "png")],
        )
        .await;

        let err = get_metadata(&source.0)
            .await
            .expect_err("a raster archive is rejected");

        assert_eq!(
            err.to_string(),
            format!("Expected MVT tiles, got png in {}", source.0.display())
        );
    }

    #[tokio::test]
    async fn rejects_a_file_tree_output() {
        let err = convert(
            Path::new("input.mbtiles"),
            (Path::new("tiles"), ContainerFormat::Files),
            EncoderConfig::default(),
            None,
            Compression::None,
            None,
        )
        .await
        .expect_err("a directory output is rejected");

        assert_eq!(
            err.to_string(),
            "Output must be either an .mbtiles or a .pmtiles file when input is an .mbtiles file, got: tiles"
        );
    }

    #[tokio::test]
    async fn converts_an_archive_into_an_mlt_mbtiles_keeping_the_source_schema() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("mbtiles");
        write_two_tile_source(&source.0, &[("name", "tiny"), ("format", "pbf")]).await;

        convert(
            &source.0,
            (&output.0, ContainerFormat::Mbtiles),
            EncoderConfig::default(),
            None,
            Compression::None,
            None,
        )
        .await
        .expect("conversion succeeds");

        let mbt = Mbtiles::new(&output.0).expect("output opens");
        let mut conn = mbt.open_readonly().await.expect("output connects");
        assert_eq!(
            mbt.detect_type(&mut conn)
                .await
                .expect("type is detectable"),
            MbtType::Flat
        );
        drop(conn);

        assert_eq!(
            output_metadata_value(&output.0, "format").await,
            Some("mlt".to_string())
        );
        assert_eq!(
            output_metadata_value(&output.0, "name").await,
            Some("tiny".to_string())
        );

        let tiles = output_tiles(&output.0).await;
        assert_eq!(
            tiles
                .iter()
                .map(|&(zoom, column, row, _)| (zoom, column, row))
                .collect::<Vec<_>>(),
            [(0, 0, 0), (1, 1, 1)]
        );
        for (_, _, _, data) in &tiles {
            assert_eq!(
                Parser::default()
                    .parse_layers(data)
                    .expect("converted tile parses as MLT")
                    .len(),
                1
            );
        }
    }

    #[tokio::test]
    async fn converts_an_archive_into_the_requested_schema() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("mbtiles");
        write_two_tile_source(&source.0, &[("format", "pbf")]).await;

        convert(
            &source.0,
            (&output.0, ContainerFormat::Mbtiles),
            EncoderConfig::default(),
            Some(MbtType::FlatWithHash),
            Compression::None,
            None,
        )
        .await
        .expect("conversion succeeds");

        let mbt = Mbtiles::new(&output.0).expect("output opens");
        let mut conn = mbt.open_readonly().await.expect("output connects");
        assert_eq!(
            mbt.detect_type(&mut conn)
                .await
                .expect("type is detectable"),
            MbtType::FlatWithHash
        );
    }

    #[tokio::test]
    async fn clipping_narrows_the_bounds_and_center_of_the_converted_mbtiles() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("mbtiles");
        write_two_tile_source(
            &source.0,
            &[
                ("format", "pbf"),
                ("bounds", "-180,-85,180,85"),
                ("center", "0,0,3"),
            ],
        )
        .await;

        convert(
            &source.0,
            (&output.0, ContainerFormat::Mbtiles),
            EncoderConfig::default(),
            None,
            Compression::None,
            Some(Bounds::new(20.0, 20.0, 30.0, 30.0)),
        )
        .await
        .expect("conversion succeeds");

        assert_eq!(
            output_metadata_value(&output.0, "bounds").await,
            Some("20,20,30,30".to_string())
        );
        assert_eq!(
            output_metadata_value(&output.0, "center").await,
            Some("20,20,3".to_string())
        );
    }

    #[tokio::test]
    async fn converts_an_archive_into_a_gzip_compressed_mlt_pmtiles() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("pmtiles");
        write_two_tile_source(&source.0, &[("name", "tiny"), ("format", "pbf")]).await;

        convert(
            &source.0,
            (&output.0, ContainerFormat::Pmtiles),
            EncoderConfig::default(),
            None,
            Compression::Gzip,
            None,
        )
        .await
        .expect("conversion succeeds");

        let reader = open_pmtiles(&output.0).await;
        let header = reader.get_header();
        assert_eq!(header.tile_type, TileType::Mlt);
        assert_eq!(header.tile_compression, Compression::Gzip);
        assert_eq!((header.min_zoom, header.max_zoom), (0, 1));

        let metadata: serde_json::Value =
            serde_json::from_str(&reader.get_metadata().await.expect("metadata reads"))
                .expect("metadata is JSON");
        assert_eq!(metadata["format"], "mlt");
        assert_eq!(metadata["compression"], "gzip");
        assert_eq!(metadata["name"], "tiny");

        for coord in [(0, 0, 0), (1, 1, 0)] {
            let coord = TileCoord::new(coord.0, coord.1, coord.2).expect("coordinate is valid");
            let raw = reader
                .get_tile(coord)
                .await
                .expect("tile reads")
                .expect("tile exists");
            assert_eq!(&raw[..2], &[0x1f, 0x8b]);

            let tile = reader
                .get_tile_decompressed(coord)
                .await
                .expect("tile decompresses")
                .expect("tile exists");
            assert_eq!(
                Parser::default()
                    .parse_layers(&tile)
                    .expect("converted tile parses as MLT")
                    .len(),
                1
            );
        }
    }

    #[tokio::test]
    async fn clipping_narrows_the_geography_of_the_converted_pmtiles() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("pmtiles");
        let bbox = Bounds::new(20.0, 20.0, 30.0, 30.0);
        write_two_tile_source(
            &source.0,
            &[
                ("format", "pbf"),
                ("bounds", "-180,-85,180,85"),
                ("center", "0,0,3"),
            ],
        )
        .await;

        convert(
            &source.0,
            (&output.0, ContainerFormat::Pmtiles),
            EncoderConfig::default(),
            None,
            Compression::None,
            Some(bbox),
        )
        .await
        .expect("conversion succeeds");

        let reader = open_pmtiles(&output.0).await;
        let header = reader.get_header();
        assert_eq!(
            Bounds::new(
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude
            ),
            bbox
        );
        assert_eq!(
            Center::new(
                header.center_longitude,
                header.center_latitude,
                header.center_zoom
            ),
            Center::new(20.0, 20.0, 3)
        );

        let metadata: serde_json::Value =
            serde_json::from_str(&reader.get_metadata().await.expect("metadata reads"))
                .expect("metadata is JSON");
        assert_eq!(metadata["format"], "mlt");
        assert_eq!(metadata["compression"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn extracts_only_the_tiles_overlapping_the_bbox() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt.max1.mbtiles");
        let output = std::env::temp_dir().join(format!(
            "mlt-bbox-extract-test-{}.pmtiles",
            std::process::id()
        ));
        let filter = BboxFilter::new(&[Bounds::new(20.0, 20.0, 30.0, 30.0)])
            .expect("bbox is valid")
            .expect("bbox is set");

        let extract = BboxExtract::create(&input, &output, &filter)
            .await
            .expect("extract is written");
        assert_eq!(extract.source_type, MbtType::Flat);

        let mbt = Mbtiles::new(extract.path()).expect("extract opens");
        let mut conn = mbt.open_readonly().await.expect("extract connects");
        let tiles: Vec<(i64, i64, i64)> =
            sqlx::query_as("SELECT zoom_level, tile_column, tile_row FROM tiles ORDER BY 1, 2, 3")
                .fetch_all(&mut conn)
                .await
                .expect("extract is queryable");
        assert_eq!(tiles, [(0, 0, 0), (1, 1, 1)]);
        assert_eq!(
            mbt.get_metadata_value(&mut conn, "format")
                .await
                .expect("metadata is readable"),
            Some("pbf".to_string())
        );
        drop(conn);

        let path = extract.path().to_path_buf();
        drop(extract);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn rejects_a_bbox_that_selects_no_tiles() {
        let source = TempArchive::new("mbtiles");
        let output = TempArchive::new("pmtiles");
        write_source(
            &source.0,
            MbtType::Flat,
            &[(1, 0, 0, POINT_MVT)],
            &[("format", "pbf")],
        )
        .await;
        let filter = BboxFilter::new(&[Bounds::new(20.0, 20.0, 30.0, 30.0)])
            .expect("bbox is valid")
            .expect("bbox is set");

        let Err(err) = BboxExtract::create(&source.0, &output.0, &filter).await else {
            panic!("an empty selection is rejected");
        };

        assert_eq!(
            err.to_string(),
            format!(
                "--bbox 20,20,30,30 selected no tiles from {}",
                source.0.display()
            )
        );
    }

    #[test]
    fn reads_pmtiles_geography_from_mbtiles_metadata() {
        let metadata = Metadata {
            id: "test".into(),
            layer_type: None,
            tilejson: serde_json::from_value(serde_json::json!({
                "tilejson": "3.0.0",
                "tiles": [],
                "minzoom": 3,
                "maxzoom": 12,
                "bounds": [-12.345_678_9, -67.890_123_4, 98.765_432_1, 54.321_098_7],
                "center": [11.223_344_5, -44.556_677_8, 8]
            }))
            .expect("parse TileJSON metadata"),
            json: None,
            agg_tiles_hash: None,
        };

        assert_eq!(
            geography_from_metadata(&metadata),
            PmTilesGeography {
                min_zoom: Some(3),
                max_zoom: Some(12),
                bounds: Some(Bounds::new(
                    -12.345_678_9,
                    -67.890_123_4,
                    98.765_432_1,
                    54.321_098_7
                )),
                center: Some(Center::new(11.223_344_5, -44.556_677_8, 8)),
            }
        );
    }

    #[test]
    fn clipping_a_tilejson_narrows_its_bounds_and_pulls_the_center_in() {
        let mut tilejson: TileJSON = serde_json::from_value(serde_json::json!({
            "tilejson": "3.0.0",
            "tiles": [],
            "bounds": [-180.0, -85.0, 180.0, 85.0],
            "center": [0.0, 0.0, 3]
        }))
        .expect("parse TileJSON metadata");

        clip_tilejson(&mut tilejson, Bounds::new(20.0, 20.0, 30.0, 30.0));

        assert_eq!(tilejson.bounds, Some(Bounds::new(20.0, 20.0, 30.0, 30.0)));
        assert_eq!(tilejson.center, Some(Center::new(20.0, 20.0, 3)));
    }
}
