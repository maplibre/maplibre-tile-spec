use std::{fs, path::PathBuf};

use crate::OutputFormat;
use anyhow::Result;
use clap::Args;
use mlt_nom::{geojson::FeatureCollection, parse_layers};

#[derive(Args)]
pub struct DumpArgs {
    /// Path to the MLT file
    file: PathBuf,

    /// Output format
    #[arg(short, long, default_value_t, value_enum)]
    format: OutputFormat,
}

pub fn dump(args: &DumpArgs, decode: bool) -> Result<()> {
    let buffer = fs::read(&args.file)?;
    let mut layers = parse_layers(&buffer)?;
    if decode {
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
