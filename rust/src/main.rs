
use std::fs;
use std::fs::read;
use quick_protobuf::{BytesReader, MessageRead};
use maplibre_tile_spec::maplibre::tile::MapLibreTile;
use maplibre_tile_spec::proto::TileSetMetadata;

extern crate maplibre_tile_spec;

fn main() {
    // unsafe { backtrace_on_stack_overflow::enable() };

    // let meta_data = read("F:\\Maps\\tiles\\x20y20z5.metadata.pbf").unwrap();
    // let data = read("F:\\Maps\\tiles\\x20y20z5.mlt").unwrap();
    let meta_data = read("E:\\Work\\maplibre-tile-spec\\7-66-44.mlt.meta.pbf").unwrap();
    let data = read("E:\\Work\\maplibre-tile-spec\\7-66-44.mlt").unwrap();
    let data = Box::new(data);

    let mut reader = BytesReader::from_bytes(meta_data.as_ref());

    let metadata = Box::new(TileSetMetadata::from_reader(&mut reader, meta_data.as_ref()).expect("Failed to read metadata"));
    // let ft = FeatureTable::load(data.as_ref());

    let mbtile = MapLibreTile::decode(&data, &metadata);

    println!("{:?}", mbtile.layers[3]);
    
    // fs::write("F:\\Maps\\tiles\\test\\sus.json", serde_json::to_string(&mbtile).expect("Failed to write mbtile to stdout")).unwrap();
}
