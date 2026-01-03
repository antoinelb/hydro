use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClimateError {
    #[error("precipitation and pet must have the same length (got {0} and {1})")]
    LengthMismatch(usize, usize),
    #[error("expected {0} params, got {1}")]
    ParamsMismatch(usize, usize),
}

impl From<ClimateError> for PyErr {
    fn from(err: ClimateError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}
