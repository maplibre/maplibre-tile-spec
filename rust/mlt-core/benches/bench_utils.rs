#![allow(dead_code)]

use std::fs;
use std::hint::black_box;
use std::io::{Read as _, Write as _};
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
    load_all_tiles("", ".mvt")
}

/// Load every `extension` file under `../../test/<test_subpath>`, sorted by path.
/// In debug builds (CI), returns only the first file to keep tests fast.
#[must_use]
pub fn load_all_tiles(test_subpath: &str, extension: &str) -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test")
        .join(test_subpath);
    let mut tiles = Vec::new();
    walk_dir(&dir, extension, &mut tiles);
    assert!(
        !tiles.is_empty(),
        "No {extension} files found under {}",
        dir.display()
    );
    tiles.sort_by(|a, b| a.0.cmp(&b.0));
    #[cfg(debug_assertions)]
    tiles.truncate(1);
    tiles
}

#[must_use]
pub fn load_mlt_tiles(zoom: u8) -> Vec<(String, Vec<u8>)> {
    load_tiles(zoom, "expected/0x01/omt", ".mlt")
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

#[must_use]
pub fn total_bytes(tiles: &[(String, Vec<u8>)]) -> usize {
    tiles.iter().map(|(_, d)| d.len()).sum()
}

#[must_use]
pub fn compress_gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("gzip compress failed");
    encoder.finish().expect("gzip finish failed")
}

#[must_use]
pub fn decompress_gzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .expect("gzip decompress failed");
    out
}

/// Decode every feature's properties and geometry, the work an MVT reader does for a renderer.
pub fn mvt_decode(data: &[u8]) {
    let reader =
        fast_mvt::MvtReaderRef::new(black_box(data)).expect("mvt reader construction failed");
    for layer in reader.layers() {
        for feature in layer.features() {
            let _ = black_box(feature.properties_vec().expect("mvt properties failed"));
            let _ = black_box(feature.geometry().expect("mvt geometry failed"));
        }
    }
    let _ = black_box(reader);
}
