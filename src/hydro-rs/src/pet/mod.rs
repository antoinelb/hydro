pub mod oudin;

use pyo3::prelude::*;

pub const Models: &[&str] = &["oudin"];

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "pet")?;
    m.add("Models", Models)?;
    m.add_submodule(&oudin::make_module(py)?)?;
    Ok(m)
}
