pub mod gr4j;
use ndarray::{Array1, Array2};

use crate::model::{Data, Error, Metadata};
use crate::utils::register_submodule;
use pyo3::prelude::*;

pub fn get_model(
    model: &str,
) -> Result<
    (
        fn() -> (Array1<f64>, Array2<f64>),
        fn(&Array1<f64>, &Data, &Metadata) -> Result<Array1<f64>, Error>,
    ),
    Error,
> {
    match model {
        "gr4j" => Ok((gr4j::init, gr4j::simulate)),
        _ => Err(Error::WrongModel(model.to_string(), "gr4j".to_string())),
    }
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "climate")?;
    register_submodule(py, &m, &gr4j::make_module(py)?, "hydro_rs.climate")?;
    Ok(m)
}
