use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/metadata");

    // Ensure the output directory exists
    fs::create_dir_all(&out_dir).unwrap();

    // Generate the file using prost_build
    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(
            &["../../spec/schema/mlt_tileset_metadata.proto"],
            &["../../spec/schema/"],
        )
        .unwrap();

    // Define the original and new file paths
    let original_file = out_dir.join("mlt.rs");
    let new_file = out_dir.join("proto_tileset.rs");

    // Rename the file
    if original_file.exists() {
        fs::rename(&original_file, &new_file).unwrap();
    } else {
        panic!("Generated file 'mlt.rs' not found!");
    }
}
