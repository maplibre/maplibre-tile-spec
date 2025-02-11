use alloc::vec::Vec;

pub fn encode_u32(input: &[u32]) -> Vec<u8> {
    let mut encoded = Vec::new();
    if input.is_empty() {
        return encoded;
    }

    let mut current_value = input[0];
    let mut count = 1;

    for &value in &input[1..] {
        if value == current_value {
            count += 1;
            if count == u8::MAX {
                encoded.extend_from_slice(&current_value.to_le_bytes());
                encoded.push(count);
                count = 0;
            }
        } else {
            encoded.extend_from_slice(&current_value.to_le_bytes());
            encoded.push(count);
            current_value = value;
            count = 1;
        }
    }

    encoded.extend_from_slice(&current_value.to_le_bytes());
    encoded.push(count);
    encoded
}

pub fn decode_u32(encoded: &[u8]) -> Vec<u32> {
    let mut decoded = Vec::new();

    let mut i = 0;
    while i < encoded.len() {
        let value = u32::from_le_bytes([encoded[i], encoded[i + 1], encoded[i + 2], encoded[i + 3]]);
        let count = encoded[i + 4];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 5;
    }

    decoded
}
