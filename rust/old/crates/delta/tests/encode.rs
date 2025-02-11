use delta::encode;

#[test]
#[cfg_attr(not(feature = "scalar"), ignore)]
fn test_encode_delta_default() {
    let input = [1, 2, 3, 6, 7, 8, 10, 12, 15];
    let mut encoded = vec![0; 9];

    encode(&input, &mut encoded.as_mut_slice());

    assert_eq!(encoded, [1, 1, 1, 3, 1, 1, 2, 2, 3]);
}

#[test]
#[cfg_attr(not(feature = "SIMDx2"), ignore)]
fn test_encode_delta_simdx2() {
    let input = [1, 2, 3, 6, 7, 8, 10, 12, 15];
    let mut encoded = vec![0; 9];

    encode(&input, &mut encoded.as_mut_slice());

    assert_eq!(encoded, [1, 2, 2, 4, 4, 2, 3, 4, 5]);
}

#[test]
#[cfg_attr(not(feature = "SIMDx4"), ignore)]
fn test_encode_delta_simdx4() {
    let input = [1, 2, 3, 6, 7, 8, 10, 12, 15];
    let mut encoded = vec![0; 9];

    encode(&input, &mut encoded.as_mut_slice());

    assert_eq!(encoded, [1, 2, 3, 6, 6, 6, 7, 6, 8]);
}

#[test]
#[cfg_attr(not(feature = "SIMDx8"), ignore)]
fn test_encode_delta_simdx8() {
    let input = [1, 2, 3, 6, 7, 8, 10, 12, 15];
    let mut encoded = vec![0; 9];

    encode(&input, &mut encoded.as_mut_slice());

    assert_eq!(encoded, [1, 2, 3, 6, 7, 8, 10, 12, 14]);
}
