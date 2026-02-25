use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = mlt_pyo3::stub_info()?;
    stub.generate()?;
    Ok(())
}
