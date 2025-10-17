use std::ffi::OsString;
use std::fs;
use std::path::Path;

use insta::{assert_debug_snapshot, with_settings};
use mlt_nom::{Decodable, parse_binary_stream};
use test_each_file::test_each_path;

/// Parse a single MLT file and assert a snapshot of the result.
fn parse_one_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    eprintln!("Testing MLT file: {}", path.display());
    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let buffer = fs::read(path).unwrap();
    match parse_binary_stream(&buffer) {
        Ok(mut value) => {
            assert_debug_snapshot!(file_name.as_str(), value);
            for v in &mut value {
                if let Err(_e) = v.ensure_decoded() {
                    // assert_debug_snapshot!(format!("{file_name}___bad-decode"), _e);
                    return;
                }
            }
            // assert_debug_snapshot!(format!("{file_name}-decoded"), value);
        }
        Err(e) => {
            let filesize = buffer.len();
            assert_debug_snapshot!(format!("{file_name}___bad___{filesize}"), e);
        }
    }
}

test_each_path! { in "../test/expected/tag0x01" => test }

fn test(path: &Path) {
    if path.extension().unwrap_or_default() != "mlt" {
        return;
    }
    let mut snapshot_path = OsString::from("snapshots-");
    snapshot_path.push(path.parent().unwrap().file_name().unwrap());
    with_settings! {
        { omit_expression => true,
          snapshot_path => snapshot_path ,
          prepend_module_to_snapshot => false },
        {
            parse_one_file(path);
        }
    }
}

#[test]
#[ignore = "used for manual testing of a single file"]
fn test_plain() {
    // let path = "../../test/expected/tag0x01/simple/line-boolean.mlt";
    let path = "../../test/expected/tag0x01/omt/11_1062_1368.mlt";
    // let path = "../../test/expected/tag0x01/omt/11_1062_1368.mlt";
    // let path = "../../test/expected/tag0x01/bing/6-32-21.mlt";
    let buffer = fs::read(path).unwrap();
    parse_binary_stream(&buffer).unwrap();
}
