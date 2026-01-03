use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use thiserror::Error;

use crate::climate::ClimateError;
use crate::utils::MetricsError;

#[derive(Error, Debug)]
pub enum CalibrationError {
    #[error("{0}")]
    Climate(#[from] ClimateError),
    #[error("{0}")]
    Metrics(#[from] MetricsError),
    #[error("{0}")]
    InvalidModel(String),
    #[error("{0}")]
    InvalidObjective(String),
}

impl From<CalibrationError> for PyErr {
    fn from(err: CalibrationError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Objective {
    Rmse,
    Nse,
    Kge,
}

impl FromStr for Objective {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rmse" => Ok(Self::Rmse),
            "nse" => Ok(Self::Nse),
            "kge" => Ok(Self::Kge),
            _ => Err(format!(
                "Unknown objective function '{}'. Valid options: nse, kge, rmse",
                s
            )),
        }
    }
}
