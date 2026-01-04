pub mod cemaneige;
use ndarray::{Array1, Array2};

use crate::model::{Error, SimulateFnPtr};
use crate::utils::register_submodule;
use pyo3::prelude::*;

pub fn get_model(
    model: &str,
) -> Result<(fn() -> (Array1<f64>, Array2<f64>), SimulateFnPtr), Error> {
    match model {
        "cemaneige" => Ok((cemaneige::init, cemaneige::simulate)),
        _ => Err(Error::WrongModel(
            model.to_string(),
            "cemaneige".to_string(),
        )),
    }
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "snow")?;
    register_submodule(py, &m, &cemaneige::make_module(py)?, "hydro_rs.snow")?;
    Ok(m)
}
