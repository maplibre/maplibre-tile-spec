pub fn encode_delta(input: &[i64], output: &mut [i64]) {
    if input.is_empty() || output.len() < input.len() {
        return;
    }

    output[0] = input[0];
    for i in 1..input.len() {
        output[i] = input[i] - input[i - 1];
    }
}

pub fn decode_delta(encoded: &[i64], output: &mut [i64]) {
    if encoded.is_empty() || output.len() < encoded.len() {
        return;
    }

    output[0] = encoded[0];
    for i in 1..encoded.len() {
        output[i] = output[i - 1] + encoded[i];
    }
}