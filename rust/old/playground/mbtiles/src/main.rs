mod tile_repository;
mod defs;

use std::io::{Bytes, Read};
use std::fs;
use flate2::read::GzDecoder;
use geozero::mvt::Message;
use geozero::mvt::tile::{Feature, GeomType};
use reqwest::blocking;
use crate::tile_repository::MBTilesTileRepository;


pub fn request_tile(repo: MBTilesTileRepository) {
    // let body = blocking::get("http://lx003360.rsint.net:8080/api/data/osm/5/16/10.pbf")
    // // let body = blocking::get("http://lx003360.rsint.net:8080/api/data/contours/9/274/179.pbf")
    //     .unwrap()
    //     .bytes()
    //     .unwrap();
    
    let tile = repo.get_tile(20, 20, 5)
        .expect("Could not fetch Tile");

    let mut uncompressed = GzDecoder::<&[u8]>::new(tile.tile_data.as_slice());
    let mut tile_data: Vec<u8>  = vec![];

    uncompressed.read_to_end(tile_data.as_mut()).expect("Could not decode GZip");

    let tile = geozero::mvt::Tile::decode(tile_data.as_ref())
        .expect("Could not decode MVT Tile");
    
    let mut feature_count = 0;

    for layer in tile.layers {
        feature_count += layer.features.len();
    }
    
    println!("Feature Count: {}", feature_count);


    // fs::write(format!("F:\\Maps\\tiles\\x{}y{}z{}.mvt", tile.tile_row, tile.tile_column, tile.zoom_level), tile_data).unwrap();
}

// All Keys: [
//     "admin_level",
//     "disputed",
//     "maritime",
//     "subclass",
//     "class",
//     "name_int",
//     "name_de",
//     "name",
//     "rank",
//     "name:latin",
//     "name_en",
//     "iso_a2",
//     "name:nonlatin",
//     "id",
//     "intermittent",
// ]


fn main() {
    let repo = MBTilesTileRepository::from_file("F:\\Maps\\czech-republic.mbtiles")
        .expect("Could not open file");

    request_tile(repo);
}
