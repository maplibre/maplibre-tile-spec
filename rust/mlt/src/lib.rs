//! The `mlt` command line tool's subcommands.

pub mod convert;
pub mod dump;
#[cfg(feature = "unstable-v2")]
pub mod from_geojson;
pub mod hexdump;
pub mod ls;
pub mod ui;

use clap::ValueEnum;

#[derive(Clone, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// `GeoJSON` output
    #[clap(alias = "geojson")]
    GeoJson,
}
