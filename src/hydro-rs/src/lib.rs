mod calibration;
mod climate;
mod pet;
mod snow;
mod utils;

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_submodule(&calibration::make_module(m.py())?)?;
    m.add_submodule(&climate::make_module(m.py())?)?;
    m.add_submodule(&pet::make_module(m.py())?)?;
    m.add_submodule(&snow::make_module(m.py())?)?;
    m.add_submodule(&utils::make_module(m.py())?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}

define_stub_info_gatherer!(stub_info);
