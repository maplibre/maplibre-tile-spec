#![cfg(not(feature = "hotpath"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mlt-ls-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/synthetic/0x01")
        .join(name)
}

fn tree() -> TempDir {
    let dir = TempDir::new();
    let nested = dir.0.join("nested");
    fs::create_dir_all(&nested).unwrap();
    for name in ["line.mlt", "line.json", "point.mlt"] {
        fs::copy(fixture(name), dir.0.join(name)).unwrap();
    }
    fs::copy(fixture("point.mlt"), nested.join("deep.mlt")).unwrap();
    fs::write(dir.0.join("broken.mlt"), b"not a tile").unwrap();
    dir
}

fn ls(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("ls")
        .args(args)
        .current_dir(&dir.0)
        .output()
        .unwrap()
}

fn listed(dir: &TempDir, args: &[&str]) -> String {
    let output = ls(dir, &[args, &["--format", "json"]].concat());
    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut paths: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap())
        .collect();
    paths.sort_unstable();
    paths.join("\n")
}

#[test]
fn a_directory_is_listed_recursively() {
    insta::assert_snapshot!(listed(&tree(), &["."]), @"
    ./broken.mlt
    ./line.mlt
    ./nested/deep.mlt
    ./point.mlt
    ");
}

#[test]
fn no_recursive_stays_in_the_top_directory() {
    insta::assert_snapshot!(listed(&tree(), &[".", "--no-recursive"]), @"
    ./broken.mlt
    ./line.mlt
    ./point.mlt
    ");
}

#[test]
fn an_excluded_glob_is_skipped() {
    insta::assert_snapshot!(listed(&tree(), &[".", "-E", "**/broken.mlt", "-E", "**/nested"]), @"
    ./line.mlt
    ./point.mlt
    ");
}

#[test]
fn a_glob_argument_is_expanded() {
    insta::assert_snapshot!(listed(&tree(), &["*.mlt"]), @"
    broken.mlt
    line.mlt
    point.mlt
    ");
}

#[test]
fn an_extension_filter_replaces_the_default_extensions() {
    insta::assert_snapshot!(listed(&tree(), &[".", "-e", "json"]), @"./line.json");
}

#[test]
fn a_file_argument_is_listed_directly() {
    insta::assert_snapshot!(listed(&tree(), &["point.mlt"]), @"point.mlt");
}

#[test]
fn a_broken_tile_fails_the_run() {
    let output = ls(&tree(), &["."]);
    assert!(!output.status.success());
}

#[test]
fn a_run_with_no_tile_files_fails() {
    let dir = TempDir::new();
    let output = ls(&dir, &["."]);
    assert!(!output.status.success());
    insta::assert_snapshot!(String::from_utf8(output.stderr).unwrap(), @"No tile files found");
}

#[test]
fn a_tile_matching_its_json_is_left_out_of_the_validation_table() {
    let output = ls(&tree(), &["line.mlt", "--validate-to-json"]);
    assert!(output.status.success());
    insta::assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
     File | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | JSON
    ------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+------
    ");
}

#[test]
fn a_tile_with_no_json_beside_it_fails_validation() {
    let output = ls(&tree(), &["point.mlt", "--validate-to-json"]);
    assert!(!output.status.success());
    insta::assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | JSON
    -----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             | ✗
    ");
}

#[test]
fn all_details_adds_the_algorithms_column() {
    let output = ls(&tree(), &["point.mlt", "--details", "all"]);
    insta::assert_snapshot!(String::from_utf8(output.stdout).unwrap(), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | Algorithms
    -----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+-------------------------------------------------------------------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             | data[vertex]/varint/componentwise-delta,length[var-binary]/varint
    ");
}
