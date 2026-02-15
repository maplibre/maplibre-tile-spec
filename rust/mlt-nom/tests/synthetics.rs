use std::fs;
use std::path::Path;

use mlt_nom::geojson::FeatureCollection;
use mlt_nom::parse_layers;
use serde_json::Value;
use test_each_file::test_each_path;

test_each_path! { for ["mlt", "json5"] in "../test/synthetic/0x01" => pair_test }

fn pair_test([mlt, json5]: [&Path; 2]) {
    test_one(mlt, json5);
}

fn test_one(mlt: &Path, json5: &Path) {
    let buffer = fs::read(mlt).unwrap();
    let mut data = parse_layers(&buffer).unwrap();
    for layer in &mut data {
        layer.decode_all().unwrap();
    }

    let expected: FeatureCollection = json5::from_str(&fs::read_to_string(json5).unwrap()).unwrap();
    let actual = FeatureCollection::from_layers(&data).unwrap();

    // Normalize very small floats (near 0) to handle precision issues due to serializing to JSON and back
    //
    // Rust (serde_json::Number) stores floats internally as f64.
    // This means that f32 will get parsed as f64 widening its precision
    // We counter-fudge values very very small to compensate
    //
    //  There is no good way to handle this since JSON does not give us any information if we are reading an f64 or f32
    let actual_json = normalize_tiny_floats(&serde_json::to_value(&actual).unwrap());
    let expected_json = normalize_tiny_floats(&serde_json::to_value(&expected).unwrap());

    pretty_assertions::assert_eq!(
        serde_json::to_string_pretty(&actual_json).unwrap(),
        serde_json::to_string_pretty(&expected_json).unwrap()
    );
}

/// Replace extremely small float values (< 1e-40) with 0.0 to handle codec precision issues
fn normalize_tiny_floats(value: &Value) -> Value {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.abs() < 1e-40 && f != 0.0 {
                    Value::from(0.0)
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_tiny_floats).collect()),
        Value::Object(obj) => Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), normalize_tiny_floats(v)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[test]
fn test_plain() {
    let path = "../../test/synthetic/0x01/point";
    let mlt = Path::new(path).with_extension("mlt");
    let json = Path::new(path).with_extension("json5");
    test_one(&mlt, &json);
}
