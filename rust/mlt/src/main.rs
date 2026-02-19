#![cfg(feature = "cli")]

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::cli::dump::{AfterDump, DumpArgs, dump};
use crate::cli::ls::{LsArgs, ls};
use crate::cli::ui::{UiArgs, ui};
mod cli;

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
    /// Parse an MLT file and dump raw layer data without decoding
    Dump(DumpArgs),
    /// Parse an MLT file, decode all layers, and dump the result
    Decode(DumpArgs),
    /// List .mlt files with statistics
    Ls(LsArgs),
    /// Visualize an MLT file in an interactive TUI
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
