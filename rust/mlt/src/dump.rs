use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use mlt_core::geojson::FeatureCollection;
use mlt_core::{MltError, parse_layers};

use crate::OutputFormat;
use crate::ls::is_mvt_extension;

#[derive(Args)]
pub struct DumpArgs {
    /// Path to a tile file (.mlt, .mvt, .pbf)
    file: PathBuf,

    /// Output format
    #[arg(short, long, default_value_t, value_enum)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AfterDump {
    KeepRaw,
    Decode,
}

pub fn dump(args: &DumpArgs, decode: AfterDump) -> Result<()> {
    let buffer = fs::read(&args.file)?;

    if is_mvt_extension(&args.file) {
        dump_mvt(args, buffer)?;
    } else {
        dump_mlt(args, decode, &buffer)?;
    }
    Ok(())
}

fn dump_mlt(args: &DumpArgs, decode: AfterDump, buffer: &[u8]) -> Result<(), MltError> {
    let mut layers = parse_layers(buffer)?;
    if decode == AfterDump::Decode {
        for layer in &mut layers {
            layer.decode_all()?;
        }
    }

    match args.format {
        OutputFormat::Text => {
            for (i, layer) in layers.iter().enumerate() {
                println!("=== Layer {i} ===");
                println!("{layer:#?}");
            }
        }
        OutputFormat::GeoJson => {
            let fc = FeatureCollection::from_layers(&layers)?;
            println!("{}", serde_json::to_string_pretty(&fc)?);
        }
    }
    Ok(())
}

fn dump_mvt(args: &DumpArgs, buffer: Vec<u8>) -> Result<(), MltError> {
    let fc = mlt_core::mvt::mvt_to_feature_collection(buffer)?;
    match args.format {
        OutputFormat::Text => {
            for (i, feature) in fc.features.iter().enumerate() {
                println!("=== Feature {i} ===");
                println!("{feature:#?}");
            }
        }
        OutputFormat::GeoJson => {
            println!("{}", serde_json::to_string_pretty(&fc)?);
        }
    }
    Ok(())
}
