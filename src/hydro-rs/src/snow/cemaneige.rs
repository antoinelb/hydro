use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[gen_stub_pyfunction(module = "hydro_rs.snow.cemaneige")]
#[pyfunction]
fn simulate<'py>(
    py: Python<'py>,
    precipitation: PyReadonlyArray1<'py, f64>,
    temperature: PyReadonlyArray1<'py, f64>,
    day_of_year: PyReadonlyArray1<'py, f64>,
    latitude: f64,
) -> Bound<'py, PyArray1<f64>> {
    let _precipitation = precipitation.as_slice().unwrap();

    let n_timesteps: usize = _precipitation.len();

    let mut effective_precipitation: Vec<f64> = vec![];

    for t in 0..n_timesteps {
        effective_precipitation.push(_precipitation[t]);
    }

    PyArray1::from_vec(py, effective_precipitation)
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "cemaneige")?;
    m.add_function(wrap_pyfunction!(simulate, &m)?)?;
    Ok(m)
}
