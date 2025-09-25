#![allow(unused_mut)]

mod structures;
mod utils;

use std::io::Cursor;
use std::path::Path;

use galileo_mvt::MvtTile;

use crate::structures::Layer;
use crate::structures::v1::{OwnedFeatureMetaTable, OwnedFeatureTable};

#[cfg(test)]
mod tests;

fn simple_test() {
    println!("\n=== Layer Stream Parsing Demo ===");
    let test_data = create_test_data();
    println!("Parsing test data of size: {}", test_data.len());

    match structures::parse_binary_stream(&test_data) {
        Ok((remaining, layers)) => {
            println!("Successfully parsed {} layers:", layers.len());
            for (i, layer) in layers.iter().enumerate() {
                match layer {
                    Layer::Layer(layer_v1) => {
                        println!("  Layer {i}: Layer");
                        println!("    Name: {}", layer_v1.meta.name);
                        println!("    Extent: {}", layer_v1.meta.extent);
                        for (j, column) in layer_v1.meta.columns.iter().enumerate() {
                            match column.name {
                                Some(name) => {
                                    println!(
                                        "    Column {j}: name='{name}', type={:?}",
                                        column.typ
                                    );
                                }
                                None => println!("    Column {j}: type={:?}", column.typ),
                            }
                        }
                        println!("    Data: {:?}", layer_v1.data);
                    }
                    Layer::Unknown(unknown) => {
                        println!(
                            "  Layer {i}: Unknown(tag={}), value={:?}",
                            unknown.tag, unknown.value
                        );
                    }
                }
            }
            if !remaining.is_empty() {
                println!("Warning: {} bytes remaining unparsed", remaining.len());
            }
        }
        Err(e) => println!("Parse error: {e:?}"),
    }
}

fn generate() {
    // walk using walkdir through all *.mvt files in ../test/fixtures
    for path in walkdir::WalkDir::new("../test/fixtures")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().unwrap_or_default() == "mvt")
    {
        generate_tile(path.path());
    }
}

fn generate_tile(path: &Path) {
    let data = std::fs::read(path).expect("Failed to read MVT file");
    println!(
        "Processing MVT file: {} ({} bytes)",
        path.display(),
        data.len()
    );

    let tile = MvtTile::decode(&mut Cursor::new(&data), false).unwrap();

    let mut mlt = Vec::with_capacity(tile.layers.len());
    for layer in &tile.layers {
        let mut columns = Vec::new();
        let mut data = Vec::new();
        for _feature in &layer.features {
            // let geometry = &feature.geometry;
            // let bytes =
            //     bincode::serde::encode_to_vec(geometry, bincode::config::standard()).unwrap();
            // let (deserialized, _): (MvtGeometry, _) =
            //     bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();

            // assert_eq!(&deserialized, geometry);
        }
        mlt.push(OwnedFeatureTable {
            meta: OwnedFeatureMetaTable {
                name: layer.name.clone(),
                extent: layer.size,
                columns,
            },
            data,
        });
    }
}

fn main() {
    simple_test();
    generate();
}

/// Helper function to create test data
fn create_test_data() -> Vec<u8> {
    let mut data = Vec::new();

    // Tuple 1: tag=1 (Layer), create metadata + data
    let mut layer = Vec::new();

    // Layer name: "roads" (length=5, then "roads")
    utils::encode_str(&mut layer, b"roads");

    // Extent: 4096
    utils::encode_varint(&mut layer, 4096);

    // Column count: 6
    utils::encode_varint(&mut layer, 6);

    // Column 1: type=1 (ID) - no name for ID columns
    utils::encode_varint(&mut layer, 1); // column_type (ID)

    // Column 2: type=2 (Geometry) - no name for Geometry columns
    utils::encode_varint(&mut layer, 2); // column_type (Geometry)

    // Column 3: type=3 (StringProperty) - has name
    utils::encode_varint(&mut layer, 3); // column_type (StringProperty)
    utils::encode_str(&mut layer, b"name"); // name

    // Column 4: type=4 (FloatProperty) - has name
    utils::encode_varint(&mut layer, 4); // column_type (FloatProperty)
    utils::encode_str(&mut layer, b"price"); // name

    // Column 5: type=6 (IntProperty) - has name
    utils::encode_varint(&mut layer, 6); // column_type (IntProperty)
    utils::encode_str(&mut layer, b"count"); // name

    // Column 6: type=9 (BoolProperty) - has name
    utils::encode_varint(&mut layer, 9); // column_type (BoolProperty)
    utils::encode_str(&mut layer, b"active"); // name

    // Add some additional data after metadata
    layer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);

    utils::encode_varint(&mut data, layer.len() as u64 + 1); // size (1 for tag + data)
    utils::encode_varint(&mut data, 1); // tag
    data.extend_from_slice(&layer); // value

    // Tuple 2: tag=42, size=3 (1 for tag + 2 for value), value=[0xAA, 0xBB]
    utils::encode_varint(&mut data, 3); // size (1 for tag + 2 for value)
    utils::encode_varint(&mut data, 42); // tag
    data.extend_from_slice(&[0xAA, 0xBB]); // value

    // Tuple 3: tag=100, size=0, value=[]
    utils::encode_varint(&mut data, 1); // size (1 for tag byte)
    utils::encode_varint(&mut data, 100); // tag
    // no value bytes for size 0

    data
}
