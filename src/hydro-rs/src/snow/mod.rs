pub mod cemaneige;

use pyo3::prelude::*;

pub const Models: &[&str] = &["cemaneige"];

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "snow")?;
    m.add("Models", Models)?;
    m.add_submodule(&cemaneige::make_module(py)?)?;
    Ok(m)
}
