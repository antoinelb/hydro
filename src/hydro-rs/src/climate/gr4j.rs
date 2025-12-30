use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[gen_stub_pyfunction(module = "hydro_rs.climate.gr4j")]
#[pyfunction]
fn simulate<'py>(py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
    let params: Vec<f64> = vec![];
    PyArray1::from_vec(py, params)
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "gr4j")?;
    m.add_function(wrap_pyfunction!(simulate, &m)?)?;
    Ok(m)
}
