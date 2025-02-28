use delta::decode;

#[test]
#[cfg_attr(not(feature = "scalar"), ignore)]
fn test_decode_delta_default() {
    let input = [1, 1, 1, 3, 1, 1, 2, 2, 3];
    let mut decoded = vec![0; 9];

    decode(&input, &mut decoded.as_mut_slice());

    assert_eq!(decoded, [1, 2, 3, 6, 7, 8, 10, 12, 15]);
}
