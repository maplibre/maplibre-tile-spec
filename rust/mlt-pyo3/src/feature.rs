use pyo3::types::{PyBytes, PyDict};
use pyo3::{Py, pyclass};

/// A decoded MLT feature with geometry, id, and properties.
#[pyclass]
pub struct MltFeature {
    #[pyo3(get)]
    id: Option<u64>,
    #[pyo3(get)]
    geometry_type: String,
    #[pyo3(get)]
    wkb: Py<PyBytes>,
    #[pyo3(get)]
    properties: Py<PyDict>,
}

impl MltFeature {
    pub(crate) fn new(
        id: Option<u64>,
        geometry_type: String,
        wkb: Py<PyBytes>,
        properties: Py<PyDict>,
    ) -> Self {
        MltFeature {
            id,
            geometry_type,
            wkb,
            properties,
        }
    }
}
