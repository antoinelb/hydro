mod pet;

use pyo3::prelude::*;

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the oudin submodule
    m.add_submodule(&pet::oudin::make_module(m.py())?)?;
    Ok(())
}
