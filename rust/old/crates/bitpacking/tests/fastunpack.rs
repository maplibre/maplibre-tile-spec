use bitpacking::BitPacking;

static TEST_DATA: [u32; 32] = [
    1506, 468, 3129, 2824, 1715, 3459, 448, 1685, 242, 3189, 1405, 1689, 2603, 1459, 2860, 2397,
    4019, 823, 464, 123, 2422, 1142, 1492, 3915, 2152, 2890, 662, 2045, 3823, 739, 3650, 326
];

// First 11 Tests should panic as the largest number in `TEST_DATA` is 4019 (which is 12 bit big)

#[test]
fn fastunpack0() {
    let mut output: [u32; 32] = [0; 32];
    let temp: [u32; 32] = [0; 32];

    BitPacking::fastunpack(&temp, &mut output, 1);

    assert_eq!(temp, output);
}

#[test]
#[should_panic]
fn fastunpack1() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 1);
    BitPacking::fastunpack(&temp, &mut output, 1);

    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack2() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 2);
    BitPacking::fastunpack(&temp, &mut output, 2);

    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack3() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 3);
    BitPacking::fastunpack(&temp, &mut output, 3);

    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack4() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 4);
    BitPacking::fastunpack(&temp, &mut output, 4);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack5() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 5);
    BitPacking::fastunpack(&temp, &mut output, 5);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack6() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 6);
    BitPacking::fastunpack(&temp, &mut output, 6);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack7() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 7);
    BitPacking::fastunpack(&temp, &mut output, 7);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack8() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 8);
    BitPacking::fastunpack(&temp, &mut output, 8);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack9() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 9);
    BitPacking::fastunpack(&temp, &mut output, 9);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack10() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 10);
    BitPacking::fastunpack(&temp, &mut output, 10);


    assert_eq!(output, TEST_DATA);
}

#[test]
#[should_panic]
fn fastunpack11() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 11);
    BitPacking::fastunpack(&temp, &mut output, 11);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack12() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 12);
    BitPacking::fastunpack(&temp, &mut output, 12);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack13() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 13);
    BitPacking::fastunpack(&temp, &mut output, 13);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack14() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 14);
    BitPacking::fastunpack(&temp, &mut output, 14);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack15() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 15);
    BitPacking::fastunpack(&temp, &mut output, 15);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack16() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 16);
    BitPacking::fastunpack(&temp, &mut output, 16);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack17() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 17);
    BitPacking::fastunpack(&temp, &mut output, 17);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack18() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 18);
    BitPacking::fastunpack(&temp, &mut output, 18);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack19() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 19);
    BitPacking::fastunpack(&temp, &mut output, 19);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack20() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 20);
    BitPacking::fastunpack(&temp, &mut output, 20);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack21() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 21);
    BitPacking::fastunpack(&temp, &mut output, 21);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack22() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 22);
    BitPacking::fastunpack(&temp, &mut output, 22);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack23() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 23);
    BitPacking::fastunpack(&temp, &mut output, 23);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack24() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 24);
    BitPacking::fastunpack(&temp, &mut output, 24);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack25() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 25);
    BitPacking::fastunpack(&temp, &mut output, 25);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack26() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 26);
    BitPacking::fastunpack(&temp, &mut output, 26);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack27() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 27);
    BitPacking::fastunpack(&temp, &mut output, 27);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack28() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 28);
    BitPacking::fastunpack(&temp, &mut output, 28);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack29() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 29);
    BitPacking::fastunpack(&temp, &mut output, 29);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack30() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 30);
    BitPacking::fastunpack(&temp, &mut output, 30);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack31() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 31);
    BitPacking::fastunpack(&temp, &mut output, 31);


    assert_eq!(output, TEST_DATA);
}

#[test]
fn fastunpack32() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    BitPacking::fastpack(&TEST_DATA, &mut temp, 32);
    BitPacking::fastunpack(&temp, &mut output, 32);


    assert_eq!(output, TEST_DATA);
    assert_eq!(output, TEST_DATA); // Equals
}

#[test]
#[should_panic]
fn fastunpack33() {
    let mut output: [u32; 32] = [0; 32];
    let mut temp: [u32; 32] = [0; 32];

    // 33 bitwidth is not supported, please panic
    BitPacking::fastpack(&TEST_DATA, &mut temp, 33);
    BitPacking::fastunpack(&temp, &mut output, 33);

}
