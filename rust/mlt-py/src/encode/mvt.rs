//! Encode an entire MVT tile to MLT bytes.

use mlt_core::MltResult;
use mlt_core::encoder::EncoderConfig;
use mlt_core::mvt::mvt_to_tile_layers;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

/// Encode an entire MVT tile to MLT using default encoding options.
///
/// `data` is a raw Mapbox Vector Tile (protobuf).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn encode_mvt(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "bytes"))] data: &[u8],
) -> PyResult<Py<PyBytes>> {
    let bytes = py
        .detach(|| -> MltResult<Vec<u8>> {
            let data = data.to_vec();
            let mut out = Vec::new();
            for tile in mvt_to_tile_layers(data)? {
                out.extend_from_slice(&tile.encode(EncoderConfig::default())?);
            }
            Ok(out)
        })
        .map_err(|e| PyValueError::new_err(format!("MLT encode error: {e}")))?;
    Ok(PyBytes::new(py, &bytes).unbind())
}
