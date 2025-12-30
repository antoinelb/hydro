use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn calculate_rmse(observations: &[f64], predictions: &[f64]) -> f64 {
    if observations.len() != predictions.len() {
        f64::NAN
    } else {
        let sum: f64 = observations
            .iter()
            .zip(predictions)
            .map(|(o, p)| (o - p).powi(2))
            .sum();
        (sum / observations.len() as f64).sqrt()
    }
}

pub fn calculate_nse(observations: &[f64], predictions: &[f64]) -> f64 {
    if observations.len() != predictions.len() {
        f64::NAN
    } else {
        let mean: f64 = observations.iter().sum::<f64>() / observations.len() as f64;
        let (numerator, denominator) = observations
            .iter()
            .zip(predictions)
            .fold((0.0, 0.0), |(num, den), (&o, &p)| {
                (num + (o - p).powi(2), den + (o - mean).powi(2))
            });
        1.0 - numerator / denominator
    }
}

pub fn calculate_kge(observations: &[f64], predictions: &[f64]) -> f64 {
    if observations.len() != predictions.len() {
        f64::NAN
    } else {
        let observations_mean = observations.iter().sum::<f64>() / observations.len() as f64;
        let observations_mean_2 =
            observations.iter().map(|x| x.powi(2)).sum::<f64>() / observations.len() as f64;
        let predictions_mean = predictions.iter().sum::<f64>() / observations.len() as f64;
        let predictions_mean_2 =
            predictions.iter().map(|x| x.powi(2)).sum::<f64>() / observations.len() as f64;
        let observations_predictions_mean = observations
            .iter()
            .zip(predictions)
            .map(|(o, p)| o * p)
            .sum::<f64>()
            / observations.len() as f64;

        let observations_std = (observations_mean_2 - observations_mean.powi(2)).sqrt();
        let predictions_std = (predictions_mean_2 - predictions_mean.powi(2)).sqrt();
        let covariance = observations_predictions_mean - observations_mean * predictions_mean;

        let r: f64 = covariance / (observations_std * predictions_std);
        let alpha: f64 = predictions_std / observations_std;
        let beta: f64 = predictions_mean / observations_mean;

        1. - ((r - 1.).powi(2) + (alpha - 1.).powi(2) + (beta - 1.).powi(2)).sqrt()
    }
}

#[gen_stub_pyfunction(module = "hydro_rs.utils")]
#[pyfunction]
#[pyo3(name = "calculate_rmse")]
pub fn py_calculate_rmse<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    predictions: PyReadonlyArray1<'py, f64>,
) -> f64 {
    calculate_rmse(
        observations.as_slice().unwrap(),
        predictions.as_slice().unwrap(),
    )
}

#[gen_stub_pyfunction(module = "hydro_rs.utils")]
#[pyfunction]
#[pyo3(name = "calculate_nse")]
pub fn py_calculate_nse<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    predictions: PyReadonlyArray1<'py, f64>,
) -> f64 {
    calculate_nse(
        observations.as_slice().unwrap(),
        predictions.as_slice().unwrap(),
    )
}

#[gen_stub_pyfunction(module = "hydro_rs.utils")]
#[pyfunction]
#[pyo3(name = "calculate_kge")]
pub fn py_calculate_kge<'py>(
    observations: PyReadonlyArray1<'py, f64>,
    predictions: PyReadonlyArray1<'py, f64>,
) -> f64 {
    calculate_kge(
        observations.as_slice().unwrap(),
        predictions.as_slice().unwrap(),
    )
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "utils")?;
    m.add_function(wrap_pyfunction!(py_calculate_rmse, &m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_nse, &m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_kge, &m)?)?;
    Ok(m)
}
