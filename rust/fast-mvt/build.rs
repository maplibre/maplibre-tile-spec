#[cfg(feature = "codegen")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/vector_tile.proto");
    println!("cargo:rerun-if-changed=src/generated/");

    buffa_build::Config::new()
        .files(&["src/vector_tile.proto"])
        .includes(&["src"])
        .out_dir("src/generated")
        .include_file("mod.rs")
        .generate_json(true)
        .generate_arbitrary(true)
        .gate_impls_on_crate_features(true)
        .preserve_unknown_fields(false)
        .compile()
}

#[cfg(not(feature = "codegen"))]
fn main() {}
