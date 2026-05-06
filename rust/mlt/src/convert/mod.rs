mod files;
mod tileset;

use std::path::PathBuf;

use anyhow::{Result as AnyResult, bail};
use bytes::Bytes;
use clap::{Args, ValueEnum};
use indicatif::ProgressState;
use martin_tile_utils::{Encoding, decode_brotli, decode_gzip, decode_zlib, decode_zstd};
use mbtiles::{MbtType, NormalizedSchema};
use mlt_core::encoder::{EncodedUnknown, Encoder, EncoderConfig};
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{Decoder, Layer, Parser};

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "state.per_sec() is always non-negative and well below 2^63 tiles/sec"
)]
fn whole_rate_per_sec(state: &ProgressState, w: &mut dyn std::fmt::Write) {
    let _ = w.write_fmt(format_args!("{}/s", state.per_sec() as u64));
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum TileFormat {
    Mbtiles,
    Pmtiles,
    Files,
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
            Some("mvt") => Self::Mvt,
            _ => Self::Mlt,
        }
    }
}

#[derive(Clone, Default, ValueEnum)]
enum SortMode {
    /// Try all sort strategies and keep the smallest result
    #[default]
    Auto,
    /// Do not reorder features (original order only)
    None,
    /// Only try Z-order (Morton) curve sort
    Morton,
    /// Only try Hilbert curve sort
    Hilbert,
    /// Only try feature-ID ascending sort
    Id,
}

#[derive(Args)]
pub struct ConvertArgs {
    /// Input: a directory with .mlt/.mvt tiles, a single tile file, an .mbtiles database
    input: PathBuf,
    /// Output: a directory for re-encoded .mlt files, an .mbtiles database or a .pmtiles file
    output: PathBuf,
    /// Add tessellation
    #[clap(short, long, default_value = "false")]
    tessellate: bool,
    /// Sort strategy to try when re-encoding (encoder keeps the smallest result)
    #[clap(long, default_value = "auto")]
    sort: SortMode,
    /// Schema type for the output `.mbtiles` file; defaults to the input file's schema
    #[clap(long)]
    mbtiles_format: Option<MbtFormat>,
    /// Disable grouping of similar string columns into shared dictionaries
    #[clap(long, default_value = "false")]
    no_shared_dict: bool,
    /// Output tile format (`mlt` re-encodes; `mvt` decodes MLT inputs back to MVT)
    #[clap(long, default_value = "mlt")]
    to: TileFormat,
}

impl ConvertArgs {
    pub fn input_format(&self) -> TileFormat {
        match self
            .input
            .extension()
            .as_deref()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
        {
            "mbtiles" => TileFormat::Mbtiles,
            "pmtiles" => TileFormat::Pmtiles,
            _ => TileFormat::Files,
        }
    }
    pub fn output_format(&self) -> TileFormat {
        match self
            .output
            .extension()
            .as_deref()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
        {
            "mbtiles" => TileFormat::Mbtiles,
            "pmtiles" => TileFormat::Pmtiles,
            _ => TileFormat::Files,
        }
    }
}

pub fn convert(args: &ConvertArgs) -> AnyResult<()> {
    let cfg = EncoderConfig {
        tessellate: args.tessellate,
        try_spatial_morton_sort: matches!(args.sort, SortMode::Auto | SortMode::Morton),
        try_spatial_hilbert_sort: matches!(args.sort, SortMode::Auto | SortMode::Hilbert),
        try_id_sort: matches!(args.sort, SortMode::Auto | SortMode::Id),
        allow_shared_dict: !args.no_shared_dict,
        ..Default::default()
    };

    if args.input_format() == TileFormat::Mbtiles {
        if args.to == TileFormat::Mvt {
            bail!(
                "--to mvt is not supported for .mbtiles input/output yet; convert to a directory instead"
            );
        }
        if args.output_format() == TileFormat::Files {
            bail!(
                "Output must be either an .mbtiles or a .pmtiles file when input is an .mbtiles file, got: {}",
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

        return tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?
            .block_on(tileset::convert_tiles(
                (&args.input, args.input_format()),
                (&args.output, args.output_format()),
                cfg,
                args.mbtiles_format,
            ));
    }

    files::convert_files(&args.input, &args.output, cfg, args.to)
}

fn convert_mlt_buffer(buffer: &[u8], cfg: EncoderConfig) -> AnyResult<Vec<u8>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut out: Vec<u8> = Vec::new();

    for layer in layers {
        match layer {
            Layer::Tag01(l) => {
                let tile = l.into_tile(&mut dec)?;
                out.extend_from_slice(&tile.encode(cfg)?);
            }
            Layer::Unknown(u) => {
                out.extend(EncodedUnknown::from(u).write_to(Encoder::default())?.data);
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

fn encode_one(data: Vec<u8>, encoding: Encoding, cfg: EncoderConfig) -> AnyResult<Bytes> {
    let mvt = match encoding {
        Encoding::Gzip => decode_gzip(&data)?,
        Encoding::Zlib => decode_zlib(&data)?,
        Encoding::Brotli => decode_brotli(&data)?,
        Encoding::Zstd => decode_zstd(&data)?,
        Encoding::Uncompressed | Encoding::Internal => data,
    };
    convert_mvt_buffer(mvt, cfg).map(Bytes::from_owner)
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
