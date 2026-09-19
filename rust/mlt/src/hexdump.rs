use std::fs;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{Result as AnyResult, bail};
use clap::{Args, ValueEnum};
use mlt_core::dump::{self, RenderOpts};

use crate::ls::is_mlt_extension;

#[derive(Args)]
pub struct HexdumpArgs {
    /// Path to an MLT tile file (.mlt)
    file: PathBuf,

    /// Only dump this layer index (0-based)
    #[arg(long)]
    layer: Option<usize>,

    /// How to render data-stream payloads
    #[arg(long, value_enum, default_value_t = DataMode::Both)]
    data: DataMode,

    /// Truncate raw payload hex to this many bytes (0 = unlimited)
    #[arg(long, default_value_t = 256)]
    max_blob: usize,

    /// Hide the bit-level breakdown of packed bytes
    #[arg(long)]
    no_bits: bool,

    /// Colorize output
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    color: ColorWhen,

    /// Hex bytes per row
    #[arg(long, default_value_t = 16)]
    width: usize,
}

#[derive(Clone, Copy, ValueEnum)]
enum DataMode {
    /// Raw hex plus decoded values
    Both,
    /// Raw hex only
    Blob,
    /// Decoded values only
    Decoded,
    /// One-line summary, no payload bytes
    Hidden,
}

impl From<DataMode> for dump::DataMode {
    fn from(m: DataMode) -> Self {
        match m {
            DataMode::Both => Self::Both,
            DataMode::Blob => Self::Blob,
            DataMode::Decoded => Self::Decoded,
            DataMode::Hidden => Self::Hidden,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

pub fn hexdump(args: &HexdumpArgs) -> AnyResult<()> {
    if !is_mlt_extension(&args.file) {
        bail!("`hexdump` only supports MLT files (.mlt); MVT/PBF tiles are protobuf-encoded");
    }
    let buffer = fs::read(&args.file)?;
    let (tree, err) = dump::annotate_tile(&buffer);

    // A layer the walk never reached is explained by `err`, not by being out of range.
    let selected = match args.layer {
        Some(idx) => dump::filter_layer(&tree, idx).ok_or(idx),
        None => Ok(tree),
    };

    let color = match args.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => io::stdout().is_terminal(),
    };
    let opts = RenderOpts {
        width: args.width.max(1),
        show_bits: !args.no_bits,
        color,
        data_mode: args.data.into(),
        max_blob: args.max_blob,
    };

    if let Ok(tree) = &selected {
        let stdout = io::stdout();
        let mut w = BufWriter::new(stdout.lock());
        dump::render(tree, &buffer, &opts, &mut w)?;
        w.flush()?;
    }

    // Whatever is left only has to fail the exit code, the dump is already written.
    if let Some(err) = err {
        return Err(err.into());
    }
    match selected {
        Ok(_) => Ok(()),
        Err(idx) => bail!("layer index {idx} out of range"),
    }
}
