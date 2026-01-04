pub mod oudin;

use crate::utils::register_submodule;
use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "pet")?;
    register_submodule(py, &m, &oudin::make_module(py)?, "hydro_rs.pet")?;
    Ok(m)
}
