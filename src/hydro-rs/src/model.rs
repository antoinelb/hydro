#![allow(clippy::type_complexity)]

use crate::metrics::MetricsError;
use ndarray::{s, Array1, Array2, ArrayView1, Axis};
use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
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

#[derive(Clone, Copy)]
pub struct Data<'a> {
    pub precipitation: ArrayView1<'a, f64>, // mm/day
    pub temperature: ArrayView1<'a, f64>,   // °C
    pub pet: ArrayView1<'a, f64>,           // mm/day
    pub day_of_year: ArrayView1<'a, usize>, // 1-365
}

impl<'a> Data<'a> {
    pub fn new(
        precipitation: ArrayView1<'a, f64>,
        temperature: ArrayView1<'a, f64>,
        pet: ArrayView1<'a, f64>,
        day_of_year: ArrayView1<'a, usize>,
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
            temperature,
            pet,
            day_of_year,
        })
    }
}

pub struct Metadata<'a> {
    pub area: f64,                             // km^2
    pub elevation_layers: ArrayView1<'a, f64>, // m
    pub median_elevation: f64,                 // m
}

pub type SimulateFn = Box<
    dyn for<'a, 'b, 'c> Fn(
            ArrayView1<'a, f64>,
            Data<'b>,
            &Metadata<'c>,
        ) -> Result<Array1<f64>, Error>
        + Send
        + Sync,
>;

pub fn compose_init(
    snow_init: fn() -> (Array1<f64>, Array2<f64>),
    climate_init: fn() -> (Array1<f64>, Array2<f64>),
) -> impl Fn() -> (Array1<f64>, Array2<f64>, usize) {
    move || {
        let (snow_defaults, snow_bounds) = snow_init();
        let (climate_defaults, climate_bounds) = climate_init();
        let default_values = ndarray::concatenate(
            Axis(0),
            &[snow_defaults.view(), climate_defaults.view()],
        )
        .unwrap();
        let bounds = ndarray::concatenate(
            Axis(0),
            &[snow_bounds.view(), climate_bounds.view()],
        )
        .unwrap();

        (default_values, bounds, snow_defaults.len())
    }
}

pub type SimulateFnPtr = for<'a, 'b, 'c> fn(
    ArrayView1<'a, f64>,
    Data<'b>,
    &Metadata<'c>,
) -> Result<Array1<f64>, Error>;

pub fn compose_simulate(
    snow_simulate: SimulateFnPtr,
    climate_simulate: SimulateFnPtr,
    n_snow_params: usize,
) -> SimulateFn {
    Box::new(move |params, data, metadata| {
        let snow_params = params.slice(s![..n_snow_params]);
        let climate_params = params.slice(s![n_snow_params..]);

        let effective_precipitation =
            snow_simulate(snow_params, data, metadata)?;

        let climate_data = Data {
            precipitation: effective_precipitation.view(),
            temperature: data.temperature,
            pet: data.pet,
            day_of_year: data.day_of_year,
        };

        climate_simulate(climate_params, climate_data, metadata)
    })
}

#[derive(FromPyObject)]
pub struct PyData<'py> {
    pub precipitation: PyReadonlyArray1<'py, f64>,
    pub temperature: PyReadonlyArray1<'py, f64>,
    pub pet: PyReadonlyArray1<'py, f64>,
    pub day_of_year: PyReadonlyArray1<'py, usize>,
}

impl<'py> PyData<'py> {
    pub fn as_data(&self) -> Result<Data<'_>, Error> {
        Data::new(
            self.precipitation.as_array(),
            self.temperature.as_array(),
            self.pet.as_array(),
            self.day_of_year.as_array(),
        )
    }
}

#[derive(FromPyObject)]
pub struct PyMetadata<'py> {
    pub area: f64,
    pub elevation_layers: PyReadonlyArray1<'py, f64>,
    pub median_elevation: f64,
}

impl<'py> PyMetadata<'py> {
    pub fn as_metadata(&self) -> Metadata<'_> {
        Metadata {
            area: self.area,
            elevation_layers: self.elevation_layers.as_array(),
            median_elevation: self.median_elevation,
        }
    }
}
