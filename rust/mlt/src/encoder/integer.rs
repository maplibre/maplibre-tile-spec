pub fn encoded_u32s_to_bytes(encoded: &[u32]) -> Vec<u8> {
    let mut encoded_bytes: Vec<u8> = Vec::with_capacity(std::mem::size_of_val(encoded));
    for val in encoded.iter() {
        encoded_bytes.extend(&val.to_be_bytes());
    }
    encoded_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoded_u32s_to_bytes() {
        let input: Vec<u32> = vec![0x12345678, 0x90abcdef];
        let result: Vec<u8> = encoded_u32s_to_bytes(&input);
        assert_eq!(result, vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]);
    }
}
