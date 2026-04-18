use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mlt_core::__private::Morton;
use mlt_core::Decoder;

const NUM_BITS: u32 = 15;
const COORDINATE_SHIFT: u32 = 1 << (NUM_BITS - 1);

// This code runs in CI because of --all-targets, so make it run really fast.
#[cfg(debug_assertions)]
pub const BENCHMARKED_LENGTHS: [u32; 1] = [1];
#[cfg(not(debug_assertions))]
pub const BENCHMARKED_LENGTHS: [u32; 3] = [64, 256, 1024];

/// Interleave `x` and `y` into a single Morton code using 15 bits per component.
///
/// Even bit positions encode `x`, odd positions encode `y`.
/// This is the inverse of [`Morton::decode_codes`] / [`Morton::decode_delta`].
#[must_use]
#[inline]
pub fn encode_morton_15(x: u32, y: u32) -> u32 {
    let mut code = 0u32;
    for bit in 0..15 {
        code |= ((x >> bit) & 1) << (2 * bit);
        code |= ((y >> bit) & 1) << (2 * bit + 1);
    }
    code
}

fn make_morton_codes(n: u32) -> Vec<u32> {
    (0..n)
        .map(|i| {
            let x = (i * 7 + 13) & 0x7FFF;
            let y = (i * 11 + 31) & 0x7FFF;
            encode_morton_15(x, y)
        })
        .collect()
}

fn make_morton_deltas(n: u32) -> Vec<u32> {
    let codes = make_morton_codes(n);
    let mut prev = 0i32;
    codes
        .iter()
        .map(|&c| {
            let delta = c.cast_signed().wrapping_sub(prev).cast_unsigned();
            prev = c.cast_signed();
            delta
        })
        .collect()
}

fn bench_impls<I: Clone, O>(
    c: &mut Criterion,
    group_name: &str,
    make_input: impl Fn(u32) -> Vec<I>,
    imp: impl Fn(&[I]) -> O,
) {
    let mut group = c.benchmark_group(group_name);
    for n in BENCHMARKED_LENGTHS {
        let input = make_input(n);
        group.throughput(Throughput::Elements(u64::from(n)));
        group.bench_with_input(BenchmarkId::new("impl", n), &input, |b, input| {
            b.iter_batched(
                || input.clone(),
                |i| black_box(imp(&i)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_morton(c: &mut Criterion) {
    let meta = Morton {
        bits: NUM_BITS,
        shift: COORDINATE_SHIFT,
    };
    bench_impls(c, "morton/decode_codes", make_morton_codes, |v| {
        meta.decode_codes(v, &mut Decoder::with_max_size(u32::MAX))
            .unwrap()
    });

    bench_impls(c, "morton/decode_delta", make_morton_deltas, |v| {
        meta.decode_delta(v, &mut Decoder::with_max_size(u32::MAX))
            .unwrap()
    });
}

criterion_group!(benches, bench_morton);
criterion_main!(benches);
