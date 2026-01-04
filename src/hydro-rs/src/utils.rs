use pyo3::prelude::*;

/// Register a submodule in sys.modules so it can be imported.
///
/// `parent_path` is the full module path of the parent (e.g., "hydro_rs" or "hydro_rs.climate").
pub fn register_submodule(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    child: &Bound<'_, PyModule>,
    parent_path: &str,
) -> PyResult<()> {
    parent.add_submodule(child)?;
    let child_name = child.name()?;
    let full_name = format!("{}.{}", parent_path, child_name);
    py.import("sys")?
        .getattr("modules")?
        .set_item(full_name, child)?;
    Ok(())
}
