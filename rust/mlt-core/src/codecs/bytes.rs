use integer_encoding::VarInt;
use usize_cast::IntoUsize as _;

use crate::MltError::UnsupportedPhysicalEncoding;
use crate::codecs::fastpfor::decode_fastpfor;
use crate::decoder::FastPForKind;
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::{Decoder, MltResult};

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
/// Decoder-side mirror of the encoder's `PhysicalIntStreamKind`.
pub trait PhysicalWord: Copy + Sized + VarInt {
    /// Read one little-endian word from exactly `size_of::<Self>()` bytes.
    fn from_le_word(bytes: &[u8]) -> Self;

    /// Narrow a value the caller has already bounded to this word's width.
    #[cfg(feature = "unstable-v2")]
    fn from_u64(value: u64) -> Self;

    /// Physically decode a `FastPFOR`-compressed stream into `Vec<Self>`.
    /// `FastPFOR` only supports `u32`; the `u64` implementation returns an error.
    fn decode_fastpfor(
        data: &[u8],
        num_values: u32,
        kind: FastPForKind,
        dec: &mut Decoder,
    ) -> MltResult<Vec<Self>>;
}

impl PhysicalWord for u32 {
    #[inline]
    fn from_le_word(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("infallible: 4-byte chunk"))
    }

    #[cfg(feature = "unstable-v2")]
    #[inline]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the caller bounds the value to this word's width"
    )]
    fn from_u64(value: u64) -> Self {
        value as Self
    }

    fn decode_fastpfor(
        data: &[u8],
        num_values: u32,
        kind: FastPForKind,
        dec: &mut Decoder,
    ) -> MltResult<Vec<Self>> {
        decode_fastpfor(data, num_values, kind, dec)
    }
}

impl PhysicalWord for u64 {
    #[inline]
    fn from_le_word(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("infallible: 8-byte chunk"))
    }

    #[cfg(feature = "unstable-v2")]
    #[inline]
    fn from_u64(value: u64) -> Self {
        value
    }

    fn decode_fastpfor(
        _data: &[u8],
        _num_values: u32,
        _kind: FastPForKind,
        _dec: &mut Decoder,
    ) -> MltResult<Vec<Self>> {
        Err(UnsupportedPhysicalEncoding("FastPFOR decoding u64"))
    }
}

/// Decode a slice of exactly `num_values` little-endian words into a `Vec<T>`, charging `dec` for the allocation.
pub fn decode_bytes_to_words<T: PhysicalWord>(
    input: &[u8],
    num_values: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<T>> {
    let width = size_of::<T>();
    let expected_bytes = num_values.into_usize().checked_mul(width).or_overflow()?;
    fail_if_invalid_stream_size(input.len(), expected_bytes)?;

    let alloc_size = num_values.into_usize();
    let mut values = dec.alloc(alloc_size)?;
    values.extend(input.chunks_exact(width).map(T::from_le_word));

    debug_assert_length(&values, alloc_size);
    Ok(values)
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
    use bitvec::order::Lsb0;
    use bitvec::view::BitView as _;
    use proptest::prelude::*;

    use super::*;
    use crate::MltError::{InvalidDecodingStreamSize, MemoryLimitExceeded};
    use crate::test_helpers::{dec, starved_dec};

    proptest! {
        #[test]
        fn encode_bools_to_bytes_roundtrip(bools: Vec<bool>) {
            let mut bytes = Vec::new();
            let data = encode_bools_to_bytes(bools.iter().copied(), &mut bytes);
            let packed = &data.view_bits::<Lsb0>()[..bools.len()];
            prop_assert_eq!(packed.iter().by_vals().collect::<Vec<bool>>(), bools);
        }

        #[test]
        fn test_u32_bytes_roundtrip(data: Vec<u32>) {
            let mut encoded = Vec::with_capacity(data.len() * 4);
            for val in &data {
                encoded.extend_from_slice(&val.to_le_bytes());
            }
            let decoded = decode_bytes_to_words::<u32>(&encoded, u32::try_from(data.len()).unwrap(), &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_u64_bytes_roundtrip(data: Vec<u64>) {
            let mut encoded = Vec::with_capacity(data.len() * 8);
            for val in &data {
                encoded.extend_from_slice(&val.to_le_bytes());
            }
            let decoded = decode_bytes_to_words::<u64>(&encoded, u32::try_from(data.len()).unwrap(), &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }

    #[test]
    fn decodes_u32_words_little_endian() {
        let bytes = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let u32s = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec()).unwrap();
        assert_eq!(u32s, vec![0x0102_0304, 0xAABB_CCDD]);
    }

    #[test]
    fn decodes_u64_words_little_endian() {
        let bytes = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let u64s = decode_bytes_to_words::<u64>(&bytes, 2, &mut dec()).unwrap();
        assert_eq!(u64s, vec![1, 2]);
    }

    #[test]
    fn zero_words_decode_from_no_bytes() {
        let u32s = decode_bytes_to_words::<u32>(&[], 0, &mut dec()).unwrap();
        assert_eq!(u32s, [] as [u32; 0]);
    }

    #[test]
    fn too_few_bytes_for_the_word_count_is_an_error() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let err = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec()).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(4, 8)), "{err:?}");
    }

    #[test]
    fn trailing_bytes_after_the_word_count_are_an_error() {
        let bytes = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let err = decode_bytes_to_words::<u32>(&bytes, 2, &mut dec()).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(12, 8)), "{err:?}");
    }

    #[test]
    fn a_maximal_word_count_fails_the_size_check_before_allocating() {
        let err = decode_bytes_to_words::<u32>(&[0, 0, 0, 0], u32::MAX, &mut dec()).unwrap_err();
        assert!(
            matches!(err, InvalidDecodingStreamSize(4, 17_179_869_180)),
            "{err:?}"
        );
    }

    #[test]
    fn decoding_past_the_memory_budget_is_rejected() {
        let bytes = [0; 8];
        let err = decode_bytes_to_words::<u32>(&bytes, 2, &mut starved_dec()).unwrap_err();
        assert!(matches!(err, MemoryLimitExceeded { .. }), "{err:?}");
    }

    #[test]
    fn fastpfor_decoding_into_u64_words_is_unsupported() {
        let err =
            <u64 as PhysicalWord>::decode_fastpfor(&[], 0, FastPForKind::Block256Be, &mut dec())
                .unwrap_err();
        assert!(
            matches!(err, UnsupportedPhysicalEncoding("FastPFOR decoding u64")),
            "{err:?}"
        );
    }

    #[test]
    fn trailing_bytes_on_zero_words_are_an_error() {
        let bytes = [0x01];
        let err = decode_bytes_to_words::<u32>(&bytes, 0, &mut dec()).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(1, 0)), "{err:?}");
    }
}
