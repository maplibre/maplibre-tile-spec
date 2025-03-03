
use criterion::{criterion_group, criterion_main, Criterion};

use std::fs;
use std::time::Duration;
use std::cell::RefCell;
use std::io::Cursor;

use fastpfor_rs::FastPFOR;
use lazy_static::lazy_static;

extern crate varint;
use varint::VarintRead;

lazy_static! {
    static ref ASSETS_LARGE_RAW: Vec<u32> = {
        let data = fs::read_to_string("./assets/large_raw.txt")
            .expect("Error: can not read from assets (large_raw.txt)");
        data.split(",")
            .map(|i| { if i.parse::<i32>().is_err() { eprintln!("{}", i);} i.parse::<i32>().unwrap() as u32 })
            .collect()
    };
    static ref ASSETS_LARGE_ENCODED: Vec<u32> = {
        let data = fs::read_to_string("./assets/large_fastpfor.txt")
            .expect("Error: can not read from assets (large_fastpfor.txt)");
        data.split(",")
            .map(|i| { if i.parse::<i32>().is_err() { eprintln!("{}", i);} i.parse::<i32>().unwrap() as u32 })
            .collect()
    };
    static ref ASSETS_LARGE_VARINT: Vec<u8> = {
        let data = fs::read_to_string("./assets/large_varint.txt")
            .expect("Error: can not read from assets (large_varint.txt)");
        data.split(",")
            .map(|i| { if i.parse::<i32>().is_err() { eprintln!("{}", i);} i.parse::<i32>().unwrap() as u8 })
            .collect()
    };
}

thread_local! {
    pub static CORE: RefCell<FastPFOR> = RefCell::new(FastPFOR::default());
    pub static OUTPUT_FASTPFOR: RefCell<Vec<u32>> = RefCell::new(vec![0; ASSETS_LARGE_RAW.len()]);
    pub static OUTPUT_VARINT: RefCell<Vec<u32>> = RefCell::new(vec![0; ASSETS_LARGE_RAW.len()]);
    pub static ASSETS_LARGE_VARINT_CURSOR: RefCell<Cursor<Vec<u8>>> = RefCell::new(Cursor::new(ASSETS_LARGE_VARINT.clone()));
}



fn test_fastpfor() {
    CORE.with_borrow_mut(|core| {
        OUTPUT_FASTPFOR.with_borrow_mut(|buffer| {
            core.decode(&ASSETS_LARGE_ENCODED, buffer);
        });
    });
}
fn test_varint() {
    OUTPUT_VARINT.with_borrow_mut(|buffer| {
        ASSETS_LARGE_VARINT_CURSOR.with_borrow_mut(|cursor| {
            let mut i = 0;
            loop {

                match cursor.read_unsigned_varint_32() {
                    Ok(num) => { buffer[i] = num; },
                    Err(_) => { break; }
                }
                i += 1;
            }
        });
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("FastPFOR decode", |b| b.iter(|| test_fastpfor()));
    c.bench_function("VarInt decode", |b| b.iter(|| test_varint()));
}


criterion_group!{
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(900));
    targets = criterion_benchmark
}
criterion_main!(benches);
