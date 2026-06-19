//! Helpers shared across the encoding entry points (independent of input format).

use mlt_core::encoder::EncoderConfig;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Build a Python `ValueError` with the given message.
pub(crate) fn val_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

/// Build an [`EncoderConfig`] from the Python-facing options.
/// `sort` collapses the three sort-strategy toggles into one choice, mirroring the `mlt convert` CLI.
pub(crate) fn encoder_config(
    tessellate: bool,
    sort: &str,
    shared_dict: bool,
    fsst: bool,
    fastpfor: bool,
) -> PyResult<EncoderConfig> {
    let (morton, hilbert, id) = match sort {
        "all" => (true, true, true),
        "auto" | "morton" => (true, false, false),
        "hilbert" => (false, true, false),
        "id" => (false, false, true),
        "none" => (false, false, false),
        other => {
            return Err(val_err(format!(
                "invalid 'sort' {other:?}; expected one of: auto, morton, hilbert, id, none"
            )));
        }
    };
    Ok(EncoderConfig::default()
        .with_tessellation(tessellate)
        .with_spatial_morton_sort(morton)
        .with_spatial_hilbert_sort(hilbert)
        .with_id_sort(id)
        .with_shared_dict(shared_dict)
        .with_fsst(fsst)
        .with_fastpfor(fastpfor))
}
