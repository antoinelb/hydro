mod pet;

use pyo3::prelude::*;

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_submodule(&pet::make_module(m.py())?)?;
    Ok(())
}
