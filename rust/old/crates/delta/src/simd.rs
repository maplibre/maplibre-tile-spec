
use core::simd::Simd;

#[cfg(feature = "SIMDx2")]
const VECTOR_SIZE: usize = 2;
#[cfg(feature = "SIMDx4")]
const VECTOR_SIZE: usize = 4;
#[cfg(feature = "SIMDx8")]
const VECTOR_SIZE: usize = 8;

pub fn encode_delta(input: &[i64], output: &mut [i64]) {
    let mut prev: Simd<i64, VECTOR_SIZE> = Simd::splat(0);

    // Process all chunks that fit into SIMD vectors
    let chunks = input.chunks_exact(VECTOR_SIZE);
    let remainder = chunks.remainder();

    for (chunk_in, chunk_out) in chunks.zip(output.chunks_exact_mut(VECTOR_SIZE)) {
        let current = Simd::from_slice(chunk_in);
        let delta = &current - prev;
        delta.copy_to_slice(chunk_out);
        prev = current;
    }

    // Handle the remaining elements
    for i in 0..remainder.len() {
        let current = remainder[i].clone();
        output[input.len() - remainder.len() + i] = current - prev[i % VECTOR_SIZE];
        prev = Simd::splat(current);
    }
}

/// # Info
/// Decodes the input slice with delta encoding in a performant manor with SIMD support
pub fn decode_delta(input: &[i64], output: &mut [i64]) {
    let mut prev: Simd<i64, VECTOR_SIZE> = Simd::splat(0);

    // Process all chunks that fit into SIMD vectors
    let chunks = input.chunks_exact(VECTOR_SIZE);
    let remainder = chunks.remainder();

    for (chunk_in, chunk_out) in chunks.zip(output.chunks_exact_mut(VECTOR_SIZE)) {
        let delta = Simd::from_slice(chunk_in);
        let current = delta + prev;
        current.copy_to_slice(chunk_out);
        prev = current;
    }

    // Handle the remaining elements
    for i in 0..remainder.len() {
        let delta = remainder[i];
        output[input.len() - remainder.len() + i] = delta + prev[i % VECTOR_SIZE];
        prev = Simd::splat(output[input.len() - remainder.len() + i]);
    }
}
