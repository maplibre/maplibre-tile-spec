use std::fs;
use std::path::Path;

use mlt_nom::parse_layers;
use test_each_file::test_each_path;

test_each_path! { for ["mlt", "json"] in "../test/synthetic/0x01" => pair_test }

fn pair_test([mlt, json]: [&Path; 2]) {
    test_one(mlt, json);
}

fn test_one(mlt: &Path, json: &Path) {
    let buffer = fs::read(mlt).unwrap();
    let mut data = parse_layers(&buffer).unwrap();
    for layer in &mut data {
        layer.decode_all().unwrap();
        // layer.print_json();
    }

    let expected_data: serde_json::Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();

    // implement conversion from Vec<Layer> to serde_json::Value, and compare with expected_data
    todo!();
}

#[test]
#[ignore]
fn test_plain() {
    let path = "../../test/synthetic/0x01/point";

    let mlt = Path::new(path).with_extension("mlt");
    let json = Path::new(path).with_extension("json");
    test_one(&mlt, &json);
}
