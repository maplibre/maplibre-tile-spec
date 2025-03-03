use alloc::vec::Vec;

pub fn encode_f64(input: &[f64]) -> Vec<u8> {
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

pub fn decode_f64(encoded: &[u8]) -> Vec<f64> {
    let mut decoded = Vec::new();

    let mut i = 0;
    while i < encoded.len() {
        let value = f64::from_le_bytes([
            encoded[i], encoded[i + 1], encoded[i + 2], encoded[i + 3],
            encoded[i + 4], encoded[i + 5], encoded[i + 6], encoded[i + 7]
        ]);
        let count = encoded[i + 8];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 9;
    }

    decoded
}
