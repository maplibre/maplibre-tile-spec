use std::ffi::OsString;
use std::fs;
use std::path::Path;

use insta::{assert_debug_snapshot, with_settings};
use mlt_nom::parse_layers;
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/expected/tag0x01" as parse => parse }

fn parse([path]: [&Path; 1]) {
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

/// Parse a single MLT file and assert a snapshot of the result.
fn parse_one_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    eprintln!("Parsing MLT file: {}", path.display());
    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let decode_name = format!("{file_name}___decoded");
    let buffer = fs::read(path).unwrap();
    match parse_layers(&buffer) {
        Ok(mut layers) => {
            assert_debug_snapshot!(file_name.as_str(), layers);
            for layer in &mut layers {
                match layer.decode_all() {
                    Ok(v) => assert_debug_snapshot!(decode_name.as_str(), v),
                    Err(e) => assert_debug_snapshot!(format!("{file_name}___bad-decode"), e),
                }
            }
        }
        Err(e) => {
            let filesize = buffer.len();
            assert_debug_snapshot!(format!("{file_name}___bad___{filesize}"), e);
        }
    }
}

// test_each_path! { for ["mlt"] in "../test/expected/tag0x01" as decode => decode }
// fn decode([path]: [&Path; 1]) {
//     let buffer = fs::read(path).unwrap();
//     let mut layers = parse_layers(&buffer).expect("MLT file parse");
//     for layer in &mut layers {
//         match layer.decode_all() {
//             Ok(v) => {
//                 assert_debug_snapshot!(path.as_str(), v);
//             }
//             Err(e) => {
//                 // assert_debug_snapshot!(format!("{file_name}___bad-decode"), _e);
//                 todo!("handle decode error: {e:#?}");
//             }
//         }
//     }
// }


#[test]
#[ignore = "used for manual testing of a single file"]
fn test_plain() {
    // let path = "../../test/expected/tag0x01/simple/line-boolean.mlt";
    let path = "../../test/expected/tag0x01/omt/11_1062_1368.mlt";
    // let path = "../../test/expected/tag0x01/omt/11_1062_1368.mlt";
    // let path = "../../test/expected/tag0x01/bing/6-32-21.mlt";

    let path = Path::new(path);
    let buffer = fs::read(&path).unwrap();
    parse_layers(&buffer).unwrap();
    // decode([&path]);
}
