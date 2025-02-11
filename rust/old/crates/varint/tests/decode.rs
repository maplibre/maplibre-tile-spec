use varint::VarInt;

#[test]
fn test_decode_single_byte() {
    let encoded = [172, 2];

    let result = VarInt::decode_single(&encoded);

    assert_eq!(result, 300);
}

#[test]
fn test_decode_bytes() {
    let encoded = [172, 2];
    let mut buffer = [0; 1];
    let decoded = [300];

    let result = VarInt::decode(&encoded, &mut buffer);

    assert_eq!(result, 1);
    assert_eq!(buffer, decoded);
}
