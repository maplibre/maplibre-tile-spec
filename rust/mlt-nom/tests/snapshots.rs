use std::fs;
use std::path::Path;

use insta::{assert_debug_snapshot, with_settings};
use mlt_nom::parse_binary_stream;

/// Test parsing all MLT files
#[test]
fn test_mlt_files() {
    let test_dirs = [
        ("../../test/expected/tag0x01/simple", "simple"),
        ("../../test/expected/tag0x01/amazon", "amazon"),
        ("../../test/expected/tag0x01/amazon_here", "amazon_here"),
        ("../../test/expected/tag0x01/bing", "bing"),
        ("../../test/expected/tag0x01/omt", "omt"),
    ];
    for (path, file_name) in test_dirs {
        let path = Path::new(path);
        let snapshot_path = String::from("snapshots-") + file_name;
        // snapshots should go to /tests/snapshots-* directory
        with_settings! {
            { omit_expression => true,
              snapshot_path => snapshot_path ,
              prepend_module_to_snapshot => false },
            {
                test_dir(path);
            }
        }
    }
}

/// Test parsing all MLT files in the given directory.
fn test_dir(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    let dir_content = fs::read_dir(dir).unwrap();
    for mlt_path in dir_content {
        let path = mlt_path.unwrap().path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mlt"))
        {
            parse_one_file(path);
        }
    }
}

/// Parse a single MLT file and assert a snapshot of the result.
fn parse_one_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    eprintln!("Testing MLT file: {}", path.display());
    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let buffer = fs::read(path).unwrap();
    match parse_binary_stream(&buffer) {
        Ok(value) => {
            assert_debug_snapshot!(file_name, value);
        }
        Err(e) => {
            assert_debug_snapshot!(format!("{file_name}___bad"), e);
        }
    }
}

#[test]
#[ignore]
fn test_plain() {
    let path = "../../test/expected/tag0x01/simple/multipoint-boolean.mlt";
    let buffer = fs::read(path).unwrap();
    parse_binary_stream(&buffer).unwrap();
}
