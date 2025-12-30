pub mod oudin;

use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "pet")?;
    m.add_submodule(&oudin::make_module(py)?)?;
    Ok(m)
}
