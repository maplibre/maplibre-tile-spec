pub mod convert;
pub mod dump;
#[cfg(feature = "unstable-v2")]
pub mod from_geojson;
pub mod hexdump;
pub mod ls;
pub mod ui;

use std::process::exit;

use anyhow::Result as AnyResult;
use clap::{Parser, Subcommand, ValueEnum};

// hotpath-alloc installs its own global allocator, so it can't coexist with ours.
#[cfg(not(feature = "hotpath-alloc"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::convert::{ConvertArgs, convert};
use crate::dump::{AfterDump, DumpArgs, dump};
#[cfg(feature = "unstable-v2")]
use crate::from_geojson::{FromGeoJsonArgs, from_geojson};
use crate::hexdump::{HexdumpArgs, hexdump};
use crate::ls::{LsArgs, ls};
use crate::ui::{UiArgs, ui};

#[hotpath::main]
fn main() -> AnyResult<()> {
    match Cli::parse().command {
        Commands::Convert(args) => convert(&args)?,
        #[cfg(feature = "unstable-v2")]
        Commands::FromGeojson(args) => from_geojson(&args)?,
        Commands::Dump(args) => dump(&args, AfterDump::KeepRaw)?,
        Commands::Decode(args) => dump(&args, AfterDump::Decode)?,
        Commands::Hexdump(args) => hexdump(&args)?,
        Commands::Ls(args) => {
            if !ls(&args)? {
                exit(1)
            }
        }
        Commands::Ui(args) => ui(&args)?,
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "mlt", about = "MapLibre Tile format utilities")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert .mlt, .mvt, and .pbf tiles in a directory tree to re-encoded .mlt files
    Convert(ConvertArgs),
    /// Tile a WGS84 `GeoJSON` `FeatureCollection` (2D or 3D) into a directory tree of v2 z/x/y.mlt tiles
    #[cfg(feature = "unstable-v2")]
    FromGeojson(FromGeoJsonArgs),
    /// Parse a tile file (.mlt, .mvt, .pbf) and dump raw layer data without decoding
    Dump(DumpArgs),
    /// Parse a tile file (.mlt, .mvt, .pbf), decode all layers, and dump the result
    Decode(DumpArgs),
    /// Annotated byte/bit-level hexdump of an MLT tile's metadata and streams
    Hexdump(HexdumpArgs),
    /// List tile files with statistics
    Ls(LsArgs),
    /// Visualize a tile file (.mlt, .mvt, .pbf) in an interactive TUI
    Ui(UiArgs),
}

#[derive(Clone, Default, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// `GeoJSON` output
    #[clap(alias = "geojson")]
    GeoJson,
}
