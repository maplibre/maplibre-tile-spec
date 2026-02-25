use pyo3_stub_gen::Result;

/// purely a helper bin to generate the type stubs by CI
fn main() -> Result<()> {
    let stub = mlt_pyo3::stub_info()?;
    stub.generate()?;
    Ok(())
}
