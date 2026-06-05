#![cfg(all(feature = "reader", feature = "json"))]

use std::fs;
use std::path::Path;

use fast_mvt::proto::Tile;
use fast_mvt::{DEFAULT_EXTENT, MvtReaderRef, MvtResult};
use serde::Deserialize;
use serde_json::Value;
use test_each_file::test_each_path;

test_each_path! { for ["mvt"] in "../test/mvt-fixtures/fixtures" => mvt_reader_fixture }

fn mvt_reader_fixture([mvt]: [&Path; 1]) {
    let info = mvt.with_file_name("info.json");
    let info_json: InfoJson = read_json(&info);
    let is_valid_v2 = info_json.validity.v2;
    let is_recoverable = info_json.validity.error.as_deref() == Some("recoverable");

    let data = read_data(mvt);
    let reader = MvtReaderRef::new(&data);
    let tile = mvt.with_file_name("tile.json");

    if is_valid_v2 || is_recoverable {
        let value = reader.expect("readable MVT file");
        if is_valid_v2 {
            let expected = read_tile_json(&tile);
            let actual = Tile::from_reader(&value);
            assert_eq!(actual, expected, "{}", tile.display());
        } else {
            assert_recoverable_fixture_is_readable(&value, mvt);
        }
    } else if let Ok(value) = reader
        && !unknown_value_field_fixture(mvt)
    {
        assert!(
            value.to_tile().is_err(),
            "{}: fatal invalid v2 fixture should fail owned tile parsing",
            mvt.display()
        );
    }
}

fn unknown_value_field_fixture(path: &Path) -> bool {
    matches!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some("011" | "026")
    )
}

fn assert_recoverable_fixture_is_readable(reader: &MvtReaderRef<'_>, path: &Path) {
    let tile = Tile::from_reader(reader);
    assert!(
        !tile.layers.is_empty(),
        "{}: recoverable fixture should decode at least one layer",
        path.display()
    );

    let traversal = traverse(reader);
    let owned = reader.to_tile();
    if let Err(error) = traversal {
        assert!(
            owned.is_err(),
            "{}: strict traversal failed ({error}) but owned parsing succeeded",
            path.display()
        );
    }
}

fn traverse(reader: &MvtReaderRef<'_>) -> MvtResult<()> {
    for layer in reader.layers() {
        for feature in layer.features() {
            let _id = feature.id();
            let _ = feature.geometry()?;
            for property in feature.properties() {
                let _ = property?;
            }
        }
    }
    Ok(())
}

fn read_data(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| panic!("{}: read failed: {e}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    serde_json::from_slice(&read_data(path))
        .unwrap_or_else(|e| panic!("{}: JSON parse failed: {e}", path.display()))
}

fn read_tile_json(path: &Path) -> Tile {
    let mut value: Value = read_json(path);
    normalize_string_values(&mut value);
    let mut tile: Tile = serde_json::from_value(value)
        .unwrap_or_else(|e| panic!("{}: tile JSON parse failed: {e}", path.display()));
    normalize_layer_defaults(&mut tile);
    tile
}

fn normalize_string_values(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(value) = map.get_mut("string_value")
                && !value.is_string()
                && !value.is_null()
            {
                *value = Value::String(value.to_string());
            }
            map.values_mut().for_each(normalize_string_values);
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_string_values),
        _ => {}
    }
}

fn normalize_layer_defaults(tile: &mut Tile) {
    for layer in &mut tile.layers {
        layer.extent.get_or_insert(DEFAULT_EXTENT.get());
    }
}

#[derive(Debug, Deserialize)]
struct InfoJson {
    validity: ValidityJson,
}

#[derive(Debug, Deserialize)]
struct ValidityJson {
    v2: bool,
    error: Option<String>,
}
