use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context as _, Result as AnyResult, bail};
use clap::Args;
use mlt_core::mvt::mvt_to_tile_layers;
use mlt_core::v01::{EncoderSettings, Tile01Encoder};
use mlt_core::{Decoder, EncodedLayer, Layer, Parser};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use crate::ls::is_mlt_extension;

#[derive(Args)]
pub struct ConvertArgs {
    /// Input directory containing .mlt and .mvt tiles, or single file
    input: PathBuf,
    /// Output directory where re-encoded .mlt files will be written with same structure
    output: PathBuf,
    /// Add tessellation
    #[clap(short, long, default_value = "false")]
    tessellate: bool,
}

pub fn convert(args: &ConvertArgs) -> AnyResult<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_tile_files(&args.input, &mut files)?;
    if files.is_empty() {
        eprintln!("No .mlt or .mvt files found in {}", args.input.display());
        return Ok(());
    }

    // Determine the base for computing relative paths.
    let base = if args.input.is_dir() {
        args.input.as_path()
    } else {
        args.input.parent().unwrap_or(Path::new("."))
    };

    let cfg = EncoderSettings {
        tessellate: args.tessellate,
        ..Default::default()
    };

    let failed = AtomicUsize::new(0);
    files.par_iter().for_each(|file| {
        if let Err(e) = convert_file(file, base, &args.output, cfg) {
            eprintln!("error: {}: {e:#}", file.display());
            failed.fetch_add(1, Ordering::Relaxed);
        }
    });

    let n = failed.into_inner();
    if n > 0 {
        bail!("{n} file(s) failed to convert");
    }
    Ok(())
}

fn convert_file(file: &Path, base: &Path, output: &Path, cfg: EncoderSettings) -> AnyResult<()> {
    let rel = file
        .strip_prefix(base)
        .with_context(|| format!("stripping prefix from {}", file.display()))?;
    let out_path = output.join(rel).with_extension("mlt");

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let buffer = fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let out_bytes = if is_mlt_extension(file) {
        convert_mlt_buffer(&buffer, cfg)
            .with_context(|| format!("converting MLT {}", file.display()))?
    } else {
        convert_mvt_buffer(buffer, cfg)
            .with_context(|| format!("converting MVT {}", file.display()))?
    };

    fs::write(&out_path, &out_bytes).with_context(|| format!("writing {}", out_path.display()))?;

    println!("{} -> {}", file.display(), out_path.display());
    Ok(())
}

/// Recursively collect all `.mlt` and `.mvt` files under `path`.
fn collect_tile_files(path: &Path, files: &mut Vec<PathBuf>) -> AnyResult<()> {
    if path.is_dir() {
        for entry in
            fs::read_dir(path).with_context(|| format!("reading directory {}", path.display()))?
        {
            let child = entry?.path();
            if child.is_file() && is_convert_extension(&child) {
                files.push(child);
            } else if child.is_dir() {
                collect_tile_files(&child, files)?;
            }
        }
    } else if path.is_file() && is_convert_extension(path) {
        files.push(path.to_path_buf());
    }
    Ok(())
}

fn is_convert_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mlt" | "mvt")
    )
}

/// Re-encode an MLT tile using automatic encoding selection.
///
/// Every Tag01 layer is fully decoded to [`TileLayer01`] and then re-encoded
/// via [`Tile01Encoder::encode_auto`].  Unknown layer tags are passed through
/// unchanged.
fn convert_mlt_buffer(buffer: &[u8], cfg: EncoderSettings) -> AnyResult<Vec<u8>> {
    let layers = Parser::default().parse_layers(buffer)?;
    let mut dec = Decoder::default();
    let mut out: Vec<u8> = Vec::new();

    for layer in layers {
        match layer {
            Layer::Tag01(l) => {
                let tile = l.into_tile(&mut dec)?;
                let (encoded, _) = Tile01Encoder::encode_auto(&tile, cfg)?;
                EncodedLayer::Tag01(encoded).write_to(&mut out)?;
            }
            Layer::Unknown(u) => EncodedLayer::from(u).write_to(&mut out)?,
        }
    }

    Ok(out)
}

/// Convert an MVT tile to an MLT tile using automatic encoding selection.
///
/// Each MVT layer is converted to a [`mlt_core::v01::TileLayer01`] and encoded
/// via [`Tile01Encoder::encode_auto`].
fn convert_mvt_buffer(buffer: Vec<u8>, cfg: EncoderSettings) -> AnyResult<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for tile in &mvt_to_tile_layers(buffer)? {
        let (encoded, _) = Tile01Encoder::encode_auto(tile, cfg)?;
        EncodedLayer::Tag01(encoded).write_to(&mut out)?;
    }
    Ok(out)
}
