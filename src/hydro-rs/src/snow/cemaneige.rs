use ndarray::{array, Array1, Array2};
use numpy::{PyArray1, PyArray2, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;

use crate::model::{Data, Error, Metadata, PyData, PyMetadata};

pub const TEMPERATURE_GRADIENT: [f64; 365] =
    include!("temperature_gradient.txt");

pub fn init() -> (Array1<f64>, Array2<f64>) {
    // corresponds to ctg, kf, qnbv
    let default_values = array![0.25, 3.74, 350.0];
    let bounds = array![[0.0, 1.0], [0.0, 20.0], [50.0, 800.0]];
    (default_values, bounds)
}

pub fn simulate(
    params: &Array1<f64>,
    data: &Data,
    metadata: &Metadata,
) -> Result<Array1<f64>, Error> {
    let [ctg, kf, qnbv]: [f64; 3] = params
        .as_slice()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::ParamsMismatch(3, params.len()))?;

    let precipitation = &data.precipitation;
    let temperature = &data.temperature;
    let day_of_year = &data.day_of_year;
    let elevation_layers = &metadata.elevation_layers;
    let median_elevation = metadata.median_elevation;

    let beta = 0.0;
    let vmin = 0.1;
    let tf = 0.0;
    let n_layers = elevation_layers.len();
    let g_threshold = qnbv * 0.9;
    let n_timesteps = precipitation.len();

    let elevation_offsets: Vec<f64> = elevation_layers
        .iter()
        .map(|&z| (z - median_elevation) / 100.0)
        .collect();

    let precip_weights: Vec<f64> = elevation_layers
        .iter()
        .map(|&z| (beta * (z - median_elevation)).exp())
        .collect();
    let normalization: f64 = precip_weights.iter().sum();

    let mut effective_precipitation: Vec<f64> =
        Vec::with_capacity(n_timesteps);

    let mut snowpack: Vec<f64> = vec![0.0; n_layers];
    let mut thermal_state: Vec<f64> = vec![0.0; n_layers];

    let mut layer_temp: Vec<f64> = vec![0.0; n_layers];

    for t in 0..n_timesteps {
        let theta = TEMPERATURE_GRADIENT[(day_of_year[t] - 1) % 365];
        let temp_t = temperature[t];
        let precip_t = precipitation[t];

        let mut total_liquid: f64 = 0.0;
        let mut total_melt: f64 = 0.0;

        for i in 0..n_layers {
            let layer_temperature = elevation_offsets[i] * theta + temp_t;
            layer_temp[i] = layer_temperature;

            let layer_precip = precip_t * precip_weights[i] / normalization;

            let solid_fraction = if layer_temperature > 3.0 {
                0.0
            } else if layer_temperature < -1.0 {
                1.0
            } else {
                1.0 - (layer_temperature + 1.0) / 4.0
            };

            let p_solid = solid_fraction * layer_precip;
            let p_liquid = layer_precip - p_solid;
            total_liquid += p_liquid;

            snowpack[i] += p_solid;

            thermal_state[i] = (thermal_state[i] * ctg
                + layer_temperature * (1.0 - ctg))
                .min(0.0);
        }

        for i in 0..n_layers {
            let layer_temperature = layer_temp[i];

            let potential =
                if thermal_state[i] >= tf && layer_temperature > 0.0 {
                    let max_melt = (layer_temperature - tf) * kf;
                    snowpack[i].min(max_melt)
                } else {
                    0.0
                };

            let fnts = (snowpack[i] / g_threshold).min(1.0);
            let melt_factor = fnts * (1.0 - vmin) + vmin;

            let snow_melt = potential * melt_factor;
            snowpack[i] -= snow_melt;
            total_melt += snow_melt;
        }

        effective_precipitation.push(total_liquid + total_melt);
    }

    Ok(Array1::from_vec(effective_precipitation))
}

#[pyfunction]
#[pyo3(name = "init")]
pub fn py_init<'py>(
    py: Python<'py>,
) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray2<f64>>) {
    let (default_values, bounds) = init();
    (default_values.to_pyarray(py), bounds.to_pyarray(py))
}

#[pyfunction]
#[pyo3(name = "simulate")]
pub fn py_simulate<'py>(
    py: Python<'py>,
    params: PyReadonlyArray1<f64>,
    data: PyData,
    metadata: PyMetadata,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let simulation = simulate(
        &params.as_array().to_owned(),
        &data.into_data()?,
        &metadata.into_metadata(),
    )?;
    Ok(simulation.to_pyarray(py))
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "cemaneige")?;
    m.add_function(wrap_pyfunction!(py_init, &m)?)?;
    m.add_function(wrap_pyfunction!(py_simulate, &m)?)?;
    Ok(m)
}
