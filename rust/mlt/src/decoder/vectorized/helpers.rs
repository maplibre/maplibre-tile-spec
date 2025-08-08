use crate::MltError;

use zigzag::ZigZag;

/// Decode (ZigZag + delta) for Vec2s
/// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag>(data: &[T::UInt]) -> Result<Vec<T>, MltError> {
    let len = data.len();
    if len % 2 != 0 || len < 2 {
        return Err(MltError::DecodeError(format!(
            "Input must be even-length and >= 2. Invalid length: {len}"
        )));
    }

    let mut result = Vec::with_capacity(len);
    result.push(T::decode(data[0]));
    result.push(T::decode(data[1]));

    for i in (2..len).step_by(2) {
        result.push(T::decode(data[i]) + result[i - 2]);
        result.push(T::decode(data[i + 1]) + result[i - 1]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_componentwise_delta_vec2s() {
        // original Vec2s: [(3, 5), (7, 6), (12, 4)]
        // delta:          [3, 5, 4, 1, 5, -2]
        // ZigZag:         [6, 10, 8, 2, 10, 3]
        let encoded_from_positives: Vec<u32> = vec![6, 10, 8, 2, 10, 3];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_positives).unwrap();
        assert_eq!(decoded, vec![3, 5, 7, 6, 12, 4]);

        // original Vec2s: [(3, 5), (-1, 6), (4, -4)]
        // delta:          [3, 5, -4, 1, 5, -10]
        // ZigZag:         [6, 10, 7, 2, 10, 19]
        let encoded_from_negatives: Vec<u32> = vec![6, 10, 7, 2, 10, 19];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_negatives).unwrap();
        assert_eq!(decoded, vec![3, 5, -1, 6, 4, -4]);
    }
}
