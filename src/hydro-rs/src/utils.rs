use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "utils")?;
    Ok(m)
}
