use std::fs;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{Result as AnyResult, bail};
use clap::{Args, ValueEnum};
use mlt_core::dump::{self, DumpTree, RenderOpts};

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
    let tree = dump::annotate_tile(&buffer)?;

    let tree = match args.layer {
        Some(idx) => filter_layer(&tree, idx)?,
        None => tree,
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

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    dump::render(&tree, &buffer, &opts, &mut w)?;
    w.flush()?;
    Ok(())
}

/// Keep only the regions belonging to the `idx`-th top-level layer container.
fn filter_layer(tree: &DumpTree, idx: usize) -> AnyResult<DumpTree> {
    let layer = tree
        .regions
        .iter()
        .filter(|r| r.depth == 0 && r.container)
        .nth(idx)
        .ok_or_else(|| anyhow::anyhow!("layer index {idx} out of range"))?;
    let start = layer.offset;
    let end = layer.offset + layer.len;
    let regions = tree
        .regions
        .iter()
        .filter(|r| r.offset >= start && r.offset + r.len <= end)
        .cloned()
        .collect();
    Ok(DumpTree {
        buf_len: tree.buf_len,
        regions,
    })
}
