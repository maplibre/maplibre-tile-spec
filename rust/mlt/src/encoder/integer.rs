/// Converts a slice of u32 integers into a vector of bytes in little-endian (LE) order.
pub fn u32s_to_le_bytes(encoded: &[u32]) -> Vec<u8> {
    let mut encoded_bytes: Vec<u8> = Vec::with_capacity(size_of_val(encoded));
    for val in encoded {
        encoded_bytes.extend(&val.to_le_bytes());
    }
    encoded_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32s_to_le_bytes() {
        let input: Vec<u32> = vec![0x1234_5678, 0x90ab_cdef];
        let result: Vec<u8> = u32s_to_le_bytes(&input);
        assert_eq!(result, vec![0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90]);
    }
}
