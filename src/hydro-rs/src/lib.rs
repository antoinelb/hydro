mod calibration;
mod climate;
mod pet;
mod snow;
mod utils;

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

/// Register a submodule in sys.modules so it can be imported.
fn register_submodule(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    child: &Bound<'_, PyModule>,
) -> PyResult<()> {
    parent.add_submodule(child)?;
    let parent_name = parent.name()?;
    let child_name = child.name()?;
    let full_name = format!("{}.{}", parent_name, child_name);
    py.import("sys")?
        .getattr("modules")?
        .set_item(full_name, child)?;
    Ok(())
}

#[pymodule]
fn hydro_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    register_submodule(py, m, &calibration::make_module(py)?)?;
    register_submodule(py, m, &climate::make_module(py)?)?;
    register_submodule(py, m, &pet::make_module(py)?)?;
    register_submodule(py, m, &snow::make_module(py)?)?;
    register_submodule(py, m, &utils::make_module(py)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}

define_stub_info_gatherer!(stub_info);
