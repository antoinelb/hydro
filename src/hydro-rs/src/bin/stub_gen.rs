use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = hydro_rs::stub_info()?;
    stub.generate()?;
    Ok(())
}
