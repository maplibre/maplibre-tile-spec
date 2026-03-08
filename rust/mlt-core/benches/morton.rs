use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::utils::{decode_morton_codes, decode_morton_delta};

// num_bits=15 covers tile zoom levels where coordinates fit in 15 bits per axis
// (max coord = 2^15 - 1 = 32767), the common case for OMT tiles.
const NUM_BITS: u32 = 15;
const COORDINATE_SHIFT: u32 = 1 << (NUM_BITS - 1);

fn make_morton_codes(n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| {
            let x = (i as u32 * 7 + 13) & 0x7FFF;
            let y = (i as u32 * 11 + 31) & 0x7FFF;
            let mut code = 0u32;
            for bit in 0..15u32 {
                code |= ((x >> bit) & 1) << (2 * bit);
                code |= ((y >> bit) & 1) << (2 * bit + 1);
            }
            code
        })
        .collect()
}

fn make_morton_deltas(n: usize) -> Vec<u32> {
    let codes = make_morton_codes(n);
    let mut prev = 0i32;
    codes
        .iter()
        .map(|&c| {
            #[expect(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let delta = (c as i32).wrapping_sub(prev) as u32;
            #[expect(clippy::cast_possible_wrap)]
            {
                prev = c as i32;
            }
            delta
        })
        .collect()
}

fn bench_morton_codes(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton/decode_codes");

    for n in [64usize, 256, 1024, 4096, 16384] {
        let codes = make_morton_codes(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &codes, |b, codes| {
            b.iter_batched(
                || codes.clone(),
                |codes| {
                    black_box(decode_morton_codes(
                        black_box(&codes),
                        NUM_BITS,
                        COORDINATE_SHIFT,
                    ))
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_morton_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton/decode_delta");

    for n in [64usize, 256, 1024, 4096, 16384] {
        let deltas = make_morton_deltas(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &deltas, |b, deltas| {
            b.iter_batched(
                || deltas.clone(),
                |deltas| {
                    black_box(decode_morton_delta(
                        black_box(&deltas),
                        NUM_BITS,
                        COORDINATE_SHIFT,
                    ))
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_morton_codes, bench_morton_delta);
criterion_main!(benches);
