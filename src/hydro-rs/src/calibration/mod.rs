mod sce;
mod utils;

use crate::utils::register_submodule;
use pyo3::prelude::*;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "calibration")?;
    register_submodule(py, &m, &sce::make_module(py)?, "hydro_rs.calibration")?;
    Ok(m)
}
