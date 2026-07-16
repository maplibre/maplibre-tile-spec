use integer_encoding::VarInt;
use usize_cast::IntoUsize as _;

use crate::MltError::{BufferUnderflow, UnsupportedPhysicalEncoding};
use crate::codecs::fastpfor::decode_fastpfor;
use crate::utils::take;
use crate::{Decoder, MltRefResult, MltResult};

/// Pack bools into bytes where each byte represents 8 booleans.
pub fn encode_bools_to_bytes(
    bools: impl ExactSizeIterator<Item = bool>,
    target: &mut Vec<u8>,
) -> &[u8] {
    let num_bytes = bools.len().div_ceil(8);
    target.clear();
    target.resize(num_bytes, 0u8);
    for i in bools.enumerate().filter_map(|(i, bit)| bit.then_some(i)) {
        target[i / 8] |= 1 << (i % 8);
    }
    target
}

/// A physical word type a stream can be decoded into (`u32` or `u64`).
///
/// Encapsulates the small per-width differences between the physical decode
/// paths: how one little-endian word is read from bytes, and whether `FastPFOR`
/// (which is `u32`-only) is supported. This is the decoder-side mirror of the
/// encoder's `PhysicalIntStreamKind`.
pub trait PhysicalWord: Copy + Sized + VarInt {
    /// Read one little-endian word from exactly `size_of::<Self>()` bytes.
    fn from_le_word(bytes: &[u8]) -> Self;

    /// Physically decode a `FastPFOR`-compressed stream into `Vec<Self>`.
    ///
    /// `FastPFOR` only supports `u32`; the `u64` implementation returns an error.
    fn decode_fastpfor(data: &[u8], num_values: u32, dec: &mut Decoder) -> MltResult<Vec<Self>>;
}

impl PhysicalWord for u32 {
    #[inline]
    fn from_le_word(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("infallible: 4-byte chunk"))
    }

    fn decode_fastpfor(data: &[u8], num_values: u32, dec: &mut Decoder) -> MltResult<Vec<Self>> {
        decode_fastpfor(data, num_values, dec)
    }
}

impl PhysicalWord for u64 {
    #[inline]
    fn from_le_word(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("infallible: 8-byte chunk"))
    }

    fn decode_fastpfor(_data: &[u8], _num_values: u32, _dec: &mut Decoder) -> MltResult<Vec<Self>> {
        Err(UnsupportedPhysicalEncoding("FastPFOR decoding u64"))
    }
}

/// Decode a slice of bytes into a `Vec<T>` of little-endian words, charging `dec`
/// for the output allocation.
///
/// Returns the remaining (unconsumed) input alongside the decoded values.
/// TODO: ensure the entire input is consumed, and don't return it?
pub fn decode_bytes_to_words<'a, T: PhysicalWord>(
    mut input: &'a [u8],
    num_values: u32,
    dec: &mut Decoder,
) -> MltRefResult<'a, Vec<T>> {
    let width = u32::try_from(size_of::<T>()).expect("word size fits u32");
    let Some(expected_bytes) = num_values.checked_mul(width) else {
        return Err(BufferUnderflow(u32::MAX, input.len()));
    };
    if input.len() < expected_bytes.into_usize() {
        return Err(BufferUnderflow(expected_bytes, input.len()));
    }

    let alloc_size = num_values.into_usize();
    let mut values = dec.alloc(alloc_size)?;

    for _ in 0..num_values {
        let (new_input, bytes) = take(input, width)?;
        values.push(T::from_le_word(bytes));
        input = new_input;
    }

    debug_assert_length(&values, alloc_size);
    Ok((input, values))
}

/// Helper to unpack a `Vec<u8>` into `Vec<bool>` where each byte represents 8 booleans.
/// TODO: Use `BitSlice` from bitvec crate and avoid copying?
pub fn decode_bytes_to_bools(
    bytes: &[u8],
    num_bools: usize,
    dec: &mut Decoder,
) -> MltResult<Vec<bool>> {
    if num_bools > bytes.len() * 8 {
        return Err(BufferUnderflow(
            u32::try_from(num_bools.div_ceil(8))?,
            bytes.len(),
        ));
    }
    let mut result = dec.alloc(num_bools)?;
    for i in 0..num_bools {
        result.push((bytes[i / 8] >> (i % 8)) & 1 == 1);
    }
    debug_assert_length(&result, num_bools);
    Ok(result)
}

#[inline]
pub fn debug_assert_length<T>(buffer: &[T], expected_len: usize) {
    debug_assert_eq!(
        buffer.len(),
        expected_len,
        "Expected buffer to have exact length"
    );
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::test_helpers::{assert_empty, dec};

    proptest! {
        #[test]
        fn encode_bools_to_bytes_roundtrip(bools: Vec<bool>) {
            let mut bytes = Vec::new();
            let data = encode_bools_to_bytes(bools.iter().copied(), &mut bytes);
            let bools_rountrip = decode_bytes_to_bools(data, bools.len(), &mut dec()).unwrap();
            prop_assert_eq!(bools_rountrip, bools);
        }

        #[test]
        fn test_u32_bytes_roundtrip(data: Vec<u32>) {
            let mut encoded = Vec::with_capacity(data.len() * 4);
            for val in &data {
                encoded.extend_from_slice(&val.to_le_bytes());
            }
            let decoded = assert_empty(decode_bytes_to_words::<u32>(&encoded, u32::try_from(data.len()).unwrap(), &mut dec()));
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_u64_bytes_roundtrip(data: Vec<u64>) {
            let mut encoded = Vec::with_capacity(data.len() * 8);
            for val in &data {
                encoded.extend_from_slice(&val.to_le_bytes());
            }
            let decoded = assert_empty(decode_bytes_to_words::<u64>(&encoded, u32::try_from(data.len()).unwrap(), &mut dec()));
            prop_assert_eq!(data, decoded);
        }
    }

    #[test]
    fn test_bytes_to_u32s_valid() {
        // Little-endian representation:
        // [0x04, 0x03, 0x02, 0x01] -> 0x01020304
        // [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
        let bytes: [u8; 8] = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let u32s = assert_empty(decode_bytes_to_words::<u32>(&bytes, 2, &mut dec()));
        assert_eq!(
            u32s,
            vec![0x0102_0304, 0xAABB_CCDD],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_bytes_to_u32s_empty() {
        let bytes: [u8; 0] = [];
        let u32s = assert_empty(decode_bytes_to_words::<u32>(&bytes, 0, &mut dec()));
        assert!(
            u32s.is_empty(),
            "Output should be an empty Vec for 0 values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_buffer_underflow() {
        // Only 4 bytes but requesting 2 values (8 bytes needed)
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let res = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec());
        assert!(
            res.is_err(),
            "Should error if not enough bytes for requested values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_partial_consumption() {
        // 12 bytes (3 values) but only requesting 2 values
        let bytes: [u8; 12] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let res = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec());
        assert!(res.is_ok(), "Should decode 2 values from larger buffer");
        let (remaining, u32s) = res.unwrap();
        assert_eq!(remaining.len(), 4, "Should have 4 bytes remaining");
        assert_eq!(u32s.len(), 2);
        assert_eq!(u32s, vec![0x0403_0201, 0x0807_0605]);
    }

    #[test]
    fn test_decode_u32() {
        let bytes = [1, 0, 0, 0, 2, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        let decoded = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec()).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_u64() {
        let bytes = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        let decoded = decode_bytes_to_words::<u64>(&bytes, 2, &mut dec()).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_bytes_to_u32s_empty() {
        let decoded = assert_empty(decode_bytes_to_words::<u32>(&[], 0, &mut dec()));
        assert!(decoded.is_empty());
    }
}
