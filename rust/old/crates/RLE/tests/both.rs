use RLE::{decode_bool, decode_f32, decode_f64, decode_u16, decode_u32, decode_u8, encode_bool, encode_f32, encode_f64, encode_u16, encode_u32, encode_u8};

#[test]
fn test_encode_decode_bool_rle() {
    let input = [true, true, false, false, false, true];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<bool> = vec![false; 6];

    encode_bool(&input, &mut encoded);
    decode_bool(&encoded, &mut decoded);
    
    assert_eq!(decoded, input);
}

#[test]
fn test_encode_decode_u8_rle() {
    let input = [1, 1, 2, 2, 2, 3];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<u8> = vec![0u8; 6];

    encode_u8(&input, &mut encoded);
    decode_u8(&encoded, &mut decoded);

    assert_eq!(decoded, input);
}

#[test]
fn test_encode_decode_u16_rle() {
    let input = [1, 1, 2, 2, 2, 3];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<u16> = vec![0u16; 6];

    encode_u16(&input, &mut encoded);
    decode_u16(&encoded, &mut decoded);

    assert_eq!(decoded, input);
}

#[test]
fn test_encode_decode_u32_rle() {
    let input = [1, 1, 2, 2, 2, 3];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<u32> = vec![0u32; 6];

    encode_u32(&input, &mut encoded);
    decode_u32(&encoded, &mut decoded);

    assert_eq!(decoded, input);
}

#[test]
fn test_encode_decode_f32_rle() {
    let input = [1.0, 1.0, 2.0, 2.0, 2.0, 3.0];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<f32> = vec![0f32; 27];

    encode_f32(&input, &mut encoded);
    decode_f32(&encoded, &mut decoded);

    assert_eq!(decoded, input);
}

#[test]
fn test_encode_decode_f64_rle() {
    let input = [1.0, 1.0, 2.0, 2.0, 2.0, 3.0];
    let mut encoded: Vec<u8> = vec![0u8; 6];
    let mut decoded: Vec<f64> = vec![0f64; 6];

    encode_f64(&input, &mut encoded);
    decode_f64(&encoded, &mut decoded);

    assert_eq!(decoded, input);
}
