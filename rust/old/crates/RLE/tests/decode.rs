use RLE::{decode_bool, decode_f32, decode_f64, decode_u16, decode_u32, decode_u8};

#[test]
fn test_decode_bool_rle() {
    let encoded: [u8; 6] = [1, 2, 0, 3, 1, 1];
    let mut decoded: Vec<bool> = vec![false; 27];

    decode_bool(&encoded, &mut decoded);

    assert_eq!(decoded, vec![true, true, false, false, false, true]);
}

#[test]
fn test_decode_u8_rle() {
    let encoded: [u8; 6] = [1, 2, 0, 3, 1, 1];
    let mut decoded: Vec<u8> = vec![0u8; 27];

    decode_u8(&encoded, &mut decoded);

    assert_eq!(decoded, vec![1, 1, 2, 2, 2, 3]);
}

#[test]
fn test_decode_u16_rle() {
    let encoded: [u8; 9] = [1, 0, 2, 2, 0, 3, 3, 0, 1];
    let mut decoded: Vec<u16> = vec![0u16; 27];

    decode_u16(&encoded, &mut decoded);

    assert_eq!(decoded, vec![1, 1, 2, 2, 2, 3]);
}

#[test]
fn test_decode_u32_rle() {
    let encoded: [u8; 15] = [1, 0, 0, 0, 2, 2, 0, 0, 0, 3, 3, 0, 0, 0, 1];
    let mut decoded: Vec<u32> = vec![0u32; 27];

    decode_u32(&encoded, &mut decoded);

    assert_eq!(decoded, vec![1, 1, 2, 2, 2, 3]);
}

#[test]
fn test_decode_f32_rle() {
    let encoded: [u8; 15] = [
        0, 0, 128, 63, 2, // 1.0, count 2
        0, 0, 0, 64, 3,   // 2.0, count 3
        0, 0, 64, 64, 1   // 3.0, count 1
    ];
    let mut decoded: Vec<f32> = vec![0f32; 27];

    decode_f32(&encoded, &mut decoded);

    assert_eq!(decoded, vec![1.0, 1.0, 2.0, 2.0, 2.0, 3.0]);
}

#[test]
fn test_decode_f64_rle() {
    let encoded: [u8; 27] = [
        0, 0, 0, 0, 0, 0, 240, 63, 2, // 1.0, count 2
        0, 0, 0, 0, 0, 0, 0, 64, 3,   // 2.0, count 3
        0, 0, 0, 0, 0, 0, 8, 64, 1    // 3.0, count 1
    ];
    let mut decoded: Vec<f64> = vec![0f64; 27];

    decode_f64(&encoded, &mut decoded);

    assert_eq!(decoded, vec![1.0, 1.0, 2.0, 2.0, 2.0, 3.0]);
}
