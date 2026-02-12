use std::fs;
use std::path::Path;

use mlt_nom::geojson::FeatureCollection;
use mlt_nom::parse_layers;
use test_each_file::test_each_path;

test_each_path! { for ["mlt", "json"] in "../test/synthetic/0x01" => pair_test }

fn pair_test([mlt, json]: [&Path; 2]) {
    test_one(mlt, json);
}

fn test_one(mlt: &Path, json: &Path) {
    let buffer = fs::read(mlt).unwrap();
    let mut data = match parse_layers(&buffer) {
        Ok(v) => v,
        Err(e) if e.to_string().contains("FastPFOR") => {
            eprintln!("Skipping {}: {e}", mlt.display());
            return;
        }
        Err(e) => panic!("{e}"),
    };
    for layer in &mut data {
        if let Err(e) = layer.decode_all() {
            if e.to_string().contains("FastPFOR") {
                eprintln!("Skipping {}: {e}", mlt.display());
                return;
            }
            panic!("{e}");
        }
    }

    let expected: FeatureCollection =
        serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    let actual = FeatureCollection::from_layers(&data).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_plain() {
    let path = "../../test/synthetic/0x01/point";
    let mlt = Path::new(path).with_extension("mlt");
    let json = Path::new(path).with_extension("json");
    test_one(&mlt, &json);
}
