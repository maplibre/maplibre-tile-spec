mod bbox;
mod common;
pub mod fields;
mod from_files;
mod from_geojson;
mod from_mbtiles;
mod from_pmtiles;
pub mod verify;

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result as AnyResult, bail};
use bytes::Bytes;
use clap::{Args, ValueEnum};
use indicatif::ProgressState;
use martin_tile_utils::{Encoding, Format, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{MbtType, NormalizedSchema};
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig, WireVersion};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{Decoder, Layer, Parser};
use pmtiles::Compression;
use tilejson::Bounds;

use crate::convert::bbox::BboxFilter;
use crate::convert::fields::FieldConfig;
use crate::convert::from_mbtiles::BboxExtract;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "state.per_sec() is always non-negative and well below 2^63 tiles/sec"
)]
fn whole_rate_per_sec(state: &ProgressState, w: &mut dyn std::fmt::Write) {
    let _ = w.write_fmt(format_args!("{}/s", state.per_sec() as u64));
}

/// Storage container shape inferred from a path's extension.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum ContainerFormat {
    Mbtiles,
    Pmtiles,
    Files,
}

impl ContainerFormat {
    #[must_use]
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("mbtiles") => Self::Mbtiles,
            Some("pmtiles") => Self::Pmtiles,
            _ => Self::Files,
        }
    }
}

/// CLI-facing subset of [`MbtType`] (hides the `hash_view` detail).
#[derive(Clone, Copy, Default, ValueEnum, Debug, PartialEq)]
enum MbtFormat {
    /// Single table with all tiles; no deduplication (smallest overhead)
    #[default]
    Flat,
    /// Single table with tiles and `MD5` hashes
    #[value(name = "flat-with-hash")]
    FlatWithHash,
    /// Separate `images` / `map` tables; identical tiles stored only once
    Normalized,
}

impl From<MbtFormat> for MbtType {
    fn from(f: MbtFormat) -> Self {
        match f {
            MbtFormat::Flat => Self::Flat,
            MbtFormat::FlatWithHash => Self::FlatWithHash,
            MbtFormat::Normalized => Self::Normalized {
                hash_view: true,
                schema: NormalizedSchema::DedupId,
            },
        }
    }
}

#[derive(Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum TileFormat {
    /// `MapLibre Tile` format (default)
    #[default]
    Mlt,
    /// `Mapbox Vector Tile` format
    Mvt,
}

impl TileFormat {
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mlt => "mlt",
            Self::Mvt => "mvt",
        }
    }

    /// Detect format from a path's extension; defaults to MLT for unknown.
    #[must_use]
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("mvt" | "pbf") => Self::Mvt,
            _ => Self::Mlt,
        }
    }
}

/// Which MLT wire format version `convert` writes.
#[derive(Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum MltVersion {
    /// Version 1
    #[default]
    #[value(name = "1", alias = "v1")]
    V1,
    /// Version 2 (experimental)
    #[value(name = "2", alias = "v2")]
    V2,
}

impl From<MltVersion> for WireVersion {
    fn from(version: MltVersion) -> Self {
        match version {
            MltVersion::V1 => Self::V01,
            MltVersion::V2 => Self::V02,
        }
    }
}

#[derive(Clone, Default, ValueEnum)]
enum SortMode {
    /// Try no-sort and Morton sort, keep the smaller (default).
    ///
    /// Morton wins ~8% of layers and captures nearly all the spatial gain.
    /// Hilbert and feature-ID sort each win <1% of layers, at double the encode work.
    #[default]
    Auto,
    /// Try every sort strategy (no-sort, Morton, Hilbert, feature-ID) and keep the smallest.
    /// Slowest, for marginally smaller output.
    All,
    /// Do not reorder features (original order only)
    None,
    /// Only try Z-order (Morton) curve sort
    Morton,
    /// Only try Hilbert curve sort
    Hilbert,
    /// Only try feature-ID ascending sort
    Id,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, ValueEnum)]
pub(super) enum TileCompression {
    /// Store MLT tile payloads without outer compression
    #[default]
    None,
    /// Gzip-compress each MLT tile payload for size, sacrificing decoding speed
    Gzip,
}

impl From<TileCompression> for Compression {
    fn from(comp: TileCompression) -> Self {
        match comp {
            TileCompression::None => Self::None,
            TileCompression::Gzip => Self::Gzip,
        }
    }
}

fn update_mlt_pmtiles_metadata(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    tile_compression: Compression,
) {
    metadata.insert(
        "format".into(),
        serde_json::Value::String(Format::Mlt.metadata_format_value().into()),
    );
    match tile_compression.content_encoding() {
        Some(compression) => {
            metadata.insert(
                "compression".into(),
                serde_json::Value::String(compression.into()),
            );
        }
        None => {
            metadata.remove("compression");
        }
    }
}

/// The encoder settings, field parsing and verification every layer is re-encoded with.
#[derive(Clone, Default)]
pub(crate) struct Reencoder {
    encoder: EncoderConfig,
    fields: Arc<FieldConfig>,
    /// Decode every encoded layer and require it to match its input.
    verify: bool,
}

impl Reencoder {
    #[hotpath::measure]
    fn encode(&self, layer: mlt_core::TileLayer) -> AnyResult<Vec<u8>> {
        let source = self.verify.then(|| layer.clone());
        let layer = self.fields.apply(layer)?;
        let encoded = layer.encode(self.encoder)?;
        if let Some(source) = source {
            verify::check_round_trip(&source, &encoded, |decoded| self.fields.restore(decoded))?;
        }
        Ok(encoded)
    }
}

#[derive(Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI on/off flag, not a state machine"
)]
pub struct ConvertArgs {
    /// Input: a directory with .mlt/.mvt/.pbf tiles, a single tile file, an .mbtiles or .pmtiles archive,
    /// or a WGS84 .geojson file holding a `FeatureCollection` to tile
    input: PathBuf,
    /// Output: a directory for re-encoded .mlt files, an .mbtiles database or a .pmtiles file
    output: PathBuf,
    /// Add tessellation
    #[clap(short, long)]
    tessellate: bool,
    /// Sort strategy to try when re-encoding (encoder keeps the smallest result)
    #[clap(long, default_value = "auto")]
    sort: SortMode,
    /// Schema type for the output `.mbtiles` file; defaults to the input file's schema
    #[clap(long)]
    mbtiles_format: Option<MbtFormat>,
    /// Disable grouping of similar string columns into shared dictionaries
    #[clap(long)]
    no_shared_dict: bool,
    /// Disable `FastPFOR` integer compression (only `VarInt` physical encodings compete)
    #[clap(long)]
    no_fastpfor: bool,
    /// Disable `FSST` string compression
    #[clap(long)]
    no_fsst: bool,
    /// Decode every encoded layer and fail unless it matches the input, parsed fields formatted back
    #[clap(long)]
    verify: bool,
    /// TOML file naming the string properties to parse into typed columns, per layer
    ///
    /// Each `[layers.<name>]` table maps a property to `{ split, kind, running-sum, into }`.
    /// `split` is `"sign"` or one character, and `kind` is one of `i32`, `u32`, `i64`, `u64` or `str`.
    /// `running-sum` stores each value after the first as its difference from the one before.
    /// `into` is `list` (the default) or `m-value`, which needs one value per vertex.
    /// A string that would not format back to itself fails the conversion.
    #[clap(long, value_name = "FILE")]
    fields: Option<PathBuf>,
    /// Output tile format (`mlt` re-encodes; `mvt` decodes MLT inputs back to MVT)
    #[clap(long, default_value = "mlt")]
    to: TileFormat,
    /// MLT wire format version to write
    #[clap(long, default_value = "1")]
    mlt_version: MltVersion,
    /// Outer compression for tile payloads
    #[clap(long, value_enum, default_value = "none")]
    tile_compression: TileCompression,
    /// Bounds to convert, in the format `min_lon,min_lat,max_lon,max_lat`
    ///
    /// Every tile overlapping the bounds is converted, at every zoom level.
    /// Can be specified multiple times.
    /// If omitted, the whole input is converted.
    #[clap(long, value_name = "BBOX", allow_hyphen_values = true)]
    bbox: Vec<Bounds>,
    /// Lowest zoom level to tile a `GeoJSON` input into (default: 0)
    #[clap(long, value_name = "ZOOM", help_heading = "GeoJSON input")]
    min_zoom: Option<u8>,
    /// Highest zoom level to tile a `GeoJSON` input into (required)
    #[clap(long, value_name = "ZOOM", help_heading = "GeoJSON input")]
    max_zoom: Option<u8>,
    /// MLT layer name of a `GeoJSON` input (default: the input file stem)
    #[clap(long, value_name = "NAME", help_heading = "GeoJSON input")]
    layer: Option<String>,
    /// Power of ten of the z grid's step in metres (-3..=4) for `GeoJSON` positions with an altitude
    ///
    /// Required when the positions carry an altitude, and rejected when they do not.
    #[clap(
        long,
        value_name = "EXPONENT",
        allow_hyphen_values = true,
        help_heading = "GeoJSON input"
    )]
    z_step: Option<i8>,
    #[command(flatten)]
    v2: V2EncoderArgs,
}

/// Encoder options that only MLT version 2 can express, which the encoder ignores under v1.
#[derive(Args, Default, PartialEq, Eq)]
#[command(next_help_heading = "MLT version 2 options (need `--mlt-version 2`)")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI on/off flag, not a state machine"
)]
struct V2EncoderArgs {
    /// Disable Framed, Exception-Free ALP float encoding, storing floats as decimal-scaled integers
    #[clap(long)]
    no_alp: bool,
    /// Disable float dictionary encoding
    #[clap(long)]
    no_float_dict: bool,
    /// Disable bit-packed dictionary codes, leaving a varint each
    #[clap(long)]
    no_bitpacking: bool,
    /// With `--tessellate`, store only the triangles of an all-polygon layer, without its outlines
    #[clap(long, requires = "tessellate")]
    triangles_only: bool,
    /// Let integer and vertex streams store the deltas of their deltas when that is shorter
    #[clap(long)]
    delta2: bool,
    /// Store vertex streams rANS-coded when that beats componentwise delta
    #[clap(long)]
    rans_vertices: bool,
}

/// Whether `path` names a `GeoJSON` file, which `convert` tiles rather than re-encodes.
fn is_geojson(path: &Path) -> bool {
    path.extension().and_then(std::ffi::OsStr::to_str) == Some("geojson")
}

impl ConvertArgs {
    #[must_use]
    pub fn input_container(&self) -> ContainerFormat {
        ContainerFormat::from_path(&self.input)
    }
    #[must_use]
    pub fn output_container(&self) -> ContainerFormat {
        ContainerFormat::from_path(&self.output)
    }
}

pub fn convert(args: &ConvertArgs) -> AnyResult<()> {
    // The encoder ignores these under v1, which would silently write a tile without them.
    if args.mlt_version != MltVersion::V2 && args.v2 != V2EncoderArgs::default() {
        bail!(
            "the MLT version 2 options in --help only apply to MLT version 2, so they need --mlt-version 2"
        );
    }
    let v2 = &args.v2;
    if args.verify && v2.triangles_only {
        bail!("--verify compares polygon outlines, which --triangles-only drops");
    }
    if args.to == TileFormat::Mvt {
        if args.verify {
            bail!("--verify checks MLT encoding, so it needs --to mlt");
        }
        if args.fields.is_some() {
            bail!("--fields parses fields into MLT columns, so it needs --to mlt");
        }
    }
    let morton = matches!(args.sort, SortMode::All | SortMode::Auto | SortMode::Morton);
    let hilbert = matches!(args.sort, SortMode::All | SortMode::Hilbert);
    let id_sort = matches!(args.sort, SortMode::All | SortMode::Id);
    let encoder = EncoderConfig::default()
        .with_tessellation(args.tessellate)
        .with_spatial_morton_sort(morton)
        .with_spatial_hilbert_sort(hilbert)
        .with_id_sort(id_sort)
        .with_shared_dict(!args.no_shared_dict)
        .with_fastpfor(!args.no_fastpfor)
        .with_fsst(!args.no_fsst)
        .with_wire_version(args.mlt_version.into())
        .with_float_alp(!v2.no_alp)
        .with_float_dict(!v2.no_float_dict)
        .with_packed_dict_codes(!v2.no_bitpacking)
        .with_triangles_only(v2.triangles_only)
        .with_delta2(v2.delta2)
        .with_rans_vertices(v2.rans_vertices);
    let reencoder = Reencoder {
        encoder,
        fields: Arc::new(match &args.fields {
            Some(path) => FieldConfig::load(path)?,
            None => FieldConfig::default(),
        }),
        verify: args.verify,
    };
    if !reencoder.fields.is_empty() && args.mlt_version != MltVersion::V2 {
        bail!(
            "--fields parses fields into m-values and nested columns, which need --mlt-version 2"
        );
    }

    let input_container = args.input_container();
    let output_container = args.output_container();
    let geojson_input = is_geojson(&args.input);
    let geojson_only = [
        ("--min-zoom", args.min_zoom.is_some()),
        ("--max-zoom", args.max_zoom.is_some()),
        ("--layer", args.layer.is_some()),
        ("--z-step", args.z_step.is_some()),
    ];
    if geojson_input {
        if args.max_zoom.is_none() {
            bail!("tiling a GeoJSON input requires --max-zoom");
        }
        if args.to == TileFormat::Mvt {
            bail!("--to mvt is not supported for a GeoJSON input; it is tiled into MLT");
        }
        if args.mlt_version != MltVersion::V2 {
            bail!("tiling a GeoJSON input writes MLT version 2, so it needs --mlt-version 2");
        }
        if output_container != ContainerFormat::Files {
            bail!(
                "a GeoJSON input is tiled into a directory of z/x/y.mlt tiles; archive output is not supported yet, got: {}",
                args.output.display()
            );
        }
        if args.output.exists() {
            bail!(
                "Output {} already exists; refusing to append. \
                 Delete it first or choose a different path.",
                args.output.display()
            );
        }
    } else if let Some((flag, _)) = geojson_only.iter().find(|(_, given)| *given) {
        bail!(
            "{flag} tiles a GeoJSON input, but the input is not a .geojson file: {}",
            args.input.display()
        );
    }
    let filter = BboxFilter::new(&args.bbox)?;
    let has_archive_input =
        input_container == ContainerFormat::Mbtiles || input_container == ContainerFormat::Pmtiles;
    if filter.is_some() && !has_archive_input {
        bail!(
            "--bbox currently requires an archive based input (mbtiles,pmtiles), but got {}",
            args.input.display()
        );
    }
    if args.tile_compression != TileCompression::None
        && (!has_archive_input || output_container != ContainerFormat::Pmtiles)
    {
        bail!(
            "--tile-compression is currently only supported when converting .mbtiles or .pmtiles input to .pmtiles output"
        );
    }
    let converted = if has_archive_input {
        if args.to == TileFormat::Mvt {
            bail!(
                "--to mvt is not supported for .mbtiles/.pmtiles input/output yet; convert to a directory instead"
            );
        }
        if output_container == ContainerFormat::Files {
            bail!(
                "Output must be either an .mbtiles or a .pmtiles file when input is an .mbtiles/.pmtiles file, got: {}",
                args.output.display()
            );
        }
        if args.output.exists() {
            bail!(
                "Output {} already exists; refusing to append. \
                 Delete it first or choose a different path.",
                args.output.display()
            );
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?;
        let output = (args.output.as_path(), output_container);
        match input_container {
            ContainerFormat::Pmtiles => runtime.block_on(from_pmtiles::convert(
                &args.input,
                output,
                &reencoder,
                args.tile_compression.into(),
                filter.as_ref(),
            )),
            ContainerFormat::Mbtiles => {
                runtime.block_on(convert_mbtiles(args, output, &reencoder, filter.as_ref()))
            }
            ContainerFormat::Files => {
                unreachable!("`has_archive_input` above rules out a directory input")
            }
        }
    } else if geojson_input {
        from_geojson::convert(args, &reencoder)
    } else {
        from_files::convert(&args.input, &args.output, &reencoder, args.to)
    };
    if converted.is_ok() {
        for name in reencoder.fields.unused() {
            eprintln!("warning: --fields names {name}, which no converted tile holds");
        }
    }
    converted
}

/// Converts an `.mbtiles` input, first extracting the requested boxes into a temporary
/// archive so that only those tiles are read and re-encoded.
async fn convert_mbtiles(
    args: &ConvertArgs,
    output: (&Path, ContainerFormat),
    reencoder: &Reencoder,
    filter: Option<&BboxFilter>,
) -> AnyResult<()> {
    let dst_type = args.mbtiles_format.map(MbtType::from);
    let tile_compression = args.tile_compression.into();
    let Some(filter) = filter else {
        return from_mbtiles::convert(
            &args.input,
            output,
            reencoder,
            dst_type,
            tile_compression,
            None,
        )
        .await;
    };

    let extract = BboxExtract::create(&args.input, output.0, filter).await?;
    from_mbtiles::convert(
        extract.path(),
        output,
        reencoder,
        dst_type.or(Some(extract.source_type)),
        tile_compression,
        Some(filter.bounds()),
    )
    .await
}

fn convert_mlt_buffer(buffer: &[u8], reencoder: &Reencoder) -> AnyResult<Vec<u8>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut out: Vec<u8> = Vec::new();

    for layer in layers {
        if let Layer::Unknown(u) = layer {
            out.extend(
                EncodedUnknown::from(u)
                    .write_to(Encoder::default())?
                    .into_raw_bytes(),
            );
            continue;
        }
        let tile = layer
            .into_tile(&mut dec)?
            .expect("unknown layers are handled above");
        out.extend_from_slice(&reencoder.encode(tile)?);
    }

    Ok(out)
}

#[hotpath::measure]
fn convert_mvt_buffer(buffer: &[u8], reencoder: &Reencoder) -> AnyResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for tile in mvt_to_tile_layers(buffer)? {
        out.extend_from_slice(&reencoder.encode(tile)?);
    }
    Ok(out)
}

/// Decode an MLT buffer to row-oriented [`mlt_core::TileLayer`]s.
///
/// MVT has no equivalent for unknown/extension MLT layer tags, so conversion
/// is rejected instead of silently dropping data.
fn mlt_buffer_to_tile_layers(buffer: &[u8]) -> AnyResult<Vec<mlt_core::TileLayer>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut tiles = Vec::new();
    for layer in layers {
        let Some(tile) = layer.into_tile(&mut dec)? else {
            bail!(
                "cannot convert MLT tile to MVT: tile contains unknown/extension layers that MVT cannot represent"
            );
        };
        tiles.push(tile);
    }
    Ok(tiles)
}

#[hotpath::measure]
fn encode_one(data: &[u8], encoding: Encoding, reencoder: &Reencoder) -> AnyResult<(Bytes, u64)> {
    let mvt = match encoding {
        Encoding::Gzip => Cow::Owned(decode_gzip(data)?),
        Encoding::Zlib => Cow::Owned(decode_zlib(data)?),
        Encoding::Brotli => Cow::Owned(decode_brotli(data)?),
        Encoding::Zstd => Cow::Owned(decode_zstd(data)?),
        Encoding::Uncompressed | Encoding::Internal => Cow::Borrowed(data),
    };
    let raw_mvt_size = mvt.len() as u64;
    convert_mvt_buffer(&mvt, reencoder).map(|data| (Bytes::from_owner(data), raw_mvt_size))
}

/// Convert one input buffer to the requested target format.
fn convert_buffer(
    buffer: Vec<u8>,
    from: TileFormat,
    to: TileFormat,
    reencoder: &Reencoder,
) -> AnyResult<Vec<u8>> {
    match (from, to) {
        (TileFormat::Mlt, TileFormat::Mlt) => convert_mlt_buffer(&buffer, reencoder),
        (TileFormat::Mvt, TileFormat::Mlt) => convert_mvt_buffer(&buffer, reencoder),
        (TileFormat::Mlt, TileFormat::Mvt) => {
            Ok(tile_layers_to_mvt(mlt_buffer_to_tile_layers(&buffer)?)?)
        }
        // Re-encoding through TileLayer is lossy (e.g. SInt vs Int wire choice)
        // and offers no benefit.
        (TileFormat::Mvt, TileFormat::Mvt) => Ok(buffer),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use futures::TryStreamExt as _;
    use pmtiles::{AsyncPmTilesReader, HashMapCache, MmapBackend, TileCoord, TileType};

    use super::*;

    #[derive(clap::Parser)]
    struct ConvertCli {
        #[command(flatten)]
        args: ConvertArgs,
    }

    fn parse_args(argv: &[&str]) -> ConvertArgs {
        <ConvertCli as clap::Parser>::parse_from(
            std::iter::once("mlt-convert").chain(argv.iter().copied()),
        )
        .args
    }

    fn path_arg(path: &Path) -> &str {
        path.to_str().expect("test paths are UTF-8")
    }

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    /// A temp path deleted on drop, whether it became a file or a directory.
    struct TempPath(PathBuf);

    impl TempPath {
        fn new(suffix: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!(
                "mlt-convert-mod-test-{}-{id}{suffix}",
                std::process::id()
            )))
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    type TestReader = AsyncPmTilesReader<MmapBackend, HashMapCache>;

    fn read_archive(path: &Path) -> (TileType, Bounds, Vec<(u8, u32, u32)>) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime builds");
        runtime.block_on(async {
            let reader = Arc::new(
                TestReader::new_with_cached_path(HashMapCache::default(), path)
                    .await
                    .expect("output archive opens"),
            );
            let header = reader.get_header();
            let tile_type = header.tile_type;
            let bounds = Bounds::new(
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
            );
            let mut coords = Vec::new();
            let mut entries = reader.entries();
            while let Some(entry) = entries.try_next().await.expect("tile directory reads") {
                coords.extend(entry.iter_coords().map(|id| {
                    let coord = TileCoord::from(id);
                    (coord.z(), coord.x(), coord.y())
                }));
            }
            (tile_type, bounds, coords)
        })
    }

    const OMT_TILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/omt/0_0_0.mvt"
    );

    fn round_trip(mvt: Vec<u8>, version: WireVersion, to: TileFormat) -> Vec<u8> {
        let reencoder = Reencoder {
            encoder: EncoderConfig::default().with_wire_version(version),
            ..Reencoder::default()
        };
        let mlt = convert_buffer(mvt, TileFormat::Mvt, TileFormat::Mlt, &reencoder).unwrap();
        convert_buffer(mlt, TileFormat::Mlt, to, &reencoder).unwrap()
    }

    /// Feature order is a per-version encoder choice, so only the layers themselves compare.
    fn layer_shape(mvt: Vec<u8>) -> Vec<(String, usize)> {
        mvt_to_tile_layers(mvt)
            .unwrap()
            .iter()
            .map(|layer| (layer.name().to_owned(), layer.features().len()))
            .collect()
    }

    #[test]
    fn converting_a_v2_tile_back_to_mvt_keeps_every_layer() {
        let mvt = fs::read(OMT_TILE).unwrap();
        let back = round_trip(mvt.clone(), WireVersion::V02, TileFormat::Mvt);
        assert_eq!(layer_shape(back), layer_shape(mvt));
    }

    #[test]
    fn re_encoding_a_v2_tile_keeps_every_layer() {
        let mvt = fs::read(OMT_TILE).unwrap();
        let expected = mvt_to_tile_layers(mvt.clone()).unwrap().len();
        let mlt = round_trip(mvt, WireVersion::V02, TileFormat::Mlt);
        let layers = Parser::default().parse_layers(&mlt).unwrap();
        assert_eq!(layers.len(), expected);
    }

    #[test]
    fn pmtiles_metadata_tracks_tile_compression() {
        let mut metadata = serde_json::Map::from_iter([
            ("format".into(), serde_json::Value::String("pbf".into())),
            (
                "compression".into(),
                serde_json::Value::String("gzip".into()),
            ),
        ]);

        update_mlt_pmtiles_metadata(&mut metadata, Compression::None);
        assert_eq!(metadata["format"], "mlt");
        assert!(!metadata.contains_key("compression"));

        update_mlt_pmtiles_metadata(&mut metadata, Compression::Gzip);
        assert_eq!(metadata["format"], "mlt");
        assert_eq!(metadata["compression"], "gzip");
    }

    #[test]
    fn container_format_comes_from_the_path_extension() {
        assert_eq!(
            ContainerFormat::from_path(Path::new("tiles.mbtiles")),
            ContainerFormat::Mbtiles
        );
        assert_eq!(
            ContainerFormat::from_path(Path::new("tiles.pmtiles")),
            ContainerFormat::Pmtiles
        );
        assert_eq!(
            ContainerFormat::from_path(Path::new("tiles.mvt")),
            ContainerFormat::Files
        );
        assert_eq!(
            ContainerFormat::from_path(Path::new("tiles")),
            ContainerFormat::Files
        );
    }

    #[test]
    fn mbtiles_format_maps_to_the_mbtiles_schema_type() {
        assert_eq!(MbtType::from(MbtFormat::Flat), MbtType::Flat);
        assert_eq!(
            MbtType::from(MbtFormat::FlatWithHash),
            MbtType::FlatWithHash
        );
        assert_eq!(
            MbtType::from(MbtFormat::Normalized),
            MbtType::Normalized {
                hash_view: true,
                schema: NormalizedSchema::DedupId,
            }
        );
    }

    #[test]
    fn tile_format_extension_matches_the_format() {
        assert_eq!(TileFormat::Mlt.extension(), "mlt");
        assert_eq!(TileFormat::Mvt.extension(), "mvt");
    }

    #[test]
    fn tile_format_comes_from_the_path_extension() {
        assert!(TileFormat::from_path(Path::new("tile.mvt")) == TileFormat::Mvt);
        assert!(TileFormat::from_path(Path::new("tile.pbf")) == TileFormat::Mvt);
        assert!(TileFormat::from_path(Path::new("tile.mlt")) == TileFormat::Mlt);
        assert!(TileFormat::from_path(Path::new("tile")) == TileFormat::Mlt);
    }

    #[test]
    fn convert_writes_mlt_v1_unless_told_otherwise() {
        assert!(parse_args(&["src", "dst"]).mlt_version == MltVersion::V1);
        assert!(parse_args(&["--mlt-version", "2", "src", "dst"]).mlt_version == MltVersion::V2);
    }

    #[test]
    fn mlt_version_maps_to_the_wire_version() {
        assert_eq!(WireVersion::from(MltVersion::V1), WireVersion::V01);
        assert_eq!(WireVersion::from(MltVersion::V2), WireVersion::V02);
    }

    #[test]
    fn containers_come_from_the_input_and_output_paths() {
        let args = parse_args(&["src.mbtiles", "dst.pmtiles"]);
        assert_eq!(args.input_container(), ContainerFormat::Mbtiles);
        assert_eq!(args.output_container(), ContainerFormat::Pmtiles);
    }

    #[test]
    fn a_bbox_without_an_archive_input_is_rejected() {
        let err = convert(&parse_args(&["--bbox", "1,1,2,2", "tiles", "out"])).unwrap_err();
        insta::assert_snapshot!(
            err.to_string(),
            @"--bbox currently requires an archive based input (mbtiles,pmtiles), but got tiles"
        );
    }

    #[test]
    fn tile_compression_without_an_archive_input_is_rejected() {
        let err = convert(&parse_args(&[
            "--tile-compression",
            "gzip",
            "tiles",
            "out.pmtiles",
        ]))
        .unwrap_err();
        insta::assert_snapshot!(
            err.to_string(),
            @"--tile-compression is currently only supported when converting .mbtiles or .pmtiles input to .pmtiles output"
        );
    }

    #[test]
    fn tile_compression_into_an_mbtiles_output_is_rejected() {
        let err = convert(&parse_args(&[
            "--tile-compression",
            "gzip",
            "src.mbtiles",
            "dst.mbtiles",
        ]))
        .unwrap_err();
        insta::assert_snapshot!(
            err.to_string(),
            @"--tile-compression is currently only supported when converting .mbtiles or .pmtiles input to .pmtiles output"
        );
    }

    #[test]
    fn verifying_a_conversion_to_mvt_is_rejected() {
        let err = convert(&parse_args(&["--verify", "--to", "mvt", "src", "dst"])).unwrap_err();
        insta::assert_snapshot!(err.to_string(), @"--verify checks MLT encoding, so it needs --to mlt");
    }

    #[test]
    fn verifying_triangles_only_is_rejected() {
        let err = convert(&parse_args(&[
            "--mlt-version",
            "2",
            "--verify",
            "--tessellate",
            "--triangles-only",
            "src",
            "dst",
        ]))
        .unwrap_err();
        insta::assert_snapshot!(err.to_string(), @"--verify compares polygon outlines, which --triangles-only drops");
    }

    #[test]
    fn a_v2_only_encoder_option_without_v2_is_rejected() {
        for flags in [
            &["--no-alp"][..],
            &["--no-float-dict"],
            &["--no-bitpacking"],
            &["--tessellate", "--triangles-only"],
            &["--delta2"],
            &["--rans-vertices"],
        ] {
            let mut argv = flags.to_vec();
            argv.extend(["src", "dst"]);
            let err = convert(&parse_args(&argv)).unwrap_err();
            assert_eq!(
                err.to_string(),
                "the MLT version 2 options in --help only apply to MLT version 2, so they need --mlt-version 2",
                "{flags:?}"
            );
        }
    }

    #[test]
    fn a_fields_file_for_a_conversion_to_mvt_is_rejected() {
        let err = convert(&parse_args(&[
            "--fields",
            "fields.toml",
            "--to",
            "mvt",
            "src",
            "dst",
        ]))
        .unwrap_err();
        insta::assert_snapshot!(err.to_string(), @"--fields parses fields into MLT columns, so it needs --to mlt");
    }

    #[test]
    fn tiling_a_geojson_input_without_v2_is_rejected() {
        let err = convert(&parse_args(&["--max-zoom", "0", "in.geojson", "out"])).unwrap_err();
        insta::assert_snapshot!(err.to_string(), @"tiling a GeoJSON input writes MLT version 2, so it needs --mlt-version 2");
    }

    #[test]
    fn converting_an_archive_to_mvt_is_rejected() {
        let err = convert(&parse_args(&["--to", "mvt", "src.mbtiles", "dst.pmtiles"])).unwrap_err();
        insta::assert_snapshot!(
            err.to_string(),
            @"--to mvt is not supported for .mbtiles/.pmtiles input/output yet; convert to a directory instead"
        );
    }

    #[test]
    fn converting_an_archive_into_a_directory_is_rejected() {
        let err = convert(&parse_args(&["src.mbtiles", "out"])).unwrap_err();
        insta::assert_snapshot!(
            err.to_string(),
            @"Output must be either an .mbtiles or a .pmtiles file when input is an .mbtiles/.pmtiles file, got: out"
        );
    }

    #[test]
    fn an_existing_archive_output_is_never_overwritten() {
        let output = TempPath::new(".pmtiles");
        fs::write(&output.0, b"not an archive").expect("placeholder is written");

        let err = convert(&parse_args(&["src.mbtiles", path_arg(&output.0)])).unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Output {} already exists; refusing to append. \
                 Delete it first or choose a different path.",
                output.0.display()
            )
        );
    }

    #[test]
    fn converting_a_single_mvt_file_writes_one_mlt_file() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt/0_0_0.mvt");
        let output = TempPath::new("-tiles");

        convert(&parse_args(&[path_arg(&input), path_arg(&output.0)]))
            .expect("conversion succeeds");

        let mlt = fs::read(output.0.join("0_0_0.mlt")).expect("output tile is written");
        let expected = mvt_to_tile_layers(fs::read(&input).unwrap()).unwrap().len();
        assert_eq!(
            Parser::default().parse_layers(&mlt).unwrap().len(),
            expected
        );
    }

    #[test]
    fn converting_a_pmtiles_archive_rewrites_every_tile_as_mlt() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/omt-planet-20260112.mvt.max1.pmtiles");
        let output = TempPath::new(".pmtiles");

        convert(&parse_args(&[path_arg(&input), path_arg(&output.0)]))
            .expect("conversion succeeds");

        let (tile_type, _, coords) = read_archive(&output.0);
        assert_eq!(tile_type, TileType::Mlt);
        assert_eq!(
            coords,
            [(0, 0, 0), (1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0)]
        );
    }

    #[test]
    fn converting_an_mbtiles_archive_rewrites_every_tile_as_mlt() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt.max1.mbtiles");
        let output = TempPath::new(".pmtiles");

        convert(&parse_args(&[path_arg(&input), path_arg(&output.0)]))
            .expect("conversion succeeds");

        let (tile_type, _, coords) = read_archive(&output.0);
        assert_eq!(tile_type, TileType::Mlt);
        assert_eq!(coords, [(0, 0, 0), (1, 1, 0)]);
    }

    fn verify_omt_conversion(flags: &[&str]) {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt.max1.mbtiles");
        let output = TempPath::new(".pmtiles");
        let mut argv = vec!["--verify"];
        argv.extend_from_slice(flags);
        argv.extend([path_arg(&input), path_arg(&output.0)]);
        convert(&parse_args(&argv)).expect("every tile decodes back to its input");
    }

    #[test]
    fn an_omt_archive_decodes_back_to_its_input() {
        verify_omt_conversion(&[]);
    }

    #[test]
    fn an_omt_archive_decodes_back_to_its_input_through_v2() {
        verify_omt_conversion(&["--mlt-version", "2"]);
    }

    #[test]
    fn an_omt_archive_decodes_back_to_its_input_through_v2_delta2() {
        verify_omt_conversion(&["--mlt-version", "2", "--delta2"]);
    }

    fn field_config(toml: &str) -> TempPath {
        let config = TempPath::new(".toml");
        fs::write(&config.0, toml).unwrap();
        config
    }

    #[test]
    fn an_omt_archive_decodes_back_to_its_input_through_a_fields_file() {
        let config = field_config("[layers.water]\nclass = { split = \"-\", kind = \"str\" }\n");
        verify_omt_conversion(&["--mlt-version", "2", "--fields", path_arg(&config.0)]);
    }

    #[test]
    fn an_mbtiles_tile_that_fails_its_fields_file_fails_the_conversion() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt.max1.mbtiles");
        let config = field_config("[layers.water]\nclass = { split = \",\", kind = \"u64\" }\n");
        let output = TempPath::new(".mbtiles");

        let err = convert(&parse_args(&[
            "--mlt-version",
            "2",
            "--fields",
            path_arg(&config.0),
            path_arg(&input),
            path_arg(&output.0),
        ]))
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "2 tiles failed to convert and are missing from {}, the first with: \
                 layer water: field class of feature 1: \"ocean\" is not a number: \
                 invalid digit found in string",
                output.0.display()
            )
        );
    }

    #[test]
    fn a_fields_file_without_v2_is_rejected() {
        let config = field_config("[layers.water]\nclass = { split = \",\", kind = \"str\" }\n");

        let err = convert(&parse_args(&[
            "--fields",
            path_arg(&config.0),
            "--mlt-version",
            "1",
            "src.mbtiles",
            "dst.mbtiles",
        ]))
        .unwrap_err();

        insta::assert_snapshot!(
            err.to_string(),
            @"--fields parses fields into m-values and nested columns, which need --mlt-version 2"
        );
    }

    #[test]
    fn a_bbox_drops_the_mbtiles_tiles_outside_it_and_clips_the_recorded_bounds() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt.max1.mbtiles");
        let output = TempPath::new(".pmtiles");

        convert(&parse_args(&[
            "--bbox",
            "-30,-30,-20,-20",
            path_arg(&input),
            path_arg(&output.0),
        ]))
        .expect("conversion succeeds");

        let (tile_type, bounds, coords) = read_archive(&output.0);
        assert_eq!(tile_type, TileType::Mlt);
        assert_eq!(bounds, Bounds::new(-30.0, -30.0, -20.0, -20.0));
        assert_eq!(coords, [(0, 0, 0)]);
    }
}
