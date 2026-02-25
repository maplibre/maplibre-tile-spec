use pyo3::types::{PyBytes, PyDict};
use pyo3::{Py, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// A decoded MLT feature with geometry, id, and properties.
#[gen_stub_pyclass]
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

#[gen_stub_pymethods]
#[pymethods]
impl MltFeature {
    fn __repr__(&self) -> String {
        format!(
            "MltFeature(id={:?}, geometry_type={:?})",
            self.id, self.geometry_type
        )
    }
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
