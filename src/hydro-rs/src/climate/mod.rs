mod bucket;
pub mod gr4j;
mod utils;

use ndarray::{Array1, ArrayView1};
use pyo3::prelude::*;
pub use utils::ClimateError;

pub type SimulateFn =
    fn(ArrayView1<f64>, ArrayView1<f64>, ArrayView1<f64>) -> Result<Array1<f64>, ClimateError>;

pub fn get_model(name: &str) -> Result<SimulateFn, String> {
    match name.to_lowercase().as_str() {
        "gr4j" => Ok(gr4j::simulate),
        _ => Err(format!(
            "Unknown model '{}'. Valid options: gr4j",
            name
        )),
    }
}

/// Register a submodule in sys.modules so it can be imported.
fn register_submodule(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    child: &Bound<'_, PyModule>,
) -> PyResult<()> {
    parent.add_submodule(child)?;
    let child_name = child.name()?;
    let full_name = format!("hydro_rs.climate.{}", child_name);
    py.import("sys")?
        .getattr("modules")?
        .set_item(full_name, child)?;
    Ok(())
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "climate")?;
    register_submodule(py, &m, &gr4j::make_module(py)?)?;
    register_submodule(py, &m, &bucket::make_module(py)?)?;
    Ok(m)
}
