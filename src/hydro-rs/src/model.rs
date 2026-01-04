use crate::metrics::MetricsError;
use ndarray::{s, Array1, Array2, Axis};
use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("precipitation, temperature, pet and day_of_year must have the same length (got {0}, {1}, {2} and {3})")]
    LengthMismatch(usize, usize, usize, usize),
    #[error("expected {0} params, got {1}")]
    ParamsMismatch(usize, usize),
    #[error("Unknown model '{0}'. Valid options: {1}")]
    WrongModel(String, String),
    #[error(transparent)]
    Metrics(#[from] MetricsError),
}

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}

/// Shared data that doesn't change during calibration (cheap to clone via Arc)
#[derive(Clone)]
pub struct SharedData {
    pub temperature: Arc<Vec<f64>>,
    pub pet: Arc<Vec<f64>>,
    pub day_of_year: Arc<Vec<usize>>,
}

/// Full data including precipitation (which changes when composing snow + climate models)
#[derive(Clone)]
pub struct Data {
    pub precipitation: Vec<f64>,
    pub shared: SharedData,
}

impl Data {
    pub fn new(
        precipitation: Vec<f64>,
        temperature: Vec<f64>,
        pet: Vec<f64>,
        day_of_year: Vec<usize>,
    ) -> Result<Self, Error> {
        if precipitation.len() != temperature.len()
            || precipitation.len() != pet.len()
            || precipitation.len() != day_of_year.len()
        {
            return Err(Error::LengthMismatch(
                precipitation.len(),
                temperature.len(),
                pet.len(),
                day_of_year.len(),
            ));
        }

        Ok(Data {
            precipitation,
            shared: SharedData {
                temperature: Arc::new(temperature),
                pet: Arc::new(pet),
                day_of_year: Arc::new(day_of_year),
            },
        })
    }

    /// Create a new Data with replaced precipitation, sharing the other fields
    pub fn with_precipitation(&self, precipitation: Vec<f64>) -> Self {
        Data {
            precipitation,
            shared: self.shared.clone(), // Cheap: just increments Arc refcounts
        }
    }

    // Convenience accessors
    pub fn temperature(&self) -> &[f64] {
        &self.shared.temperature
    }

    pub fn pet(&self) -> &[f64] {
        &self.shared.pet
    }

    pub fn day_of_year(&self) -> &[usize] {
        &self.shared.day_of_year
    }
}

pub struct Metadata {
    pub elevation_layers: Array1<f64>,
    pub median_elevation: f64,
}

pub type SimulateFn =
    Box<dyn Fn(&Array1<f64>, &Data, &Metadata) -> Result<Array1<f64>, Error>>;

pub fn compose_init(
    snow_init: fn() -> (Array1<f64>, Array2<f64>),
    climate_init: fn() -> (Array1<f64>, Array2<f64>),
) -> impl Fn() -> (Array1<f64>, Array2<f64>, usize) {
    move || {
        let (snow_defaults, snow_bounds) = snow_init();
        let (climate_defaults, climate_bounds) = climate_init();
        let default_values = ndarray::concatenate(
            Axis(0),
            &vec![snow_defaults.view(), climate_defaults.view()],
        )
        .unwrap();
        let bounds = ndarray::concatenate(
            Axis(0),
            &vec![snow_bounds.view(), climate_bounds.view()],
        )
        .unwrap();

        (default_values, bounds, snow_defaults.len())
    }
}

pub fn compose_simulate(
    snow_simulate: fn(
        &Array1<f64>,
        &Data,
        &Metadata,
    ) -> Result<Vec<f64>, Error>,
    climate_simulate: fn(
        &Array1<f64>,
        &Data,
        &Metadata,
    ) -> Result<Vec<f64>, Error>,
    n_snow_params: usize,
) -> impl Fn(&Array1<f64>, &Data, &Metadata) -> Result<Vec<f64>, Error> {
    move |params, data, metadata| {
        let snow_params = params.slice(s![..n_snow_params]).to_owned();
        let climate_params = params.slice(s![n_snow_params..]).to_owned();
        let effective_precipitation =
            snow_simulate(&snow_params, &data, &metadata)?;
        // Use with_precipitation for cheap cloning (just Arc refcount increments)
        let data = data.with_precipitation(effective_precipitation);
        climate_simulate(&climate_params, &data, &metadata)
    }
}

#[derive(FromPyObject)]
pub struct PyData<'py> {
    pub precipitation: PyReadonlyArray1<'py, f64>,
    pub temperature: PyReadonlyArray1<'py, f64>,
    pub pet: PyReadonlyArray1<'py, f64>,
    pub day_of_year: PyReadonlyArray1<'py, usize>,
}

impl PyData<'_> {
    pub fn into_data(self) -> Result<Data, Error> {
        Data::new(
            self.precipitation.as_array().to_owned(),
            self.temperature.as_array().to_owned(),
            self.pet.as_array().to_owned(),
            self.day_of_year.as_array().to_owned(),
        )
    }
}

#[derive(FromPyObject)]
pub struct PyMetadata<'py> {
    pub elevation_layers: PyReadonlyArray1<'py, f64>,
    pub median_elevation: f64,
}

impl PyMetadata<'_> {
    pub fn into_metadata(self) -> Metadata {
        Metadata {
            elevation_layers: self.elevation_layers.as_array().to_owned(),
            median_elevation: self.median_elevation,
        }
    }
}
