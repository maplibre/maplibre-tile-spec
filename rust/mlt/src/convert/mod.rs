mod bbox;
mod common;
mod from_files;
mod from_mbtiles;
mod from_pmtiles;

use std::path::{Path, PathBuf};

use anyhow::{Result as AnyResult, bail};
use bytes::Bytes;
use clap::{Args, ValueEnum};
use indicatif::ProgressState;
use martin_tile_utils::{Encoding, Format, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{MbtType, NormalizedSchema};
#[cfg(feature = "unstable-v2")]
use mlt_core::encoder::WireVersion;
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{Decoder, Layer, Parser};
use pmtiles::Compression;
use tilejson::Bounds;

use crate::convert::bbox::BboxFilter;
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
///
/// Defaults to the newest version the binary was built with.
#[derive(Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum MltVersion {
    /// Version 1
    #[cfg_attr(not(feature = "unstable-v2"), default)]
    #[value(name = "1", alias = "v1")]
    V1,
    /// Version 2
    #[cfg(feature = "unstable-v2")]
    #[cfg_attr(feature = "unstable-v2", default)]
    #[value(name = "2", alias = "v2")]
    V2,
}

#[cfg(feature = "unstable-v2")]
impl From<MltVersion> for WireVersion {
    fn from(version: MltVersion) -> Self {
        match version {
            MltVersion::V1 => Self::V01,
            #[cfg(feature = "unstable-v2")]
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

#[derive(Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI on/off flag, not a state machine"
)]
pub struct ConvertArgs {
    /// Input: a directory with .mlt/.mvt/.pbf tiles, a single tile file, an .mbtiles or .pmtiles archive
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
    /// Disable `ALP` float encoding, storing floats as decimal-scaled integers
    #[cfg(feature = "unstable-v2")]
    #[clap(long)]
    no_alp: bool,
    /// Disable float dictionary encoding
    #[cfg(feature = "unstable-v2")]
    #[clap(long)]
    no_float_dict: bool,
    /// Output tile format (`mlt` re-encodes; `mvt` decodes MLT inputs back to MVT)
    #[clap(long, default_value = "mlt")]
    to: TileFormat,
    /// MLT wire format version to write
    #[cfg_attr(
        not(feature = "unstable-v2"),
        doc = "",
        doc = "Version 2 requires building with `--features unstable-v2`."
    )]
    #[cfg_attr(not(feature = "unstable-v2"), clap(long, default_value = "1"))]
    #[cfg_attr(feature = "unstable-v2", clap(long, default_value = "2"))]
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
    let morton = matches!(args.sort, SortMode::All | SortMode::Auto | SortMode::Morton);
    let hilbert = matches!(args.sort, SortMode::All | SortMode::Hilbert);
    let id_sort = matches!(args.sort, SortMode::All | SortMode::Id);
    let cfg = EncoderConfig::default()
        .with_tessellation(args.tessellate)
        .with_spatial_morton_sort(morton)
        .with_spatial_hilbert_sort(hilbert)
        .with_id_sort(id_sort)
        .with_shared_dict(!args.no_shared_dict)
        .with_fastpfor(!args.no_fastpfor)
        .with_fsst(!args.no_fsst);
    #[cfg(feature = "unstable-v2")]
    let cfg = cfg
        .with_wire_version(args.mlt_version.into())
        .with_float_alp(!args.no_alp)
        .with_float_dict(!args.no_float_dict);

    let filter = BboxFilter::new(&args.bbox)?;
    let input_container = args.input_container();
    let output_container = args.output_container();
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
    if has_archive_input {
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
        return match input_container {
            ContainerFormat::Pmtiles => runtime.block_on(from_pmtiles::convert(
                &args.input,
                output,
                cfg,
                args.tile_compression.into(),
                filter.as_ref(),
            )),
            ContainerFormat::Mbtiles => {
                runtime.block_on(convert_mbtiles(args, output, cfg, filter.as_ref()))
            }
            ContainerFormat::Files => {
                unreachable!("`has_archive_input` above rules out a directory input")
            }
        };
    }

    from_files::convert(&args.input, &args.output, cfg, args.to)
}

/// Converts an `.mbtiles` input, first extracting the requested boxes into a temporary
/// archive so that only those tiles are read and re-encoded.
async fn convert_mbtiles(
    args: &ConvertArgs,
    output: (&Path, ContainerFormat),
    cfg: EncoderConfig,
    filter: Option<&BboxFilter>,
) -> AnyResult<()> {
    let dst_type = args.mbtiles_format.map(MbtType::from);
    let tile_compression = args.tile_compression.into();
    let Some(filter) = filter else {
        return from_mbtiles::convert(&args.input, output, cfg, dst_type, tile_compression, None)
            .await;
    };

    let extract = BboxExtract::create(&args.input, output.0, filter).await?;
    from_mbtiles::convert(
        extract.path(),
        output,
        cfg,
        dst_type.or(Some(extract.source_type)),
        tile_compression,
        Some(filter.bounds()),
    )
    .await
}

fn convert_mlt_buffer(buffer: &[u8], cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut out: Vec<u8> = Vec::new();

    for layer in layers {
        #[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
        match layer {
            Layer::Tag01(l) => {
                let tile = l.into_tile(&mut dec)?;
                out.extend_from_slice(&tile.encode(cfg)?);
            }
            Layer::Unknown(u) => {
                out.extend(
                    EncodedUnknown::from(u)
                        .write_to(Encoder::default())?
                        .into_raw_bytes(),
                );
            }
            _ => {}
        }
    }

    Ok(out)
}

fn convert_mvt_buffer(buffer: Vec<u8>, cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for tile in mvt_to_tile_layers(buffer)? {
        out.extend_from_slice(&tile.encode(cfg)?);
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
        #[expect(clippy::wildcard_enum_match_arm, reason = "Layer is non_exhaustive")]
        match layer {
            Layer::Tag01(l) => {
                tiles.push(l.into_tile(&mut dec)?);
            }
            Layer::Unknown(_) => {
                bail!(
                    "cannot convert MLT tile to MVT: tile contains unknown/extension layers that MVT cannot represent"
                );
            }
            _ => {}
        }
    }
    Ok(tiles)
}

fn encode_one(data: Vec<u8>, encoding: Encoding, cfg: EncoderConfig) -> AnyResult<(Bytes, u64)> {
    let mvt = match encoding {
        Encoding::Gzip => decode_gzip(&data)?,
        Encoding::Zlib => decode_zlib(&data)?,
        Encoding::Brotli => decode_brotli(&data)?,
        Encoding::Zstd => decode_zstd(&data)?,
        Encoding::Uncompressed | Encoding::Internal => data,
    };
    let raw_mvt_size = mvt.len() as u64;
    convert_mvt_buffer(mvt, cfg).map(|data| (Bytes::from_owner(data), raw_mvt_size))
}

/// Convert one input buffer to the requested target format.
fn convert_buffer(
    buffer: Vec<u8>,
    from: TileFormat,
    to: TileFormat,
    cfg: EncoderConfig,
) -> AnyResult<Vec<u8>> {
    match (from, to) {
        (TileFormat::Mlt, TileFormat::Mlt) => convert_mlt_buffer(&buffer, cfg),
        (TileFormat::Mvt, TileFormat::Mlt) => convert_mvt_buffer(buffer, cfg),
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
    use super::*;

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
}
