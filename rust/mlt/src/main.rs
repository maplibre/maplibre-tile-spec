pub mod convert;
pub mod dump;
pub mod hexdump;
pub mod ls;
pub mod ui;

use std::process::exit;

use anyhow::Result as AnyResult;
use clap::{Parser, Subcommand, ValueEnum};

// hotpath-alloc installs its own global allocator to track allocations, so it
// can't coexist with ours.
#[cfg(not(feature = "hotpath-alloc"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::convert::{ConvertArgs, convert};
use crate::dump::{AfterDump, DumpArgs, dump};
use crate::hexdump::{HexdumpArgs, hexdump};
use crate::ls::{LsArgs, ls};
use crate::ui::{UiArgs, ui};

#[hotpath::main]
fn main() -> AnyResult<()> {
    match Cli::parse().command {
        Commands::Convert(args) => convert(&args)?,
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
