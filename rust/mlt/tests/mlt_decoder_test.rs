mod common;

use mlt::MltResult;
use std::path::{Path, PathBuf};
use std::fs;

#[test]
fn test_mlt_tiles() -> MltResult<()> {
    let mlt_path = Path::new("../../test/expected/omt/");
    let all_files = fs::read_dir(mlt_path)?;
    
    // Collect all mlt files
    let mlt_files: Vec<PathBuf> = all_files
        .filter_map(Result::ok)
        .map(|f| f.path())
        .filter(|f| f.extension().map_or(false, |ext| ext == "mlt"))
        .collect();

    for mlt_file in mlt_files {
        // Currently, only grab 2_2_2.mlt as it has matching metadata 2_2_2.mlt.meta.pbf
        if mlt_file.file_name().unwrap() != "2_2_2.mlt" {
            continue;
        }
        println!("Processing: {:?}", mlt_file);
    }

    Ok(())
}

