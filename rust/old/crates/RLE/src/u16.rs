use alloc::vec::Vec;

pub fn encode_u16(input: &[u16]) -> Vec<u8> {
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

pub fn decode_u16(encoded: &[u8]) -> Vec<u16> {
    let mut decoded = Vec::new();

    let mut i = 0;
    while i < encoded.len() {
        let value = u16::from_le_bytes([encoded[i], encoded[i + 1]]);
        let count = encoded[i + 2];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 3;
    }

    decoded
}
