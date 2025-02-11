
use varint::VarInt;

#[test]
fn test_measure_single_byte() {
    assert_eq!(VarInt::length(300), 2);
}
