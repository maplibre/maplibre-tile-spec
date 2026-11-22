//! The `z/x/y.mlt` directory that a `GeoJSON` or `CityGML` input is tiled into.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result as AnyResult};
use mlt_core::{Decoder, MltError, Parser, TileLayer};

use super::Reencoder;

/// Encodes `layer` as the tile `zoom/x/y` and writes it under `output`.
/// Returns `false`, writing nothing, when the tile needs more than the default decoder budget.
pub(super) fn write_tile(
    output: &Path,
    reencoder: &Reencoder,
    zoom: u8,
    x: u32,
    y: u32,
    layer: TileLayer,
) -> AnyResult<bool> {
    let checked = reencoder
        .encode(layer)
        .with_context(|| format!("encoding tile {zoom}/{x}/{y}"))
        .and_then(|bytes| {
            // `--verify` has already decoded the tile with the default budget.
            if !reencoder.verify {
                decode_with_default_budget(&bytes)
                    .with_context(|| format!("decoding tile {zoom}/{x}/{y} back"))?;
            }
            Ok(bytes)
        });
    let bytes = match checked {
        Ok(bytes) => bytes,
        Err(e) if is_memory_limit(&e) => return Ok(false),
        Err(e) => return Err(e),
    };
    let dir = output.join(zoom.to_string()).join(x.to_string());
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{y}.mlt"));
    fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// Whether `err` stems from a decoder running out of its memory budget.
fn is_memory_limit(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<MltError>(),
            Some(MltError::MemoryLimitExceeded { .. })
        )
    })
}

/// Decodes `bytes` within the default memory budget, as a renderer reading the tile would.
fn decode_with_default_budget(bytes: &[u8]) -> Result<(), MltError> {
    let layers = Parser::default().parse_layers(bytes)?;
    let mut dec = Decoder::default();
    layers
        .into_iter()
        .try_for_each(|layer| layer.into_tile(&mut dec).map(drop))
}
