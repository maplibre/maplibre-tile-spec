mod common;

use mlt::MltResult;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use mlt::read_metadata;

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
        if mlt_file.file_name().unwrap() != "2_2_2.mlt" {
            continue;
        }
        test_tile(&mlt_file)?;
    }

    Ok(())
}

#[expect(unused_variables)]
fn test_tile(file: &Path) -> MltResult<()> {
    let mut mlt_file = fs::File::open(file)?;
    let mut buffer = Vec::new();
    mlt_file.read_to_end(&mut buffer)?;
    let meta = read_metadata(&file.with_extension("mlt.meta.pbf"))?;

    Ok(())
}
