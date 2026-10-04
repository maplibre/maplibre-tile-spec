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

fn synthetic(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/synthetic/0x01")
        .join(name)
}

fn mlt_ls(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("ls")
        .args(args)
        .current_dir(&dir.0)
        .output()
        .unwrap()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn a_directory_argument_lists_the_tiles_in_its_subdirectories() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("nested")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("top.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("nested/deep.mlt")).unwrap();

    let output = mlt_ls(&dir, &["."]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File            | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    -----------------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     nested/deep.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             
     top.mlt         |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             
    -----------------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     TOTAL           |  48B |  -26% |     18B |  20B |   111% |     88B | -83% |     2 |       2 |      4 |
    ");
}

#[test]
fn no_recursive_leaves_the_subdirectories_out() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("nested")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("top.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("nested/deep.mlt")).unwrap();

    let output = mlt_ls(&dir, &[".", "--no-recursive"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File    | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    ---------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     top.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt
    ");
}

#[test]
fn an_excluded_file_and_an_excluded_directory_are_both_skipped() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("nested")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("kept.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("skipped.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("nested/deep.mlt")).unwrap();

    let output = mlt_ls(&dir, &[".", "-E", "**/skipped.mlt", "-E", "**/nested"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File     | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    ----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     kept.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt
    ");
}

#[test]
fn a_glob_argument_is_expanded_to_the_files_it_matches() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("a.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("b.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("c.pbf")).unwrap();

    let output = mlt_ls(&dir, &["*.mlt"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File  | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    -------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     a.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             
     b.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             
    -------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     TOTAL |  48B |  -26% |     18B |  20B |   111% |     88B | -83% |     2 |       2 |      4 |
    ");
}

#[test]
fn an_extension_filter_leaves_out_the_other_default_extensions() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("tile.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("tile.pbf")).unwrap();

    let output = mlt_ls(&dir, &[".", "-e", "mlt"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File     | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    ----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     tile.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt
    ");
}

#[test]
fn a_file_argument_lists_just_that_file() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("wanted.mlt")).unwrap();
    fs::copy(synthetic("point.mlt"), dir.0.join("other.mlt")).unwrap();

    let output = mlt_ls(&dir, &["wanted.mlt"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File       | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    ------------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     wanted.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt
    ");
}

#[test]
fn a_tile_that_does_not_parse_is_reported_and_fails_the_run() {
    let dir = TempDir::new();
    fs::write(dir.0.join("broken.mlt"), b"not a tile").unwrap();

    let output = mlt_ls(&dir, &["broken.mlt"]);

    assert!(!output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File       | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types 
    ------------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------
     broken.mlt | 10B
    ");
}

#[test]
fn a_directory_without_tiles_prints_an_error_and_fails_the_run() {
    let dir = TempDir::new();

    let output = mlt_ls(&dir, &["."]);

    assert!(!output.status.success());
    insta::assert_snapshot!(stderr_of(&output), @"No tile files found");
}

#[test]
fn json_output_lists_the_streams_of_a_tile() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("point.mlt")).unwrap();

    let output = mlt_ls(&dir, &["point.mlt", "--format", "json"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @r#"
    [
      {
        "path": "point.mlt",
        "info": {
          "path": "point.mlt",
          "size": 24,
          "encoding_pct": -26.315789473684205,
          "data_size": 9,
          "meta_size": 10,
          "meta_pct": 111.11111111111111,
          "gzipped_size": 44,
          "gzip_pct": -83.33333333333333,
          "layers": 1,
          "features": 1,
          "content": [],
          "streams": 2,
          "algorithms": [
            {
              "stream": "data[vertex]",
              "physical": "varint",
              "logical": "componentwise-delta"
            },
            {
              "stream": "length[var-binary]",
              "physical": "varint",
              "logical": null
            }
          ],
          "geometries": [
            "Point"
          ],
          "matches_json": null,
          "facets": {
            "extent": [
              "64"
            ],
            "geometry": [
              "point"
            ],
            "geomLayout": [],
            "zStep": [],
            "dataType": [],
            "mValue": [],
            "strLayout": [],
            "dictLayout": [],
            "streamType": [
              "data[vertex]",
              "length[var-binary]"
            ],
            "physical": [
              "varint"
            ],
            "logical": [
              "componentwise-delta"
            ]
          }
        }
      }
    ]
    "#);
}

#[test]
fn a_tile_matching_the_json_beside_it_is_left_out_of_the_validation_table() {
    let dir = TempDir::new();
    fs::copy(synthetic("line.mlt"), dir.0.join("line.mlt")).unwrap();
    fs::copy(synthetic("line.json"), dir.0.join("line.json")).unwrap();

    let output = mlt_ls(&dir, &["line.mlt", "--validate-to-json"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | JSON 
    ------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+------
    ");
}

#[test]
fn a_tile_with_no_json_beside_it_fails_validation() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("point.mlt")).unwrap();

    let output = mlt_ls(&dir, &["point.mlt", "--validate-to-json"]);

    assert!(!output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | JSON 
    -----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             | ✗
    ");
}

#[test]
fn a_tile_whose_json_differs_fails_validation() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("point.mlt")).unwrap();
    fs::copy(synthetic("line.json"), dir.0.join("point.json")).unwrap();

    let output = mlt_ls(&dir, &["point.mlt", "--validate-to-json"]);

    assert!(!output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | JSON 
    -----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             | ✗
    ");
}

#[test]
fn details_all_adds_the_algorithms_column() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("point.mlt")).unwrap();

    let output = mlt_ls(&dir, &["point.mlt", "--details", "all"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Gzipped | Gz % | Layer | Feature | Stream | Geometry Types | Algorithms                                                        
    -----------+------+-------+---------+------+--------+---------+------+-------+---------+--------+----------------+-------------------------------------------------------------------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     44B | -83% |     1 |       1 |      2 | Pt             | data[vertex]/varint/componentwise-delta,length[var-binary]/varint
    ");
}

#[test]
fn details_basic_drops_the_gzip_columns() {
    let dir = TempDir::new();
    fs::copy(synthetic("point.mlt"), dir.0.join("point.mlt")).unwrap();

    let output = mlt_ls(&dir, &["point.mlt", "--details", "basic"]);

    assert!(output.status.success());
    insta::assert_snapshot!(stdout_of(&output), @"
     File      | Size | Enc % | Decoded | Meta | Meta % | Layer | Feature | Stream | Geometry Types 
    -----------+------+-------+---------+------+--------+-------+---------+--------+----------------
     point.mlt |  24B |  -26% |      9B |  10B |   111% |     1 |       1 |      2 | Pt
    ");
}
