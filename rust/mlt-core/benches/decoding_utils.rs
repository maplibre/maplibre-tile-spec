use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::utils::{decode_morton_codes, decode_morton_delta};

const NUM_BITS: u32 = 15;
const COORDINATE_SHIFT: u32 = 1 << (NUM_BITS - 1);

const SIZES: &[usize] = &[64, 256, 1024];

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

fn bench_impls<I, O>(
    c: &mut Criterion,
    group_name: &str,
    make_input: impl Fn(usize) -> I,
    impls: &[(&str, fn(&I) -> O)],
) where
    I: Clone + Send + 'static,
{
    let mut group = c.benchmark_group(group_name);
    for &n in SIZES {
        let input = make_input(n);
        group.throughput(Throughput::Elements(n as u64));
        for &(name, f) in impls {
            group.bench_with_input(BenchmarkId::new(name, n), &input, |b, input| {
                b.iter_batched(
                    || input.clone(),
                    |i| black_box(f(&i)),
                    BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();
}

fn bench_morton(c: &mut Criterion) {
    bench_impls(
        c,
        "morton/decode_codes",
        make_morton_codes,
        &[("scalar", |v| {
            decode_morton_codes(v, NUM_BITS, COORDINATE_SHIFT)
        })],
    );

    bench_impls(
        c,
        "morton/decode_delta",
        make_morton_deltas,
        &[("scalar", |v| {
            decode_morton_delta(v, NUM_BITS, COORDINATE_SHIFT)
        })],
    );
}

criterion_group!(benches, bench_morton);
criterion_main!(benches);
