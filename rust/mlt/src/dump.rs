use std::fs;
use std::path::PathBuf;

use anyhow::{Result as AnyResult, bail};
use clap::Args;
use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, MltResult, Parser};

use crate::OutputFormat;
use crate::ls::is_mlt_extension;

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

pub fn dump(args: &DumpArgs, decode: AfterDump) -> AnyResult<()> {
    let buffer = fs::read(&args.file)?;

    if is_mlt_extension(&args.file) {
        dump_mlt(args, decode, &buffer)?;
    } else {
        dump_mvt(args, buffer)?;
    }
    Ok(())
}

fn dump_mlt(args: &DumpArgs, decode: AfterDump, buffer: &[u8]) -> AnyResult<()> {
    let layers = Parser::default().parse_layers(buffer)?;

    match args.format {
        OutputFormat::Text => match decode {
            AfterDump::KeepRaw => {
                for (i, layer) in layers.into_iter().enumerate() {
                    println!("=== Layer {i} ===");
                    println!("{layer:#?}");
                }
            }
            AfterDump::Decode => {
                let layers = Decoder::default().decode_all(layers)?;
                for (i, layer) in layers.into_iter().enumerate() {
                    println!("=== Layer {i} ===");
                    println!("{layer:#?}");
                }
            }
        },
        OutputFormat::GeoJson => {
            if decode == AfterDump::KeepRaw {
                bail!("GeoJSON output only works with `mlt decode`");
            }
            let fc = FeatureCollection::from_layers(Decoder::default().decode_all(layers)?)?;
            println!("{}", serde_json::to_string_pretty(&fc)?);
        }
    }
    Ok(())
}

fn dump_mvt(args: &DumpArgs, buffer: Vec<u8>) -> MltResult<()> {
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
