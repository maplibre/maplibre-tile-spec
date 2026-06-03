#[cfg(feature = "codegen")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    buffa_build::Config::new()
        .files(&["src/vector_tile.proto"])
        .includes(&["src"])
        .out_dir("src/generated")
        .include_file("mod.rs")
        .generate_json(true)
        .gate_impls_on_crate_features(true)
        .compile()
}

#[cfg(not(feature = "codegen"))]
fn main() {}
