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
    fpf: bool,
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
    Ok(EncoderConfig {
        tessellate,
        try_spatial_morton_sort: morton,
        try_spatial_hilbert_sort: hilbert,
        try_id_sort: id,
        allow_shared_dict: shared_dict,
        allow_fsst: fsst,
        allow_fpf: fpf,
    })
}
