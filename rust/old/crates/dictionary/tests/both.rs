
use dictionary::{encode, decode};

#[test]
fn test_encode_decode_dictionary() {
    let input = String::from("USA,USA,USA,USA,Mexico,Canada,Mexico,Mexico,Mexico,Argentina");

    let encoded = encode(&input);
    let decoded = decode(&encoded);

    assert_eq!(input, decoded);
}
