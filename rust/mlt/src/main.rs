pub mod dump;
pub mod ls;
pub mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::dump::{AfterDump, DumpArgs, dump};
use crate::ls::{LsArgs, ls};
use crate::ui::{UiArgs, ui};

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Dump(args) => dump(&args, AfterDump::KeepRaw)?,
        Commands::Decode(args) => dump(&args, AfterDump::Decode)?,
        Commands::Ls(args) => ls(&args)?,
        Commands::Ui(args) => ui(&args)?,
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "mlt", about = "MapLibre Tile format utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a tile file (.mlt, .mvt, .pbf) and dump raw layer data without decoding
    Dump(DumpArgs),
    /// Parse a tile file (.mlt, .mvt, .pbf), decode all layers, and dump the result
    Decode(DumpArgs),
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
