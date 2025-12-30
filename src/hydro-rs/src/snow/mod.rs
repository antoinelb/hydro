pub mod cemaneige;

use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "snow")?;
    m.add_submodule(&cemaneige::make_module(py)?)?;
    Ok(m)
}
