use std::mem::size_of;

use num_traits::{AsPrimitive, WrappingAdd, WrappingSub};
use zigzag::ZigZag;

use crate::decoder::debug_assert_alloc;
use crate::errors::AsMltError as _;
use crate::{Decoder, MltError};

/// ZigZag-encode a slice of signed values into unsigned values.
#[must_use]
pub fn encode_zigzag<T: ZigZag>(data: &[T]) -> Vec<T::UInt> {
    data.iter().map(|&v| T::encode(v)).collect()
}

fn encode_delta<T: Copy + WrappingSub>(data: &[T]) -> Vec<T> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(data.len());
    result.push(data[0]);
    for i in 1..data.len() {
        result.push(data[i].wrapping_sub(&data[i - 1]));
    }
    result
}

/// ZigZag-encode a slice of signed values after delta-encoding.
#[must_use]
pub fn encode_zigzag_delta<T: Copy + ZigZag + WrappingSub<Output = T>>(data: &[T]) -> Vec<T::UInt> {
    encode_zigzag(&encode_delta(data))
}

/// Encode signed integer vec2 values using componentwise delta + zigzag encoding.
///
/// Input: `[x0, y0, x1, y1, ...]`
/// Output: `[zigzag(x0-0), zigzag(y0-0), zigzag(x1-x0), zigzag(y1-y0), ...]`
///
/// This is the inverse of `decode_componentwise_delta_vec2s`.
#[must_use]
pub fn encode_componentwise_delta_vec2s<T>(data: &[T]) -> Vec<T::UInt>
where
    T: ZigZag + WrappingSub,
{
    let mut result = Vec::with_capacity(data.len());
    let mut prev_x = T::zero();
    let mut prev_y = T::zero();
    for chunk in data.chunks_exact(2) {
        let (x, y) = (chunk[0], chunk[1]);
        result.push(T::encode(x.wrapping_sub(&prev_x)));
        result.push(T::encode(y.wrapping_sub(&prev_y)));
        (prev_x, prev_y) = (x, y);
    }
    result
}

/// ZigZag-decode a slice, charging `dec` for the output allocation.
pub fn decode_zigzag<T: ZigZag>(data: &[T::UInt], dec: &mut Decoder) -> Result<Vec<T>, MltError> {
    dec.consume(u32::try_from(data.len() * size_of::<T>()).or_overflow()?)?;
    Ok(data.iter().map(|&v| T::decode(v)).collect())
}

/// Decode a vector of ZigZag-encoded unsigned deltas, charging `dec` for the output allocation.
pub fn decode_zigzag_delta<T: Copy + ZigZag + WrappingAdd + AsPrimitive<U>, U: 'static + Copy>(
    data: &[T::UInt],
    dec: &mut Decoder,
) -> Result<Vec<U>, MltError> {
    dec.consume(u32::try_from(data.len() * size_of::<U>()).or_overflow()?)?;
    Ok(data
        .iter()
        .scan(T::zero(), |state, &v| {
            *state = state.wrapping_add(&T::decode(v));
            Some((*state).as_())
        })
        .collect())
}

/// Decode ([`ZigZag`] + delta) for Vec2s, charging `dec` for the output allocation.
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag + WrappingAdd>(
    data: &[T::UInt],
    dec: &mut Decoder,
) -> Result<Vec<T>, MltError> {
    use crate::MltError::InvalidPairStreamSize;

    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(InvalidPairStreamSize(data.len()));
    }

    let alloc_size = data.len();
    dec.consume(u32::try_from(alloc_size * size_of::<T>()).or_overflow()?)?;
    let mut result = dec.alloc(alloc_size)?;
    let mut last1 = T::zero();
    let mut last2 = T::zero();

    for i in (0..data.len()).step_by(2) {
        last1 = last1.wrapping_add(&T::decode(data[i]));
        last2 = last2.wrapping_add(&T::decode(data[i + 1]));
        result.push(last1);
        result.push(last2);
    }

    debug_assert_alloc(&result, alloc_size);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::test_helpers::dec;

    proptest! {
        #[test]
        fn test_zigzag_roundtrip_i64(data: Vec<i64>) {
            let encoded = encode_zigzag(&data);
            let decoded = decode_zigzag::<i64>(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_delta_roundtrip_i32(data: Vec<i32>) {
            if data.is_empty() { return Ok(()); }
            let encoded = encode_zigzag_delta(&data);
            let decoded: Vec<i32> = decode_zigzag_delta::<i32, i32>(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_componentwise_delta_vec2s(data: Vec<i32>) {
            if data.len() <= 1 {
                return Err(TestCaseError::reject("data not valid vertices"))
            }
            // done this way to not have to reject less
            let data_slice = if data.len().is_multiple_of(2) {
                &data
            } else {
                &data[..data.len() - 1]
            };
            let encoded = encode_componentwise_delta_vec2s(data_slice);
            let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(data_slice, &decoded);
        }
    }

    #[test]
    fn test_encode_zigzag_empty() {
        assert!(encode_zigzag::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_encode_delta_empty() {
        assert!(encode_delta::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_encode_zigzag_delta_empty() {
        assert!(encode_zigzag_delta::<i32>(&[]).is_empty());
    }

    #[test]
    fn test_decode_zigzag_i32() {
        let encoded_u32 = [0u32, 1, 2, 3, 4, 5, u32::MAX];
        let expected_i32 = [0i32, -1, 1, -2, 2, -3, i32::MIN];
        let decoded_i32 = decode_zigzag::<i32>(&encoded_u32, &mut dec()).unwrap();
        assert_eq!(decoded_i32, expected_i32);
    }

    #[test]
    fn test_decode_zigzag_i64() {
        let encoded_u64 = [0u64, 1, 2, 3, 4, 5, u64::MAX];
        let expected_i64 = [0i64, -1, 1, -2, 2, -3, i64::MIN];
        let decoded_i64 = decode_zigzag::<i64>(&encoded_u64, &mut dec()).unwrap();
        assert_eq!(decoded_i64, expected_i64);
    }

    #[test]
    fn test_decode_zigzag_empty() {
        assert!(decode_zigzag::<i32>(&[], &mut dec()).unwrap().is_empty());
    }

    #[test]
    fn test_decode_zigzag_delta_empty() {
        assert!(
            decode_zigzag_delta::<i32, i32>(&[], &mut dec())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_decode_componentwise_delta_vec2s() {
        let values = &[1_u32, 2, 3, 4];
        let decoded = decode_componentwise_delta_vec2s::<i32>(values, &mut dec()).unwrap();
        assert_eq!(&decoded, &[-1_i32, 1, -3, 3]);
    }
}
