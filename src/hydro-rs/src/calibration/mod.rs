mod sce;
mod utils;

use pyo3::prelude::*;

/// Register a submodule in sys.modules so it can be imported.
fn register_submodule(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    child: &Bound<'_, PyModule>,
) -> PyResult<()> {
    parent.add_submodule(child)?;
    let child_name = child.name()?;
    let full_name = format!("hydro_rs.calibration.{}", child_name);
    py.import("sys")?
        .getattr("modules")?
        .set_item(full_name, child)?;
    Ok(())
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "calibration")?;
    register_submodule(py, &m, &sce::make_module(py)?)?;
    Ok(m)
}
