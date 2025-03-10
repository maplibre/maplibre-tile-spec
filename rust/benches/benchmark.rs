use std::fs::read;
use std::path::Path;
use std::time::Duration;
use criterion::{criterion_group, criterion_main, Criterion};
use lazy_static::lazy_static;
use quick_protobuf::{BytesReader, MessageRead};
use maplibre_tile_spec::maplibre::tile::MapLibreTile;
use maplibre_tile_spec::proto::TileSetMetadata;

lazy_static! {
    static ref ASSETS_MLT: Box<Vec<u8>> = Box::new(read(Path::new("assets").join("test.mlt")).expect("Can't read data file"));
    static ref ASSETS_MLT_META: Box<Vec<u8>> = Box::new(read(Path::new("assets").join("test.mlt.meta.pbf")).expect("Can't read metadata file"));

    static ref METADATA: Box<TileSetMetadata<'static>> = {
        let mut reader = BytesReader::from_bytes(&*ASSETS_MLT_META);
        Box::new(TileSetMetadata::from_reader(&mut reader, &*ASSETS_MLT_META).expect("Can't parse metadata"))
    };
}




fn bench_test_tile() {
    MapLibreTile::decode(&ASSETS_MLT, &**METADATA);
}




fn criterion_benchmark(c: &mut Criterion) {
    // Access each, to lazy init
    let _ = ASSETS_MLT.len();
    let _ = ASSETS_MLT_META.len();
    let _ = METADATA.name.clone();

    c.bench_function("Example-tile decode", |b| b.iter(|| bench_test_tile()));
}


criterion_group!{
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(2400));
    targets = criterion_benchmark
}
criterion_main!(benches);
