use std::fs;
use std::path::Path;

pub const BENCHMARKED_ZOOM_LEVELS: [u8; 3] = [4, 7, 13];

#[must_use]
pub fn load_mlt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "expected/tag0x01/omt", ".mlt")
}

#[must_use]
pub fn load_tiles(zoom: u8, test_subpath: &str, extension: &str) -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(test_subpath);
    let prefix = format!("{zoom}_");
    let mut tiles = Vec::new();
    let entries =
        fs::read_dir(&dir).unwrap_or_else(|err| panic!("can't read {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("can't read entry {}: {err}", dir.display()));
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with(&prefix)
            && let Some(stem) = name.strip_suffix(extension)
        {
            let data = fs::read(entry.path())
                .unwrap_or_else(|err| panic!("can't read {}: {err}", entry.path().display()));
            tiles.push((stem.to_string(), data));
        }
    }
    assert!(
        !tiles.is_empty(),
        "No tiles found for zoom level {zoom} in {}",
        dir.display()
    );
    tiles.sort_by(|a, b| a.0.cmp(&b.0));
    tiles
}
