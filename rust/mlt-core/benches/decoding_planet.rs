//! Per-tile decode time of the same tiles in several encodings, in a shuffled order per tile so shared-host noise and cache state hit every encoding alike.
//!
//! Usage: `decoding_planet --repeats N --out results.json name=kind:path ...`
//! `kind` is `mvt`, `mvt-gzip`, `mlt` or `mlt-gzip`, `path` a pack file of `[z u8][len u32 LE][bytes]` records in the same tile order for every encoding.

use std::fmt::Write as _;
use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

use mlt_core::__private::{dec, parser};

#[path = "bench_utils.rs"]
mod bench_utils;
use bench_utils::{decompress_gzip, mvt_decode};

#[derive(Clone, Copy)]
enum Kind {
    Mvt,
    MvtGzip,
    Mlt,
    MltGzip,
}

struct Encoding {
    name: String,
    kind: Kind,
    tiles: Vec<Vec<u8>>,
}

fn read_pack(path: &str) -> (Vec<u8>, Vec<Vec<u8>>) {
    let bytes = fs::read(path).unwrap_or_else(|err| panic!("can't read {path}: {err}"));
    let mut zooms = Vec::new();
    let mut tiles = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        zooms.push(bytes[at]);
        let len = u32::from_le_bytes(bytes[at + 1..at + 5].try_into().unwrap()) as usize;
        tiles.push(bytes[at + 5..at + 5 + len].to_vec());
        at += 5 + len;
    }
    (zooms, tiles)
}

fn decode(kind: Kind, data: &[u8]) {
    match kind {
        Kind::Mvt => mvt_decode(data),
        Kind::MvtGzip => mvt_decode(&decompress_gzip(black_box(data))),
        Kind::Mlt => decode_mlt(data),
        Kind::MltGzip => decode_mlt(&decompress_gzip(black_box(data))),
    }
}

fn decode_mlt(data: &[u8]) {
    let layers = parser()
        .parse_layers(black_box(data))
        .expect("mlt parse failed");
    let mut d = dec();
    black_box(d.decode_all(layers).expect("mlt decode_all failed"));
}

fn shuffle(items: &mut [usize], seed: u64) {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    for i in (1..items.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        items.swap(i, (state % (i as u64 + 1)) as usize);
    }
}

fn main() {
    let mut repeats = 15_usize;
    let mut out = None;
    let mut encodings = Vec::new();
    let mut zooms = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repeats" => repeats = args.next().expect("--repeats value").parse().unwrap(),
            "--out" => out = args.next(),
            "--bench" => {}
            spec => {
                let (name, rest) = spec.split_once('=').expect("name=kind:path");
                let (kind, path) = rest.split_once(':').expect("name=kind:path");
                let kind = match kind {
                    "mvt" => Kind::Mvt,
                    "mvt-gzip" => Kind::MvtGzip,
                    "mlt" => Kind::Mlt,
                    "mlt-gzip" => Kind::MltGzip,
                    other => panic!("unknown kind {other}"),
                };
                let (z, tiles) = read_pack(path);
                if let Some(first) = encodings.first() {
                    let first: &Encoding = first;
                    assert_eq!(first.tiles.len(), tiles.len(), "{name} has a different tile count");
                }
                zooms = z;
                encodings.push(Encoding { name: name.to_owned(), kind, tiles });
            }
        }
    }
    let n_tiles = encodings.first().expect("no encodings").tiles.len();
    let n_enc = encodings.len();
    let mut best = vec![vec![Duration::MAX; n_enc]; n_tiles];

    for round in 0..repeats {
        for (t, row) in best.iter_mut().enumerate() {
            let mut order: Vec<usize> = (0..n_enc).collect();
            shuffle(&mut order, (round * n_tiles + t) as u64);
            for i in order {
                let e = &encodings[i];
                let start = Instant::now();
                decode(e.kind, &e.tiles[t]);
                row[i] = row[i].min(start.elapsed());
            }
        }
        eprintln!("round {}/{repeats}", round + 1);
    }

    let mut json = String::from("{\n  \"encodings\": [");
    for (i, e) in encodings.iter().enumerate() {
        write!(json, "{}\"{}\"", if i == 0 { "" } else { ", " }, e.name).unwrap();
    }
    json.push_str("],\n  \"tiles\": [\n");
    for (t, row) in best.iter().enumerate() {
        let ns = row.iter().map(|d| d.as_nanos().to_string()).collect::<Vec<_>>().join(", ");
        let bytes = encodings.iter().map(|e| e.tiles[t].len().to_string()).collect::<Vec<_>>().join(", ");
        let comma = if t + 1 == n_tiles { "" } else { "," };
        writeln!(json, "    {{\"z\": {}, \"ns\": [{ns}], \"bytes\": [{bytes}]}}{comma}", zooms[t]).unwrap();
    }
    json.push_str("  ]\n}\n");
    match out {
        Some(path) => fs::write(path, json).expect("write results"),
        None => print!("{json}"),
    }
}
