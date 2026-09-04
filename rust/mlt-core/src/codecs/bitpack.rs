//! Fixed-width bit packing, the physical layout v2 dictionary code streams compete with.
//!
//! The payload is `[u8 width][ceil(count * width / 8) bytes]`, the values laid
//! LSB-first end to end, so value `i` occupies bits `i * width ..` of the byte run.

use crate::MltError::{InvalidDecodingStreamSize, ParsingBitWidth};
use crate::codecs::bytes::PhysicalWord;
use crate::{MltError, MltResult};

/// Largest width the codec packs, which is what keeps the shift accumulator in range.
pub(crate) const MAX_WIDTH: u32 = 32;

/// How many bits every value of `values` needs, or [`None`] when bit packing cannot express them.
///
/// An empty stream and a stream of zeros both take one bit, since a width of zero
/// would leave the value count as the only thing the payload says.
pub(crate) fn bit_width<T: Copy + Into<u64>>(values: &[T]) -> Option<u32> {
    let max = values.iter().copied().map(Into::into).max().unwrap_or(0);
    let width = (u64::BITS - max.leading_zeros()).max(1);
    (width <= MAX_WIDTH).then_some(width)
}

/// The byte length [`pack`] produces for `count` values of `width` bits, the leading byte included.
pub(crate) fn packed_len(count: usize, width: u32) -> usize {
    1 + (count * width as usize).div_ceil(8)
}

/// Pack `values` at `width` bits each into `out`, which the width byte leads.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the low byte of the accumulator is what is being written"
)]
pub(crate) fn pack<T: Copy + Into<u64>>(values: &[T], width: u32, out: &mut Vec<u8>) {
    debug_assert!((1..=MAX_WIDTH).contains(&width));
    out.push(u8::try_from(width).expect("width is at most MAX_WIDTH"));
    let mut acc: u64 = 0;
    let mut bits = 0_u32;
    for &value in values {
        acc |= value.into() << bits;
        bits += width;
        while bits >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    }
    if bits > 0 {
        out.push(acc as u8);
    }
}

/// Read `count` values back out of a packed payload.
pub fn unpack<T: PhysicalWord>(data: &[u8], count: u32) -> MltResult<Vec<T>> {
    let (&width, packed) = data.split_first().ok_or(InvalidDecodingStreamSize(0, 1))?;
    let width = u32::from(width);
    if !(1..=MAX_WIDTH).contains(&width) {
        return Err(ParsingBitWidth(width));
    }
    let count = count as usize;
    let expected = packed_len(count, width) - 1;
    if packed.len() != expected {
        return Err(InvalidDecodingStreamSize(packed.len(), expected));
    }
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1 << width) - 1
    };
    let mut values = Vec::with_capacity(count);
    let mut acc: u64 = 0;
    let mut bits = 0_u32;
    let mut bytes = packed.iter();
    for _ in 0..count {
        while bits < width {
            let byte = *bytes.next().ok_or(MltError::BufferUnderflow(1, 0))?;
            acc |= u64::from(byte) << bits;
            bits += 8;
        }
        values.push(T::from_u64(acc & mask));
        acc >>= width;
        bits -= width;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty(&[])]
    #[case::zeros(&[0, 0, 0])]
    #[case::one_bit(&[1, 0, 1, 1, 0, 0, 1, 0, 1])]
    #[case::three_bits(&[0, 7, 3, 4, 5, 1])]
    #[case::byte_aligned(&[255, 0, 128, 7])]
    #[case::seventeen_bits(&[0, 131_071, 65_536, 1])]
    #[case::full_width(&[u32::MAX, 0, 1])]
    fn packed_values_read_back(#[case] values: &[u32]) {
        let width = bit_width(values).unwrap();
        let mut out = Vec::new();
        pack(values, width, &mut out);
        assert_eq!(out.len(), packed_len(values.len(), width));
        let back: Vec<u32> = unpack(&out, u32::try_from(values.len()).unwrap()).unwrap();
        assert_eq!(back, values);
    }

    #[rstest]
    #[case::zero_width(&[0, 0])]
    #[case::past_max_width(&[33, 0])]
    fn a_width_outside_the_codec_is_rejected(#[case] payload: &[u8]) {
        let err = unpack::<u32>(payload, 1).unwrap_err();
        assert!(matches!(err, ParsingBitWidth(_)), "{err:?}");
    }

    #[test]
    fn a_payload_of_the_wrong_length_is_rejected() {
        let err = unpack::<u32>(&[4, 0x21], 4).unwrap_err();
        assert!(matches!(err, InvalidDecodingStreamSize(1, 2)), "{err:?}");
    }

    #[test]
    fn a_value_wider_than_the_codec_has_no_width() {
        assert_eq!(bit_width(&[u64::from(u32::MAX) + 1]), None);
    }
}
