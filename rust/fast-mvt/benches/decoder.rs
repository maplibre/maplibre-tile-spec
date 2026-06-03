use std::hint::black_box;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main};
use fast_mvt::MvtReaderRef;

mod common;

use common::load_repo_mvt_files;

fn bench_decode(c: &mut Criterion) {
    let tiles = read_sample_data();

    let mut group = c.benchmark_group("mvt decode");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    bench_tiles(&mut group, "fast-mvt traverse", &tiles, traverse_fast_mvt);
    bench_tiles(
        &mut group,
        "mvt-reader traverse",
        &tiles,
        traverse_mvt_reader,
    );
    group.finish();
}

fn read_sample_data() -> Vec<Vec<u8>> {
    let fixtures = load_repo_mvt_files();

    let tiles = fixtures
        .into_iter()
        .filter(|v| {
            mvt_reader::Reader::new(v.clone())
                .and_then(|vv| vv.get_layer_metadata().map(|_| ()))
                .is_ok()
        })
        .collect::<Vec<_>>();

    tiles
}

fn bench_tiles<R>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    tiles: &[Vec<u8>],
    mut bench_fn: impl FnMut(&[u8]) -> R,
) {
    if tiles.is_empty() {
        return;
    }
    let bytes: usize = tiles.iter().map(Vec::len).sum();
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function(format!("{name} ({} tiles)", tiles.len()), |bench| {
        bench.iter(|| {
            for data in tiles {
                black_box(bench_fn(black_box(data.as_slice())));
            }
        });
    });
}

fn traverse_fast_mvt(data: &[u8]) {
    let reader = MvtReaderRef::new(data).expect("fast-mvt parse");
    for layer in reader.layers() {
        for feature in layer.features() {
            black_box(feature.id());
            black_box(feature.geometry().expect("fast-mvt geometry"));
            for property in feature.properties() {
                black_box(property.expect("fast-mvt property"));
            }
        }
    }
}

fn traverse_mvt_reader(data: &[u8]) {
    let reader = mvt_reader::Reader::new(data.to_vec()).expect("mvt-reader parse");

    for layer in reader
        .get_layer_metadata()
        .expect("mvt-reader layer metadata")
    {
        for feature in reader
            .get_features_as::<i32>(layer.layer_index)
            .expect("mvt-reader features")
        {
            black_box(feature.id);
            black_box(feature.get_geometry());
            for property in feature.properties.as_ref().expect("mvt-reader properties") {
                black_box(property);
            }
        }
    }
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
