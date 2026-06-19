use num_traits::{AsPrimitive, WrappingAdd, WrappingSub};
use zigzag::ZigZag;

use crate::MltError::InvalidPairStreamSize;
use crate::{Decoder, MltResult};

/// Upper bound on vertex components for the componentwise-delta codecs (X, Y, Z, M). Lets the
/// per-component running state live in a small stack array instead of a per-call heap `Vec`.
const MAX_COMPONENTS: usize = 4;

/// ZigZag-encode `data` into `target`.
///
/// `target` is treated as a scratch buffer: cleared before writing.
pub fn encode_zigzag<'a, T: ZigZag>(data: &[T], target: &'a mut Vec<T::UInt>) -> &'a [T::UInt] {
    target.clear();
    target.extend(data.iter().map(|&v| T::encode(v)));
    target
}

/// Delta-then-ZigZag-encode `data` into `target` in a single pass.
///
/// `target` is treated as a scratch buffer: cleared before writing.
/// Fuses the delta and zigzag steps to avoid an intermediate allocation.
pub fn encode_zigzag_delta<'a, T: Copy + ZigZag + WrappingSub<Output = T>>(
    data: &[T],
    target: &'a mut Vec<T::UInt>,
) -> &'a [T::UInt] {
    target.clear();
    target.reserve(data.len());
    let mut prev = T::zero();
    for &v in data {
        target.push(T::encode(v.wrapping_sub(&prev)));
        prev = v;
    }
    target
}

/// Encode interleaved `n_dims`-component vertices using componentwise delta + zigzag into `target`.
///
/// Input (`n_dims = 2`): `[x0, y0, x1, y1, ...]`
/// Output: `[zigzag(x0-0), zigzag(y0-0), zigzag(x1-x0), zigzag(y1-y0), ...]`
///
/// The delta runs independently per component position, so `n_dims` must match the value used
/// when decoding. `target` is treated as a scratch buffer: cleared before writing.
/// This is the inverse of `decode_componentwise_delta`.
pub fn encode_componentwise_delta<'a, T>(
    data: &[T],
    n_dims: usize,
    target: &'a mut Vec<T::UInt>,
) -> &'a [T::UInt]
where
    T: ZigZag + WrappingSub,
{
    target.clear();
    target.reserve(data.len());
    debug_assert!(n_dims <= MAX_COMPONENTS);
    let mut prev = [T::zero(); MAX_COMPONENTS];
    for chunk in data.chunks_exact(n_dims) {
        for (j, &v) in chunk.iter().enumerate() {
            target.push(T::encode(v.wrapping_sub(&prev[j])));
            prev[j] = v;
        }
    }
    target
}

/// ZigZag-decode a slice, charging `dec` for the output allocation.
pub fn decode_zigzag<T: ZigZag>(data: &[T::UInt], dec: &mut Decoder) -> MltResult<Vec<T>> {
    dec.consume_items::<T>(data.len())?;
    Ok(data.iter().map(|&v| T::decode(v)).collect())
}

/// Decode a vector of ZigZag-encoded unsigned deltas, charging `dec` for the output allocation.
pub fn decode_zigzag_delta<T: Copy + ZigZag + WrappingAdd + AsPrimitive<U>, U: 'static + Copy>(
    data: &[T::UInt],
    dec: &mut Decoder,
) -> MltResult<Vec<U>> {
    dec.consume_items::<U>(data.len())?;
    Ok(data
        .iter()
        .scan(T::zero(), |state, &v| {
            *state = state.wrapping_add(&T::decode(v));
            Some((*state).as_())
        })
        .collect())
}

/// Decode ([`ZigZag`] + delta) for interleaved `n_dims`-component vertices, charging `dec` for
/// the output allocation. The delta is reconstructed independently per component position, so
/// `n_dims` must match the value used when encoding. Inverse of [`encode_componentwise_delta`].
pub fn decode_componentwise_delta<T: ZigZag + WrappingAdd>(
    data: &[T::UInt],
    n_dims: usize,
    dec: &mut Decoder,
) -> MltResult<Vec<T>> {
    if data.is_empty() || n_dims == 0 || !data.len().is_multiple_of(n_dims) {
        return Err(InvalidPairStreamSize(data.len()));
    }

    let alloc_size = data.len();
    let mut result = dec.alloc(alloc_size)?;
    debug_assert!(n_dims <= MAX_COMPONENTS);
    let mut last = [T::zero(); MAX_COMPONENTS];

    for chunk in data.chunks_exact(n_dims) {
        for (j, &v) in chunk.iter().enumerate() {
            last[j] = last[j].wrapping_add(&T::decode(v));
            result.push(last[j]);
        }
    }

    dec.adjust_alloc(&result, alloc_size)?;
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
            let mut encoded = Vec::new();
            let decoded = decode_zigzag::<i64>(encode_zigzag(&data, &mut encoded), &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }

        #[test]
        fn test_delta_roundtrip_i32(data: Vec<i32>) {
            if data.is_empty() { return Ok(()); }
            let mut encoded = Vec::new();
            encode_zigzag_delta(&data, &mut encoded);
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
            let mut encoded = Vec::new();
            let data = encode_componentwise_delta(data_slice, 2, &mut encoded);
            let decoded = decode_componentwise_delta::<i32>(data, 2, &mut dec()).unwrap();
            prop_assert_eq!(data_slice, &decoded);
        }
    }

    #[test]
    fn test_encode_zigzag_empty() {
        let mut target = Vec::<u32>::new();
        assert!(encode_zigzag::<i32>(&[], &mut target).is_empty());
    }

    #[test]
    fn test_encode_zigzag_delta_empty() {
        let mut target = Vec::<u32>::new();
        encode_zigzag_delta::<i32>(&[], &mut target);
        assert!(target.is_empty());
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
        let decoded = decode_componentwise_delta::<i32>(values, 2, &mut dec()).unwrap();
        assert_eq!(&decoded, &[-1_i32, 1, -3, 3]);
    }

    #[test]
    fn test_componentwise_delta_vec3_roundtrip() {
        let data = [10_i32, 20, 30, 11, 19, 35, 5, 5, 5];
        let mut encoded = Vec::new();
        let enc = encode_componentwise_delta(&data, 3, &mut encoded);
        let decoded = decode_componentwise_delta::<i32>(enc, 3, &mut dec()).unwrap();
        assert_eq!(&data[..], &decoded);
    }
}
