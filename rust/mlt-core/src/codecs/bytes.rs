use crate::MltError::BufferUnderflow;
use crate::MltRefResult;
use crate::utils::{AsUsize as _, take};

/// Encode a `u32` slice into little-endian bytes.
#[must_use]
pub fn encode_u32s_to_bytes(data: &[u32]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() * 4);
    for &val in data {
        output.extend_from_slice(&val.to_le_bytes());
    }
    output
}

/// Encode a `u64` slice into little-endian bytes.
#[must_use]
pub fn encode_u64s_to_bytes(data: &[u64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() * 8);
    for &val in data {
        output.extend_from_slice(&val.to_le_bytes());
    }
    output
}

/// Helper to pack a `Vec<bool>` into `Vec<u8>` where each byte represents 8 booleans.
#[must_use]
pub fn encode_bools_to_bytes(bools: &[bool]) -> Vec<u8> {
    let num_bytes = bools.len().div_ceil(8);
    let mut bytes = vec![0u8; num_bytes];
    for (i, _) in bools.iter().enumerate().filter(|(_, bit)| **bit) {
        bytes[i / 8] |= 1 << (i % 8);
    }
    bytes
}

/// Decode a slice of bytes into a vector of u64 values assuming little-endian encoding
/// TODO: Should this return `MltRefResult`, or should it assert the entire input is consumed?
pub fn decode_bytes_to_u64s(mut input: &[u8], num_values: u32) -> MltRefResult<'_, Vec<u64>> {
    let Some(expected_bytes) = num_values.checked_mul(8) else {
        return Err(BufferUnderflow(u32::MAX, input.len()));
    };
    if input.len() < expected_bytes.as_usize() {
        return Err(BufferUnderflow(expected_bytes, input.len()));
    }

    let mut values = Vec::with_capacity(num_values.as_usize());
    for _ in 0..num_values {
        let (new_input, bytes) = take(input, 8)?;
        let value = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        values.push(value);
        input = new_input;
    }
    Ok((input, values))
}

/// Decode a slice of bytes into a vector of u32 values assuming little-endian encoding
pub fn decode_bytes_to_u32s(mut input: &[u8], num_values: u32) -> MltRefResult<'_, Vec<u32>> {
    let Some(expected_bytes) = num_values.checked_mul(4) else {
        return Err(BufferUnderflow(u32::MAX, input.len()));
    };
    if input.len() < expected_bytes.as_usize() {
        return Err(BufferUnderflow(expected_bytes, input.len()));
    }

    let mut values = Vec::with_capacity(num_values.as_usize());
    for _ in 0..num_values {
        let (new_input, bytes) = take(input, 4)?;
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        values.push(value);
        input = new_input;
    }
    Ok((input, values))
}

/// Helper to unpack a `Vec<u8>` into `Vec<bool>` where each byte represents 8 booleans.
#[must_use]
pub fn decode_bytes_to_bools(bytes: &[u8], num_bools: usize) -> Vec<bool> {
    debug_assert!(num_bools <= bytes.len() * 8);
    (0..num_bools)
        .map(|i| (bytes[i / 8] >> (i % 8)) & 1 == 1)
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::test_helpers::assert_empty;

    proptest! {
        #[test]
        fn encode_bools_to_bytes_roundtrip(bools: Vec<bool>) {
            let bools_rountrip = decode_bytes_to_bools(&encode_bools_to_bytes(&bools), bools.len());
            prop_assert_eq!(bools_rountrip, bools);
        }

        #[test]
        fn test_u32_bytes_roundtrip(data: Vec<u32>) {
            let encoded = encode_u32s_to_bytes(&data);
            let (rem, decoded) = decode_bytes_to_u32s(&encoded, u32::try_from(data.len()).unwrap()).unwrap();
            prop_assert_eq!(data, decoded);
            assert_empty(rem);
        }

        #[test]
        fn test_u64_bytes_roundtrip(data: Vec<u64>) {
            let encoded = encode_u64s_to_bytes(&data);
            let (rem, decoded) = decode_bytes_to_u64s(&encoded, u32::try_from(data.len()).unwrap()).unwrap();
            prop_assert_eq!(data, decoded);
            assert_empty(rem);
        }
    }

    #[test]
    fn test_bytes_to_u32s_valid() {
        // Little-endian representation:
        // [0x04, 0x03, 0x02, 0x01] -> 0x01020304
        // [0xDD, 0xCC, 0xBB, 0xAA] -> 0xAABBCCDD
        let bytes: [u8; 8] = [0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA];
        let res = decode_bytes_to_u32s(&bytes, 2);
        assert!(res.is_ok(), "Should decode valid buffer with 2 values");
        let (remaining, u32s) = res.unwrap();
        assert_empty(remaining);
        assert_eq!(
            u32s,
            vec![0x0102_0304, 0xAABB_CCDD],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_bytes_to_u32s_empty() {
        let bytes: [u8; 0] = [];
        let res = decode_bytes_to_u32s(&bytes, 0);
        assert!(res.is_ok(), "Empty slice with 0 values is valid");
        let (remaining, u32s) = res.unwrap();
        assert_empty(remaining);
        assert!(
            u32s.is_empty(),
            "Output should be an empty Vec for 0 values"
        );
    }

    #[test]
    fn test_bytes_to_u32s_buffer_underflow() {
        // Only 4 bytes but requesting 2 values (8 bytes needed)
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let res = decode_bytes_to_u32s(&bytes, 2);
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
        let res = decode_bytes_to_u32s(&bytes, 2);
        assert!(res.is_ok(), "Should decode 2 values from larger buffer");
        let (remaining, u32s) = res.unwrap();
        assert_eq!(remaining.len(), 4, "Should have 4 bytes remaining");
        assert_eq!(u32s.len(), 2, "Should have exactly 2 values");
        assert_eq!(
            u32s,
            vec![0x0403_0201, 0x0807_0605],
            "Decoded values should match"
        );
    }

    #[test]
    fn test_decode_u32() {
        let bytes = [1, 0, 0, 0, 2, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        assert_eq!(decode_bytes_to_u32s(&bytes, 2).unwrap(), expected);
    }

    #[test]
    fn test_decode_u64() {
        let bytes = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        let expected = (&[][..], vec![1, 2]);
        assert_eq!(decode_bytes_to_u64s(&bytes, 2).unwrap(), expected);
    }

    #[test]
    fn test_decode_bytes_to_u32s_empty() {
        let (input, decoded) = decode_bytes_to_u32s(&[], 0).unwrap();
        assert_empty(input);
        assert!(decoded.is_empty());
    }
}
