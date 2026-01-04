mod calibration;
mod climate;
mod metrics;
mod model;
mod pet;
mod snow;
mod utils;

use pyo3::prelude::*;
use utils::register_submodule;

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    register_submodule(py, m, &calibration::make_module(py)?, "hydro_rs")?;
    register_submodule(py, m, &climate::make_module(py)?, "hydro_rs")?;
    register_submodule(py, m, &pet::make_module(py)?, "hydro_rs")?;
    register_submodule(py, m, &snow::make_module(py)?, "hydro_rs")?;
    register_submodule(py, m, &metrics::make_module(py)?, "hydro_rs")?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
