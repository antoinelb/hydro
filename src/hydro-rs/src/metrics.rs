use ndarray::ArrayView1;
use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("observations and simulations must have the same length (got {0} and {1})")]
    LengthMismatch(usize, usize),
}

impl From<MetricsError> for PyErr {
    fn from(err: MetricsError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}

pub fn calculate_rmse(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<f64, MetricsError> {
    check_lengths(observations, simulations)?;
    let sum: f64 = observations
        .iter()
        .zip(simulations)
        .map(|(o, p)| (o - p).powi(2))
        .sum();
    Ok((sum / observations.len() as f64).sqrt())
}

pub fn calculate_nse(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<f64, MetricsError> {
    check_lengths(observations, simulations)?;
    let mean: f64 =
        observations.iter().sum::<f64>() / observations.len() as f64;
    let (numerator, denominator) = observations.iter().zip(simulations).fold(
        (0.0, 0.0),
        |(num, den), (&o, &p)| {
            (num + (o - p).powi(2), den + (o - mean).powi(2))
        },
    );
    Ok(1.0 - numerator / denominator)
}

pub fn calculate_kge(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<f64, MetricsError> {
    check_lengths(observations, simulations)?;
    let observations_mean =
        observations.iter().sum::<f64>() / observations.len() as f64;
    let observations_mean_2 =
        observations.iter().map(|x| x.powi(2)).sum::<f64>()
            / observations.len() as f64;
    let simulations_mean =
        simulations.iter().sum::<f64>() / observations.len() as f64;
    let simulations_mean_2 =
        simulations.iter().map(|x| x.powi(2)).sum::<f64>()
            / observations.len() as f64;
    let observations_simulations_mean = observations
        .iter()
        .zip(simulations)
        .map(|(o, p)| o * p)
        .sum::<f64>()
        / observations.len() as f64;

    let observations_std =
        (observations_mean_2 - observations_mean.powi(2)).sqrt();
    let simulations_std =
        (simulations_mean_2 - simulations_mean.powi(2)).sqrt();
    let covariance =
        observations_simulations_mean - observations_mean * simulations_mean;

    let r: f64 = covariance / (observations_std * simulations_std);
    let alpha: f64 = simulations_std / observations_std;
    let beta: f64 = simulations_mean / observations_mean;

    Ok(1.
        - ((r - 1.).powi(2) + (alpha - 1.).powi(2) + (beta - 1.).powi(2))
            .sqrt())
}

fn check_lengths(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<(), MetricsError> {
    if observations.len() != simulations.len() {
        Err(MetricsError::LengthMismatch(
            observations.len(),
            simulations.len(),
        ))
    } else {
        Ok(())
    }
}

#[pyfunction]
#[pyo3(name = "calculate_rmse")]
pub fn py_calculate_rmse<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    simulations: PyReadonlyArray1<'py, f64>,
) -> PyResult<f64> {
    Ok(calculate_rmse(
        observations.as_array(),
        simulations.as_array(),
    )?)
}

#[pyfunction]
#[pyo3(name = "calculate_nse")]
pub fn py_calculate_nse<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    simulations: PyReadonlyArray1<'py, f64>,
) -> PyResult<f64> {
    Ok(calculate_nse(
        observations.as_array(),
        simulations.as_array(),
    )?)
}

#[pyfunction]
#[pyo3(name = "calculate_kge")]
pub fn py_calculate_kge<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    simulations: PyReadonlyArray1<'py, f64>,
) -> PyResult<f64> {
    Ok(calculate_kge(
        observations.as_array(),
        simulations.as_array(),
    )?)
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "metrics")?;
    m.add_function(wrap_pyfunction!(py_calculate_rmse, &m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_nse, &m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_kge, &m)?)?;
    Ok(m)
}
