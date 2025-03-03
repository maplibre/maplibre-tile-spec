
use std::fs;
use std::io::Cursor;
use lazy_static::lazy_static;

extern crate varint;
use varint::VarintWrite;

lazy_static! {
    static ref ASSETS_LARGE_RAW: Vec<u32> = {
        let data = fs::read_to_string("./assets/large_raw.txt")
            .expect("Error: can not read from assets (large_raw.txt)");
        data.split(",")
            .map(|i| { if i.parse::<i32>().is_err() {eprintln!("{}", i);} i.parse::<i32>().unwrap() as u32 })
            .collect()
    };
}

fn main() {
    fs::remove_file("./assets/large_varint.txt").unwrap();

    let mut vector = Cursor::new(vec![0u8; ASSETS_LARGE_RAW.len()]);

    for i in ASSETS_LARGE_RAW.iter() {
        vector.write_unsigned_varint_32(*i).expect("Could not encode varint");
    }

    let mut result = String::with_capacity(ASSETS_LARGE_RAW.len());

    for i in vector.get_ref() {
        result.push_str(i.to_string().as_str());
        result.push_str(",");
    }

    assert_eq!(result.pop(), Some(','));

    let _ = fs::write("./assets/large_varint.txt", result);
}
