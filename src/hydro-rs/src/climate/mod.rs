mod bucket;
mod gr4j;

use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "climate")?;
    m.add_submodule(&gr4j::make_module(py)?)?;
    m.add_submodule(&bucket::make_module(py)?)?;
    Ok(m)
}
