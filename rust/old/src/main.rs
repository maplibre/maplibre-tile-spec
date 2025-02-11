
extern crate alloc;
extern crate quick_protobuf;

mod lib;
mod headers;
mod types;
use std::fs::read;
use headers::*;



mod mlt;
use quick_protobuf::{BytesReader, Reader};
use quick_protobuf::{MessageRead, MessageWrite};
use crate::mlt::TileSetMetadata;

fn main() {
    let meta_data = read("F:\\Maps\\tiles\\x20y20z5.metadata.pbf").unwrap();
    // let data = read("F:\\Maps\\tiles\\x20y20z5.mlt").unwrap();

    let mut reader = BytesReader::from_bytes(meta_data.as_ref());

    let metadata = TileSetMetadata::from_reader(&mut reader, meta_data.as_ref()).expect("Failed to read metadata");
    // let ft = FeatureTable::load(data.as_ref());

    println!("{:#?}", metadata);
}
