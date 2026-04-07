#![allow(dead_code)]

use std::fs;
use std::path::Path;

// This code runs in CI because of --all-targets, so make it run really fast.
#[cfg(debug_assertions)]
pub const BENCHMARKED_ZOOM_LEVELS: [u8; 1] = [0];
#[cfg(not(debug_assertions))]
pub const BENCHMARKED_ZOOM_LEVELS: [u8; 3] = [4, 7, 13];

/// Recursively walk `dir` and collect all files with the given `extension`.
fn walk_dir(dir: &Path, extension: &str, out: &mut Vec<(String, Vec<u8>)>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("can't read {}: {err}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("can't read entry in {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, extension, out);
        } else {
            let name = path.to_string_lossy();
            if name.ends_with(extension) {
                let data = fs::read(&path)
                    .unwrap_or_else(|err| panic!("can't read {}: {err}", path.display()));
                out.push((path.to_string_lossy().into_owned(), data));
            }
        }
    }
}

/// Load all `.mvt` files found recursively under `../../test`.
///
/// Returns `(path_string, raw_bytes)` pairs sorted by path.
/// In debug builds (CI), returns only the first file to keep tests fast.
#[must_use]
pub fn load_all_mvt_bytes() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test");
    let mut tiles = Vec::new();
    walk_dir(&dir, ".mvt", &mut tiles);
    assert!(
        !tiles.is_empty(),
        "No .mvt files found under {}",
        dir.display()
    );
    tiles.sort_by(|a, b| a.0.cmp(&b.0));
    #[cfg(debug_assertions)]
    tiles.truncate(1);
    tiles
}

#[must_use]
pub fn load_mlt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "expected/tag0x01/omt", ".mlt")
}

/// Load MVT tiles for benchmarking, preferring the expanded dataset from
/// `just rust::prepare-benchmark-data` when it is available.
///
/// Falls back to the small OMT fixture (95 tiles) when the benchmark data
/// has not been prepared yet, so existing benchmarks keep working in CI.
#[must_use]
pub fn load_benchmark_mvt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    let bench_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/omt-bench");
    if bench_dir.exists() {
        let tiles = load_tiles_if_any(zoom, "fixtures/omt-bench", ".mvt");
        if !tiles.is_empty() {
            return tiles;
        }
    }
    load_tiles(zoom, "fixtures/omt", ".mvt")
}

/// Like [`load_tiles`] but returns an empty `Vec` instead of panicking when
/// the directory does not exist or contains no matching files.
fn load_tiles_if_any(zoom: u8, test_subpath: &str, extension: &str) -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(test_subpath);
    if !dir.exists() {
        return Vec::new();
    }
    let prefix = format!("{zoom}_");
    let mut tiles = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with(&prefix)
            && let Some(stem) = name.strip_suffix(extension)
        {
            if let Ok(data) = fs::read(entry.path()) {
                tiles.push((stem.to_string(), data));
            }
        }
    }
    tiles.sort_by(|a, b| a.0.cmp(&b.0));
    tiles
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
