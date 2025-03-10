extern crate core;

use delta::{encode, decode};

#[test]
fn test_encode_decode_delta() {
    let input = [1, 1, 1, 3, 1, 1, 2, 2, 3];
    let mut encoded = vec![0; 9];
    let mut decoded = vec![0; 9];

    encode(&input, &mut encoded.as_mut_slice());
    decode(&encoded.as_slice(), &mut decoded.as_mut_slice());

    assert_eq!(decoded, input);
}
