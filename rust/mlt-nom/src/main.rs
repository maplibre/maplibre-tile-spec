#![cfg(feature = "cli")]

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::cli::dump::{DumpArgs, dump};
use crate::cli::ls::{LsArgs, ls};
mod cli;

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Dump(args) => dump(&args, false)?,
        Commands::Decode(args) => dump(&args, true)?,
        Commands::Ls(args) => ls(&args)?,
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
