mod sce;

use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "calibration")?;
    m.add_submodule(&sce::make_module(py)?)?;
    Ok(m)
}
