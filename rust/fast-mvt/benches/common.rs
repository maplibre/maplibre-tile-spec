use std::fs;
use std::path::{Path, PathBuf};

#[must_use]
pub fn load_repo_mvt_files() -> Vec<Vec<u8>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures");
    let mut paths = Vec::new();
    collect_mvt_paths(&dir, &mut paths);
    paths.sort();
    #[cfg(debug_assertions)]
    paths.truncate(8);
    assert!(
        !paths.is_empty(),
        "no .mvt fixtures found in {}",
        dir.display()
    );
    paths
        .into_iter()
        .map(|path| {
            fs::read(&path).unwrap_or_else(|err| panic!("can't read {}: {err}", path.display()))
        })
        .collect()
}

fn collect_mvt_paths(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("can't read {}: {err}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("can't read entry in {}: {err}", dir.display()))
            .path();
        if path.is_dir() {
            collect_mvt_paths(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "mvt") {
            out.push(path);
        }
    }
}
