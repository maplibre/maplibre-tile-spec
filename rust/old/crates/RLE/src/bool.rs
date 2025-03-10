
use alloc::vec::Vec;

pub fn encode_bool(input: &[bool]) -> Vec<u8> {
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
                encoded.push(current_value as u8);
                encoded.push(count);
                count = 0;
            }
        } else {
            encoded.push(current_value as u8);
            encoded.push(count);
            current_value = value;
            count = 1;
        }
    }

    encoded.push(current_value as u8);
    encoded.push(count);
    encoded
}

pub fn decode_bool(encoded: &[u8]) -> Vec<bool> {
    let mut decoded = Vec::new();

    let mut i = 0;
    while i < encoded.len() {
        let value = encoded[i] != 0;
        let count = encoded[i + 1];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 2;
    }

    decoded
}
