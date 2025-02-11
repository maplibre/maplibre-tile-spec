use alloc::vec::Vec;

pub fn encode_u8(input: &[u8]) -> Vec<u8> {
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
                encoded.push(current_value);
                encoded.push(count);
                count = 0;
            }
        } else {
            encoded.push(current_value);
            encoded.push(count);
            current_value = value;
            count = 1;
        }
    }

    encoded.push(current_value);
    encoded.push(count);
    encoded
}

pub fn decode_u8(encoded: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();

    let mut i = 0;
    while i < encoded.len() {
        let value = encoded[i];
        let count = encoded[i + 1];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 2;
    }

    decoded
}
