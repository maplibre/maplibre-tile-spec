use std::ffi::OsString;
use std::fs;
use std::path::Path;

use insta::{assert_debug_snapshot, with_settings};
use mlt_nom::parse_layers;
use test_each_file::test_each_path;

test_each_path! { for ["mlt", "json"] in "../test/synthetic/0x01" => test }

fn test([mlt, json]: [&str; 2]) {

}
