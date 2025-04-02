mod common;

use mlt::MltResult;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[test]
fn test_mlt_tiles() -> MltResult<()> {
    let mlt_path = Path::new("../../test/expected/omt/");
    let all_files = fs::read_dir(mlt_path)?;

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
        test_tile(&mlt_file)?;
    }

    Ok(())
}

fn test_tile(file: &Path) -> MltResult<()> {
    let mut file = fs::File::open(file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(())
}
