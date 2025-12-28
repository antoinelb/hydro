mod pet;
mod snow;

use pyo3::prelude::*;

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_submodule(&pet::make_module(m.py())?)?;
    m.add_submodule(&snow::make_module(m.py())?)?;
    Ok(())
}
