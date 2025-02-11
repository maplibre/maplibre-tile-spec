/// Pack the 32 integers without using a mask
pub fn fastpackwithoutmask(
    in_array: &[u32],
    inpos: usize,
    out_array: &mut [u32],
    outpos: usize,
    bit: u8,
) {
    match bit {
        0 => fastpackwithoutmask0(in_array, inpos, out_array, outpos),
        1 => fastpackwithoutmask1(in_array, inpos, out_array, outpos),
        2 => fastpackwithoutmask2(in_array, inpos, out_array, outpos),
        3 => fastpackwithoutmask3(in_array, inpos, out_array, outpos),
        4 => fastpackwithoutmask4(in_array, inpos, out_array, outpos),
        5 => fastpackwithoutmask5(in_array, inpos, out_array, outpos),
        6 => fastpackwithoutmask6(in_array, inpos, out_array, outpos),
        7 => fastpackwithoutmask7(in_array, inpos, out_array, outpos),
        8 => fastpackwithoutmask8(in_array, inpos, out_array, outpos),
        9 => fastpackwithoutmask9(in_array, inpos, out_array, outpos),
        10 => fastpackwithoutmask10(in_array, inpos, out_array, outpos),
        11 => fastpackwithoutmask11(in_array, inpos, out_array, outpos),
        12 => fastpackwithoutmask12(in_array, inpos, out_array, outpos),
        13 => fastpackwithoutmask13(in_array, inpos, out_array, outpos),
        14 => fastpackwithoutmask14(in_array, inpos, out_array, outpos),
        15 => fastpackwithoutmask15(in_array, inpos, out_array, outpos),
        16 => fastpackwithoutmask16(in_array, inpos, out_array, outpos),
        17 => fastpackwithoutmask17(in_array, inpos, out_array, outpos),
        18 => fastpackwithoutmask18(in_array, inpos, out_array, outpos),
        19 => fastpackwithoutmask19(in_array, inpos, out_array, outpos),
        20 => fastpackwithoutmask20(in_array, inpos, out_array, outpos),
        21 => fastpackwithoutmask21(in_array, inpos, out_array, outpos),
        22 => fastpackwithoutmask22(in_array, inpos, out_array, outpos),
        23 => fastpackwithoutmask23(in_array, inpos, out_array, outpos),
        24 => fastpackwithoutmask24(in_array, inpos, out_array, outpos),
        25 => fastpackwithoutmask25(in_array, inpos, out_array, outpos),
        26 => fastpackwithoutmask26(in_array, inpos, out_array, outpos),
        27 => fastpackwithoutmask27(in_array, inpos, out_array, outpos),
        28 => fastpackwithoutmask28(in_array, inpos, out_array, outpos),
        29 => fastpackwithoutmask29(in_array, inpos, out_array, outpos),
        30 => fastpackwithoutmask30(in_array, inpos, out_array, outpos),
        31 => fastpackwithoutmask31(in_array, inpos, out_array, outpos),
        32 => fastpackwithoutmask32(in_array, inpos, out_array, outpos),
        _ => panic!("Unsupported bit width."),
    }
}

pub fn fastpackwithoutmask0(
    _in_array: &[u32],
    _inpos: usize,
    _out_array: &mut [u32],
    _outpos: usize,
) {
    // nothing
}

pub fn fastpackwithoutmask1(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 1)
        | ((in_array[2 + inpos]) << 2)
        | ((in_array[3 + inpos]) << 3)
        | ((in_array[4 + inpos]) << 4)
        | ((in_array[5 + inpos]) << 5)
        | ((in_array[6 + inpos]) << 6)
        | ((in_array[7 + inpos]) << 7)
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 9)
        | ((in_array[10 + inpos]) << 10)
        | ((in_array[11 + inpos]) << 11)
        | ((in_array[12 + inpos]) << 12)
        | ((in_array[13 + inpos]) << 13)
        | ((in_array[14 + inpos]) << 14)
        | ((in_array[15 + inpos]) << 15)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 17)
        | ((in_array[18 + inpos]) << 18)
        | ((in_array[19 + inpos]) << 19)
        | ((in_array[20 + inpos]) << 20)
        | ((in_array[21 + inpos]) << 21)
        | ((in_array[22 + inpos]) << 22)
        | ((in_array[23 + inpos]) << 23)
        | ((in_array[24 + inpos]) << 24)
        | ((in_array[25 + inpos]) << 25)
        | ((in_array[26 + inpos]) << 26)
        | ((in_array[27 + inpos]) << 27)
        | ((in_array[28 + inpos]) << 28)
        | ((in_array[29 + inpos]) << 29)
        | ((in_array[30 + inpos]) << 30)
        | ((in_array[31 + inpos]) << 31);
}

pub fn fastpackwithoutmask10(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 10)
        | ((in_array[2 + inpos]) << 20)
        | ((in_array[3 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[3 + inpos]) >> (10 - 8))
        | ((in_array[4 + inpos]) << 8)
        | ((in_array[5 + inpos]) << 18)
        | ((in_array[6 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[6 + inpos]) >> (10 - 6))
        | ((in_array[7 + inpos]) << 6)
        | ((in_array[8 + inpos]) << 16)
        | ((in_array[9 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[9 + inpos]) >> (10 - 4))
        | ((in_array[10 + inpos]) << 4)
        | ((in_array[11 + inpos]) << 14)
        | ((in_array[12 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[12 + inpos]) >> (10 - 2))
        | ((in_array[13 + inpos]) << 2)
        | ((in_array[14 + inpos]) << 12)
        | ((in_array[15 + inpos]) << 22);
    out_array[5 + outpos] = in_array[16 + inpos]
        | ((in_array[17 + inpos]) << 10)
        | ((in_array[18 + inpos]) << 20)
        | ((in_array[19 + inpos]) << 30);
    out_array[6 + outpos] = ((in_array[19 + inpos]) >> (10 - 8))
        | ((in_array[20 + inpos]) << 8)
        | ((in_array[21 + inpos]) << 18)
        | ((in_array[22 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[22 + inpos]) >> (10 - 6))
        | ((in_array[23 + inpos]) << 6)
        | ((in_array[24 + inpos]) << 16)
        | ((in_array[25 + inpos]) << 26);
    out_array[8 + outpos] = ((in_array[25 + inpos]) >> (10 - 4))
        | ((in_array[26 + inpos]) << 4)
        | ((in_array[27 + inpos]) << 14)
        | ((in_array[28 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[28 + inpos]) >> (10 - 2))
        | ((in_array[29 + inpos]) << 2)
        | ((in_array[30 + inpos]) << 12)
        | ((in_array[31 + inpos]) << 22);
}

pub fn fastpackwithoutmask11(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 11) | ((in_array[2 + inpos]) << 22);
    out_array[1 + outpos] = ((in_array[2 + inpos]) >> (11 - 1))
        | ((in_array[3 + inpos]) << 1)
        | ((in_array[4 + inpos]) << 12)
        | ((in_array[5 + inpos]) << 23);
    out_array[2 + outpos] = ((in_array[5 + inpos]) >> (11 - 2))
        | ((in_array[6 + inpos]) << 2)
        | ((in_array[7 + inpos]) << 13)
        | ((in_array[8 + inpos]) << 24);
    out_array[3 + outpos] = ((in_array[8 + inpos]) >> (11 - 3))
        | ((in_array[9 + inpos]) << 3)
        | ((in_array[10 + inpos]) << 14)
        | ((in_array[11 + inpos]) << 25);
    out_array[4 + outpos] = ((in_array[11 + inpos]) >> (11 - 4))
        | ((in_array[12 + inpos]) << 4)
        | ((in_array[13 + inpos]) << 15)
        | ((in_array[14 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[14 + inpos]) >> (11 - 5))
        | ((in_array[15 + inpos]) << 5)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 27);
    out_array[6 + outpos] = ((in_array[17 + inpos]) >> (11 - 6))
        | ((in_array[18 + inpos]) << 6)
        | ((in_array[19 + inpos]) << 17)
        | ((in_array[20 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[20 + inpos]) >> (11 - 7))
        | ((in_array[21 + inpos]) << 7)
        | ((in_array[22 + inpos]) << 18)
        | ((in_array[23 + inpos]) << 29);
    out_array[8 + outpos] = ((in_array[23 + inpos]) >> (11 - 8))
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 19)
        | ((in_array[26 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[26 + inpos]) >> (11 - 9))
        | ((in_array[27 + inpos]) << 9)
        | ((in_array[28 + inpos]) << 20)
        | ((in_array[29 + inpos]) << 31);
    out_array[10 + outpos] = ((in_array[29 + inpos]) >> (11 - 10))
        | ((in_array[30 + inpos]) << 10)
        | ((in_array[31 + inpos]) << 21);
}

pub fn fastpackwithoutmask12(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 12) | ((in_array[2 + inpos]) << 24);
    out_array[1 + outpos] = ((in_array[2 + inpos]) >> (12 - 4))
        | ((in_array[3 + inpos]) << 4)
        | ((in_array[4 + inpos]) << 16)
        | ((in_array[5 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[5 + inpos]) >> (12 - 8))
        | ((in_array[6 + inpos]) << 8)
        | ((in_array[7 + inpos]) << 20);
    out_array[3 + outpos] =
        in_array[8 + inpos] | ((in_array[9 + inpos]) << 12) | ((in_array[10 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[10 + inpos]) >> (12 - 4))
        | ((in_array[11 + inpos]) << 4)
        | ((in_array[12 + inpos]) << 16)
        | ((in_array[13 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[13 + inpos]) >> (12 - 8))
        | ((in_array[14 + inpos]) << 8)
        | ((in_array[15 + inpos]) << 20);
    out_array[6 + outpos] =
        in_array[16 + inpos] | ((in_array[17 + inpos]) << 12) | ((in_array[18 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[18 + inpos]) >> (12 - 4))
        | ((in_array[19 + inpos]) << 4)
        | ((in_array[20 + inpos]) << 16)
        | ((in_array[21 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[21 + inpos]) >> (12 - 8))
        | ((in_array[22 + inpos]) << 8)
        | ((in_array[23 + inpos]) << 20);
    out_array[9 + outpos] =
        in_array[24 + inpos] | ((in_array[25 + inpos]) << 12) | ((in_array[26 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[26 + inpos]) >> (12 - 4))
        | ((in_array[27 + inpos]) << 4)
        | ((in_array[28 + inpos]) << 16)
        | ((in_array[29 + inpos]) << 28);
    out_array[11 + outpos] = ((in_array[29 + inpos]) >> (12 - 8))
        | ((in_array[30 + inpos]) << 8)
        | ((in_array[31 + inpos]) << 20);
}

pub fn fastpackwithoutmask13(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 13) | ((in_array[2 + inpos]) << 26);
    out_array[1 + outpos] = ((in_array[2 + inpos]) >> (13 - 7))
        | ((in_array[3 + inpos]) << 7)
        | ((in_array[4 + inpos]) << 20);
    out_array[2 + outpos] = ((in_array[4 + inpos]) >> (13 - 1))
        | ((in_array[5 + inpos]) << 1)
        | ((in_array[6 + inpos]) << 14)
        | ((in_array[7 + inpos]) << 27);
    out_array[3 + outpos] = ((in_array[7 + inpos]) >> (13 - 8))
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 21);
    out_array[4 + outpos] = ((in_array[9 + inpos]) >> (13 - 2))
        | ((in_array[10 + inpos]) << 2)
        | ((in_array[11 + inpos]) << 15)
        | ((in_array[12 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[12 + inpos]) >> (13 - 9))
        | ((in_array[13 + inpos]) << 9)
        | ((in_array[14 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[14 + inpos]) >> (13 - 3))
        | ((in_array[15 + inpos]) << 3)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 29);
    out_array[7 + outpos] = ((in_array[17 + inpos]) >> (13 - 10))
        | ((in_array[18 + inpos]) << 10)
        | ((in_array[19 + inpos]) << 23);
    out_array[8 + outpos] = ((in_array[19 + inpos]) >> (13 - 4))
        | ((in_array[20 + inpos]) << 4)
        | ((in_array[21 + inpos]) << 17)
        | ((in_array[22 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[22 + inpos]) >> (13 - 11))
        | ((in_array[23 + inpos]) << 11)
        | ((in_array[24 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[24 + inpos]) >> (13 - 5))
        | ((in_array[25 + inpos]) << 5)
        | ((in_array[26 + inpos]) << 18)
        | ((in_array[27 + inpos]) << 31);
    out_array[11 + outpos] = ((in_array[27 + inpos]) >> (13 - 12))
        | ((in_array[28 + inpos]) << 12)
        | ((in_array[29 + inpos]) << 25);
    out_array[12 + outpos] = ((in_array[29 + inpos]) >> (13 - 6))
        | ((in_array[30 + inpos]) << 6)
        | ((in_array[31 + inpos]) << 19);
}

pub fn fastpackwithoutmask14(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 14) | ((in_array[2 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[2 + inpos]) >> (14 - 10))
        | ((in_array[3 + inpos]) << 10)
        | ((in_array[4 + inpos]) << 24);
    out_array[2 + outpos] = ((in_array[4 + inpos]) >> (14 - 6))
        | ((in_array[5 + inpos]) << 6)
        | ((in_array[6 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[6 + inpos]) >> (14 - 2))
        | ((in_array[7 + inpos]) << 2)
        | ((in_array[8 + inpos]) << 16)
        | ((in_array[9 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[9 + inpos]) >> (14 - 12))
        | ((in_array[10 + inpos]) << 12)
        | ((in_array[11 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[11 + inpos]) >> (14 - 8))
        | ((in_array[12 + inpos]) << 8)
        | ((in_array[13 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[13 + inpos]) >> (14 - 4))
        | ((in_array[14 + inpos]) << 4)
        | ((in_array[15 + inpos]) << 18);
    out_array[7 + outpos] =
        in_array[16 + inpos] | ((in_array[17 + inpos]) << 14) | ((in_array[18 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[18 + inpos]) >> (14 - 10))
        | ((in_array[19 + inpos]) << 10)
        | ((in_array[20 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[20 + inpos]) >> (14 - 6))
        | ((in_array[21 + inpos]) << 6)
        | ((in_array[22 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[22 + inpos]) >> (14 - 2))
        | ((in_array[23 + inpos]) << 2)
        | ((in_array[24 + inpos]) << 16)
        | ((in_array[25 + inpos]) << 30);
    out_array[11 + outpos] = ((in_array[25 + inpos]) >> (14 - 12))
        | ((in_array[26 + inpos]) << 12)
        | ((in_array[27 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[27 + inpos]) >> (14 - 8))
        | ((in_array[28 + inpos]) << 8)
        | ((in_array[29 + inpos]) << 22);
    out_array[13 + outpos] = ((in_array[29 + inpos]) >> (14 - 4))
        | ((in_array[30 + inpos]) << 4)
        | ((in_array[31 + inpos]) << 18);
}

pub fn fastpackwithoutmask15(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 15) | ((in_array[2 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[2 + inpos]) >> (15 - 13))
        | ((in_array[3 + inpos]) << 13)
        | ((in_array[4 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[4 + inpos]) >> (15 - 11))
        | ((in_array[5 + inpos]) << 11)
        | ((in_array[6 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[6 + inpos]) >> (15 - 9))
        | ((in_array[7 + inpos]) << 9)
        | ((in_array[8 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[8 + inpos]) >> (15 - 7))
        | ((in_array[9 + inpos]) << 7)
        | ((in_array[10 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[10 + inpos]) >> (15 - 5))
        | ((in_array[11 + inpos]) << 5)
        | ((in_array[12 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[12 + inpos]) >> (15 - 3))
        | ((in_array[13 + inpos]) << 3)
        | ((in_array[14 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[14 + inpos]) >> (15 - 1))
        | ((in_array[15 + inpos]) << 1)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 31);
    out_array[8 + outpos] = ((in_array[17 + inpos]) >> (15 - 14))
        | ((in_array[18 + inpos]) << 14)
        | ((in_array[19 + inpos]) << 29);
    out_array[9 + outpos] = ((in_array[19 + inpos]) >> (15 - 12))
        | ((in_array[20 + inpos]) << 12)
        | ((in_array[21 + inpos]) << 27);
    out_array[10 + outpos] = ((in_array[21 + inpos]) >> (15 - 10))
        | ((in_array[22 + inpos]) << 10)
        | ((in_array[23 + inpos]) << 25);
    out_array[11 + outpos] = ((in_array[23 + inpos]) >> (15 - 8))
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 23);
    out_array[12 + outpos] = ((in_array[25 + inpos]) >> (15 - 6))
        | ((in_array[26 + inpos]) << 6)
        | ((in_array[27 + inpos]) << 21);
    out_array[13 + outpos] = ((in_array[27 + inpos]) >> (15 - 4))
        | ((in_array[28 + inpos]) << 4)
        | ((in_array[29 + inpos]) << 19);
    out_array[14 + outpos] = ((in_array[29 + inpos]) >> (15 - 2))
        | ((in_array[30 + inpos]) << 2)
        | ((in_array[31 + inpos]) << 17);
}

pub fn fastpackwithoutmask16(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 16);
    out_array[1 + outpos] = in_array[2 + inpos] | ((in_array[3 + inpos]) << 16);
    out_array[2 + outpos] = in_array[4 + inpos] | ((in_array[5 + inpos]) << 16);
    out_array[3 + outpos] = in_array[6 + inpos] | ((in_array[7 + inpos]) << 16);
    out_array[4 + outpos] = in_array[8 + inpos] | ((in_array[9 + inpos]) << 16);
    out_array[5 + outpos] = in_array[10 + inpos] | ((in_array[11 + inpos]) << 16);
    out_array[6 + outpos] = in_array[12 + inpos] | ((in_array[13 + inpos]) << 16);
    out_array[7 + outpos] = in_array[14 + inpos] | ((in_array[15 + inpos]) << 16);
    out_array[8 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 16);
    out_array[9 + outpos] = in_array[18 + inpos] | ((in_array[19 + inpos]) << 16);
    out_array[10 + outpos] = in_array[20 + inpos] | ((in_array[21 + inpos]) << 16);
    out_array[11 + outpos] = in_array[22 + inpos] | ((in_array[23 + inpos]) << 16);
    out_array[12 + outpos] = in_array[24 + inpos] | ((in_array[25 + inpos]) << 16);
    out_array[13 + outpos] = in_array[26 + inpos] | ((in_array[27 + inpos]) << 16);
    out_array[14 + outpos] = in_array[28 + inpos] | ((in_array[29 + inpos]) << 16);
    out_array[15 + outpos] = in_array[30 + inpos] | ((in_array[31 + inpos]) << 16);
}

pub fn fastpackwithoutmask17(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 17);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (17 - 2))
        | ((in_array[2 + inpos]) << 2)
        | ((in_array[3 + inpos]) << 19);
    out_array[2 + outpos] = ((in_array[3 + inpos]) >> (17 - 4))
        | ((in_array[4 + inpos]) << 4)
        | ((in_array[5 + inpos]) << 21);
    out_array[3 + outpos] = ((in_array[5 + inpos]) >> (17 - 6))
        | ((in_array[6 + inpos]) << 6)
        | ((in_array[7 + inpos]) << 23);
    out_array[4 + outpos] = ((in_array[7 + inpos]) >> (17 - 8))
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 25);
    out_array[5 + outpos] = ((in_array[9 + inpos]) >> (17 - 10))
        | ((in_array[10 + inpos]) << 10)
        | ((in_array[11 + inpos]) << 27);
    out_array[6 + outpos] = ((in_array[11 + inpos]) >> (17 - 12))
        | ((in_array[12 + inpos]) << 12)
        | ((in_array[13 + inpos]) << 29);
    out_array[7 + outpos] = ((in_array[13 + inpos]) >> (17 - 14))
        | ((in_array[14 + inpos]) << 14)
        | ((in_array[15 + inpos]) << 31);
    out_array[8 + outpos] = ((in_array[15 + inpos]) >> (17 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[9 + outpos] = ((in_array[16 + inpos]) >> (17 - 1))
        | ((in_array[17 + inpos]) << 1)
        | ((in_array[18 + inpos]) << 18);
    out_array[10 + outpos] = ((in_array[18 + inpos]) >> (17 - 3))
        | ((in_array[19 + inpos]) << 3)
        | ((in_array[20 + inpos]) << 20);
    out_array[11 + outpos] = ((in_array[20 + inpos]) >> (17 - 5))
        | ((in_array[21 + inpos]) << 5)
        | ((in_array[22 + inpos]) << 22);
    out_array[12 + outpos] = ((in_array[22 + inpos]) >> (17 - 7))
        | ((in_array[23 + inpos]) << 7)
        | ((in_array[24 + inpos]) << 24);
    out_array[13 + outpos] = ((in_array[24 + inpos]) >> (17 - 9))
        | ((in_array[25 + inpos]) << 9)
        | ((in_array[26 + inpos]) << 26);
    out_array[14 + outpos] = ((in_array[26 + inpos]) >> (17 - 11))
        | ((in_array[27 + inpos]) << 11)
        | ((in_array[28 + inpos]) << 28);
    out_array[15 + outpos] = ((in_array[28 + inpos]) >> (17 - 13))
        | ((in_array[29 + inpos]) << 13)
        | ((in_array[30 + inpos]) << 30);
    out_array[16 + outpos] = ((in_array[30 + inpos]) >> (17 - 15)) | ((in_array[31 + inpos]) << 15);
}

pub fn fastpackwithoutmask18(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 18);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (18 - 4))
        | ((in_array[2 + inpos]) << 4)
        | ((in_array[3 + inpos]) << 22);
    out_array[2 + outpos] = ((in_array[3 + inpos]) >> (18 - 8))
        | ((in_array[4 + inpos]) << 8)
        | ((in_array[5 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[5 + inpos]) >> (18 - 12))
        | ((in_array[6 + inpos]) << 12)
        | ((in_array[7 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[7 + inpos]) >> (18 - 16)) | ((in_array[8 + inpos]) << 16);
    out_array[5 + outpos] = ((in_array[8 + inpos]) >> (18 - 2))
        | ((in_array[9 + inpos]) << 2)
        | ((in_array[10 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[10 + inpos]) >> (18 - 6))
        | ((in_array[11 + inpos]) << 6)
        | ((in_array[12 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[12 + inpos]) >> (18 - 10))
        | ((in_array[13 + inpos]) << 10)
        | ((in_array[14 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[14 + inpos]) >> (18 - 14)) | ((in_array[15 + inpos]) << 14);
    out_array[9 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 18);
    out_array[10 + outpos] = ((in_array[17 + inpos]) >> (18 - 4))
        | ((in_array[18 + inpos]) << 4)
        | ((in_array[19 + inpos]) << 22);
    out_array[11 + outpos] = ((in_array[19 + inpos]) >> (18 - 8))
        | ((in_array[20 + inpos]) << 8)
        | ((in_array[21 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[21 + inpos]) >> (18 - 12))
        | ((in_array[22 + inpos]) << 12)
        | ((in_array[23 + inpos]) << 30);
    out_array[13 + outpos] = ((in_array[23 + inpos]) >> (18 - 16)) | ((in_array[24 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[24 + inpos]) >> (18 - 2))
        | ((in_array[25 + inpos]) << 2)
        | ((in_array[26 + inpos]) << 20);
    out_array[15 + outpos] = ((in_array[26 + inpos]) >> (18 - 6))
        | ((in_array[27 + inpos]) << 6)
        | ((in_array[28 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[28 + inpos]) >> (18 - 10))
        | ((in_array[29 + inpos]) << 10)
        | ((in_array[30 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[30 + inpos]) >> (18 - 14)) | ((in_array[31 + inpos]) << 14);
}

pub fn fastpackwithoutmask19(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 19);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (19 - 6))
        | ((in_array[2 + inpos]) << 6)
        | ((in_array[3 + inpos]) << 25);
    out_array[2 + outpos] = ((in_array[3 + inpos]) >> (19 - 12))
        | ((in_array[4 + inpos]) << 12)
        | ((in_array[5 + inpos]) << 31);
    out_array[3 + outpos] = ((in_array[5 + inpos]) >> (19 - 18)) | ((in_array[6 + inpos]) << 18);
    out_array[4 + outpos] = ((in_array[6 + inpos]) >> (19 - 5))
        | ((in_array[7 + inpos]) << 5)
        | ((in_array[8 + inpos]) << 24);
    out_array[5 + outpos] = ((in_array[8 + inpos]) >> (19 - 11))
        | ((in_array[9 + inpos]) << 11)
        | ((in_array[10 + inpos]) << 30);
    out_array[6 + outpos] = ((in_array[10 + inpos]) >> (19 - 17)) | ((in_array[11 + inpos]) << 17);
    out_array[7 + outpos] = ((in_array[11 + inpos]) >> (19 - 4))
        | ((in_array[12 + inpos]) << 4)
        | ((in_array[13 + inpos]) << 23);
    out_array[8 + outpos] = ((in_array[13 + inpos]) >> (19 - 10))
        | ((in_array[14 + inpos]) << 10)
        | ((in_array[15 + inpos]) << 29);
    out_array[9 + outpos] = ((in_array[15 + inpos]) >> (19 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[10 + outpos] = ((in_array[16 + inpos]) >> (19 - 3))
        | ((in_array[17 + inpos]) << 3)
        | ((in_array[18 + inpos]) << 22);
    out_array[11 + outpos] = ((in_array[18 + inpos]) >> (19 - 9))
        | ((in_array[19 + inpos]) << 9)
        | ((in_array[20 + inpos]) << 28);
    out_array[12 + outpos] = ((in_array[20 + inpos]) >> (19 - 15)) | ((in_array[21 + inpos]) << 15);
    out_array[13 + outpos] = ((in_array[21 + inpos]) >> (19 - 2))
        | ((in_array[22 + inpos]) << 2)
        | ((in_array[23 + inpos]) << 21);
    out_array[14 + outpos] = ((in_array[23 + inpos]) >> (19 - 8))
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 27);
    out_array[15 + outpos] = ((in_array[25 + inpos]) >> (19 - 14)) | ((in_array[26 + inpos]) << 14);
    out_array[16 + outpos] = ((in_array[26 + inpos]) >> (19 - 1))
        | ((in_array[27 + inpos]) << 1)
        | ((in_array[28 + inpos]) << 20);
    out_array[17 + outpos] = ((in_array[28 + inpos]) >> (19 - 7))
        | ((in_array[29 + inpos]) << 7)
        | ((in_array[30 + inpos]) << 26);
    out_array[18 + outpos] = ((in_array[30 + inpos]) >> (19 - 13)) | ((in_array[31 + inpos]) << 13);
}

pub fn fastpackwithoutmask2(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 2)
        | ((in_array[2 + inpos]) << 4)
        | ((in_array[3 + inpos]) << 6)
        | ((in_array[4 + inpos]) << 8)
        | ((in_array[5 + inpos]) << 10)
        | ((in_array[6 + inpos]) << 12)
        | ((in_array[7 + inpos]) << 14)
        | ((in_array[8 + inpos]) << 16)
        | ((in_array[9 + inpos]) << 18)
        | ((in_array[10 + inpos]) << 20)
        | ((in_array[11 + inpos]) << 22)
        | ((in_array[12 + inpos]) << 24)
        | ((in_array[13 + inpos]) << 26)
        | ((in_array[14 + inpos]) << 28)
        | ((in_array[15 + inpos]) << 30);
    out_array[1 + outpos] = in_array[16 + inpos]
        | ((in_array[17 + inpos]) << 2)
        | ((in_array[18 + inpos]) << 4)
        | ((in_array[19 + inpos]) << 6)
        | ((in_array[20 + inpos]) << 8)
        | ((in_array[21 + inpos]) << 10)
        | ((in_array[22 + inpos]) << 12)
        | ((in_array[23 + inpos]) << 14)
        | ((in_array[24 + inpos]) << 16)
        | ((in_array[25 + inpos]) << 18)
        | ((in_array[26 + inpos]) << 20)
        | ((in_array[27 + inpos]) << 22)
        | ((in_array[28 + inpos]) << 24)
        | ((in_array[29 + inpos]) << 26)
        | ((in_array[30 + inpos]) << 28)
        | ((in_array[31 + inpos]) << 30);
}

pub fn fastpackwithoutmask20(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 20);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (20 - 8))
        | ((in_array[2 + inpos]) << 8)
        | ((in_array[3 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[3 + inpos]) >> (20 - 16)) | ((in_array[4 + inpos]) << 16);
    out_array[3 + outpos] = ((in_array[4 + inpos]) >> (20 - 4))
        | ((in_array[5 + inpos]) << 4)
        | ((in_array[6 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[6 + inpos]) >> (20 - 12)) | ((in_array[7 + inpos]) << 12);
    out_array[5 + outpos] = in_array[8 + inpos] | ((in_array[9 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[9 + inpos]) >> (20 - 8))
        | ((in_array[10 + inpos]) << 8)
        | ((in_array[11 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[11 + inpos]) >> (20 - 16)) | ((in_array[12 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[12 + inpos]) >> (20 - 4))
        | ((in_array[13 + inpos]) << 4)
        | ((in_array[14 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[14 + inpos]) >> (20 - 12)) | ((in_array[15 + inpos]) << 12);
    out_array[10 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 20);
    out_array[11 + outpos] = ((in_array[17 + inpos]) >> (20 - 8))
        | ((in_array[18 + inpos]) << 8)
        | ((in_array[19 + inpos]) << 28);
    out_array[12 + outpos] = ((in_array[19 + inpos]) >> (20 - 16)) | ((in_array[20 + inpos]) << 16);
    out_array[13 + outpos] = ((in_array[20 + inpos]) >> (20 - 4))
        | ((in_array[21 + inpos]) << 4)
        | ((in_array[22 + inpos]) << 24);
    out_array[14 + outpos] = ((in_array[22 + inpos]) >> (20 - 12)) | ((in_array[23 + inpos]) << 12);
    out_array[15 + outpos] = in_array[24 + inpos] | ((in_array[25 + inpos]) << 20);
    out_array[16 + outpos] = ((in_array[25 + inpos]) >> (20 - 8))
        | ((in_array[26 + inpos]) << 8)
        | ((in_array[27 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[27 + inpos]) >> (20 - 16)) | ((in_array[28 + inpos]) << 16);
    out_array[18 + outpos] = ((in_array[28 + inpos]) >> (20 - 4))
        | ((in_array[29 + inpos]) << 4)
        | ((in_array[30 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[30 + inpos]) >> (20 - 12)) | ((in_array[31 + inpos]) << 12);
}

pub fn fastpackwithoutmask21(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 21);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (21 - 10))
        | ((in_array[2 + inpos]) << 10)
        | ((in_array[3 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[3 + inpos]) >> (21 - 20)) | ((in_array[4 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[4 + inpos]) >> (21 - 9))
        | ((in_array[5 + inpos]) << 9)
        | ((in_array[6 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[6 + inpos]) >> (21 - 19)) | ((in_array[7 + inpos]) << 19);
    out_array[5 + outpos] = ((in_array[7 + inpos]) >> (21 - 8))
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[9 + inpos]) >> (21 - 18)) | ((in_array[10 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[10 + inpos]) >> (21 - 7))
        | ((in_array[11 + inpos]) << 7)
        | ((in_array[12 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[12 + inpos]) >> (21 - 17)) | ((in_array[13 + inpos]) << 17);
    out_array[9 + outpos] = ((in_array[13 + inpos]) >> (21 - 6))
        | ((in_array[14 + inpos]) << 6)
        | ((in_array[15 + inpos]) << 27);
    out_array[10 + outpos] = ((in_array[15 + inpos]) >> (21 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[16 + inpos]) >> (21 - 5))
        | ((in_array[17 + inpos]) << 5)
        | ((in_array[18 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[18 + inpos]) >> (21 - 15)) | ((in_array[19 + inpos]) << 15);
    out_array[13 + outpos] = ((in_array[19 + inpos]) >> (21 - 4))
        | ((in_array[20 + inpos]) << 4)
        | ((in_array[21 + inpos]) << 25);
    out_array[14 + outpos] = ((in_array[21 + inpos]) >> (21 - 14)) | ((in_array[22 + inpos]) << 14);
    out_array[15 + outpos] = ((in_array[22 + inpos]) >> (21 - 3))
        | ((in_array[23 + inpos]) << 3)
        | ((in_array[24 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[24 + inpos]) >> (21 - 13)) | ((in_array[25 + inpos]) << 13);
    out_array[17 + outpos] = ((in_array[25 + inpos]) >> (21 - 2))
        | ((in_array[26 + inpos]) << 2)
        | ((in_array[27 + inpos]) << 23);
    out_array[18 + outpos] = ((in_array[27 + inpos]) >> (21 - 12)) | ((in_array[28 + inpos]) << 12);
    out_array[19 + outpos] = ((in_array[28 + inpos]) >> (21 - 1))
        | ((in_array[29 + inpos]) << 1)
        | ((in_array[30 + inpos]) << 22);
    out_array[20 + outpos] = ((in_array[30 + inpos]) >> (21 - 11)) | ((in_array[31 + inpos]) << 11);
}

pub fn fastpackwithoutmask22(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 22);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (22 - 12)) | ((in_array[2 + inpos]) << 12);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (22 - 2))
        | ((in_array[3 + inpos]) << 2)
        | ((in_array[4 + inpos]) << 24);
    out_array[3 + outpos] = ((in_array[4 + inpos]) >> (22 - 14)) | ((in_array[5 + inpos]) << 14);
    out_array[4 + outpos] = ((in_array[5 + inpos]) >> (22 - 4))
        | ((in_array[6 + inpos]) << 4)
        | ((in_array[7 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[7 + inpos]) >> (22 - 16)) | ((in_array[8 + inpos]) << 16);
    out_array[6 + outpos] = ((in_array[8 + inpos]) >> (22 - 6))
        | ((in_array[9 + inpos]) << 6)
        | ((in_array[10 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[10 + inpos]) >> (22 - 18)) | ((in_array[11 + inpos]) << 18);
    out_array[8 + outpos] = ((in_array[11 + inpos]) >> (22 - 8))
        | ((in_array[12 + inpos]) << 8)
        | ((in_array[13 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[13 + inpos]) >> (22 - 20)) | ((in_array[14 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[14 + inpos]) >> (22 - 10)) | ((in_array[15 + inpos]) << 10);
    out_array[11 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 22);
    out_array[12 + outpos] = ((in_array[17 + inpos]) >> (22 - 12)) | ((in_array[18 + inpos]) << 12);
    out_array[13 + outpos] = ((in_array[18 + inpos]) >> (22 - 2))
        | ((in_array[19 + inpos]) << 2)
        | ((in_array[20 + inpos]) << 24);
    out_array[14 + outpos] = ((in_array[20 + inpos]) >> (22 - 14)) | ((in_array[21 + inpos]) << 14);
    out_array[15 + outpos] = ((in_array[21 + inpos]) >> (22 - 4))
        | ((in_array[22 + inpos]) << 4)
        | ((in_array[23 + inpos]) << 26);
    out_array[16 + outpos] = ((in_array[23 + inpos]) >> (22 - 16)) | ((in_array[24 + inpos]) << 16);
    out_array[17 + outpos] = ((in_array[24 + inpos]) >> (22 - 6))
        | ((in_array[25 + inpos]) << 6)
        | ((in_array[26 + inpos]) << 28);
    out_array[18 + outpos] = ((in_array[26 + inpos]) >> (22 - 18)) | ((in_array[27 + inpos]) << 18);
    out_array[19 + outpos] = ((in_array[27 + inpos]) >> (22 - 8))
        | ((in_array[28 + inpos]) << 8)
        | ((in_array[29 + inpos]) << 30);
    out_array[20 + outpos] = ((in_array[29 + inpos]) >> (22 - 20)) | ((in_array[30 + inpos]) << 20);
    out_array[21 + outpos] = ((in_array[30 + inpos]) >> (22 - 10)) | ((in_array[31 + inpos]) << 10);
}

pub fn fastpackwithoutmask23(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 23);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (23 - 14)) | ((in_array[2 + inpos]) << 14);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (23 - 5))
        | ((in_array[3 + inpos]) << 5)
        | ((in_array[4 + inpos]) << 28);
    out_array[3 + outpos] = ((in_array[4 + inpos]) >> (23 - 19)) | ((in_array[5 + inpos]) << 19);
    out_array[4 + outpos] = ((in_array[5 + inpos]) >> (23 - 10)) | ((in_array[6 + inpos]) << 10);
    out_array[5 + outpos] = ((in_array[6 + inpos]) >> (23 - 1))
        | ((in_array[7 + inpos]) << 1)
        | ((in_array[8 + inpos]) << 24);
    out_array[6 + outpos] = ((in_array[8 + inpos]) >> (23 - 15)) | ((in_array[9 + inpos]) << 15);
    out_array[7 + outpos] = ((in_array[9 + inpos]) >> (23 - 6))
        | ((in_array[10 + inpos]) << 6)
        | ((in_array[11 + inpos]) << 29);
    out_array[8 + outpos] = ((in_array[11 + inpos]) >> (23 - 20)) | ((in_array[12 + inpos]) << 20);
    out_array[9 + outpos] = ((in_array[12 + inpos]) >> (23 - 11)) | ((in_array[13 + inpos]) << 11);
    out_array[10 + outpos] = ((in_array[13 + inpos]) >> (23 - 2))
        | ((in_array[14 + inpos]) << 2)
        | ((in_array[15 + inpos]) << 25);
    out_array[11 + outpos] = ((in_array[15 + inpos]) >> (23 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[12 + outpos] = ((in_array[16 + inpos]) >> (23 - 7))
        | ((in_array[17 + inpos]) << 7)
        | ((in_array[18 + inpos]) << 30);
    out_array[13 + outpos] = ((in_array[18 + inpos]) >> (23 - 21)) | ((in_array[19 + inpos]) << 21);
    out_array[14 + outpos] = ((in_array[19 + inpos]) >> (23 - 12)) | ((in_array[20 + inpos]) << 12);
    out_array[15 + outpos] = ((in_array[20 + inpos]) >> (23 - 3))
        | ((in_array[21 + inpos]) << 3)
        | ((in_array[22 + inpos]) << 26);
    out_array[16 + outpos] = ((in_array[22 + inpos]) >> (23 - 17)) | ((in_array[23 + inpos]) << 17);
    out_array[17 + outpos] = ((in_array[23 + inpos]) >> (23 - 8))
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 31);
    out_array[18 + outpos] = ((in_array[25 + inpos]) >> (23 - 22)) | ((in_array[26 + inpos]) << 22);
    out_array[19 + outpos] = ((in_array[26 + inpos]) >> (23 - 13)) | ((in_array[27 + inpos]) << 13);
    out_array[20 + outpos] = ((in_array[27 + inpos]) >> (23 - 4))
        | ((in_array[28 + inpos]) << 4)
        | ((in_array[29 + inpos]) << 27);
    out_array[21 + outpos] = ((in_array[29 + inpos]) >> (23 - 18)) | ((in_array[30 + inpos]) << 18);
    out_array[22 + outpos] = ((in_array[30 + inpos]) >> (23 - 9)) | ((in_array[31 + inpos]) << 9);
}

pub fn fastpackwithoutmask24(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 24);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (24 - 16)) | ((in_array[2 + inpos]) << 16);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (24 - 8)) | ((in_array[3 + inpos]) << 8);
    out_array[3 + outpos] = in_array[4 + inpos] | ((in_array[5 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[5 + inpos]) >> (24 - 16)) | ((in_array[6 + inpos]) << 16);
    out_array[5 + outpos] = ((in_array[6 + inpos]) >> (24 - 8)) | ((in_array[7 + inpos]) << 8);
    out_array[6 + outpos] = in_array[8 + inpos] | ((in_array[9 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[9 + inpos]) >> (24 - 16)) | ((in_array[10 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[10 + inpos]) >> (24 - 8)) | ((in_array[11 + inpos]) << 8);
    out_array[9 + outpos] = in_array[12 + inpos] | ((in_array[13 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[13 + inpos]) >> (24 - 16)) | ((in_array[14 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[14 + inpos]) >> (24 - 8)) | ((in_array[15 + inpos]) << 8);
    out_array[12 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 24);
    out_array[13 + outpos] = ((in_array[17 + inpos]) >> (24 - 16)) | ((in_array[18 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[18 + inpos]) >> (24 - 8)) | ((in_array[19 + inpos]) << 8);
    out_array[15 + outpos] = in_array[20 + inpos] | ((in_array[21 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[21 + inpos]) >> (24 - 16)) | ((in_array[22 + inpos]) << 16);
    out_array[17 + outpos] = ((in_array[22 + inpos]) >> (24 - 8)) | ((in_array[23 + inpos]) << 8);
    out_array[18 + outpos] = in_array[24 + inpos] | ((in_array[25 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[25 + inpos]) >> (24 - 16)) | ((in_array[26 + inpos]) << 16);
    out_array[20 + outpos] = ((in_array[26 + inpos]) >> (24 - 8)) | ((in_array[27 + inpos]) << 8);
    out_array[21 + outpos] = in_array[28 + inpos] | ((in_array[29 + inpos]) << 24);
    out_array[22 + outpos] = ((in_array[29 + inpos]) >> (24 - 16)) | ((in_array[30 + inpos]) << 16);
    out_array[23 + outpos] = ((in_array[30 + inpos]) >> (24 - 8)) | ((in_array[31 + inpos]) << 8);
}

pub fn fastpackwithoutmask25(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 25);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (25 - 18)) | ((in_array[2 + inpos]) << 18);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (25 - 11)) | ((in_array[3 + inpos]) << 11);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (25 - 4))
        | ((in_array[4 + inpos]) << 4)
        | ((in_array[5 + inpos]) << 29);
    out_array[4 + outpos] = ((in_array[5 + inpos]) >> (25 - 22)) | ((in_array[6 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[6 + inpos]) >> (25 - 15)) | ((in_array[7 + inpos]) << 15);
    out_array[6 + outpos] = ((in_array[7 + inpos]) >> (25 - 8)) | ((in_array[8 + inpos]) << 8);
    out_array[7 + outpos] = ((in_array[8 + inpos]) >> (25 - 1))
        | ((in_array[9 + inpos]) << 1)
        | ((in_array[10 + inpos]) << 26);
    out_array[8 + outpos] = ((in_array[10 + inpos]) >> (25 - 19)) | ((in_array[11 + inpos]) << 19);
    out_array[9 + outpos] = ((in_array[11 + inpos]) >> (25 - 12)) | ((in_array[12 + inpos]) << 12);
    out_array[10 + outpos] = ((in_array[12 + inpos]) >> (25 - 5))
        | ((in_array[13 + inpos]) << 5)
        | ((in_array[14 + inpos]) << 30);
    out_array[11 + outpos] = ((in_array[14 + inpos]) >> (25 - 23)) | ((in_array[15 + inpos]) << 23);
    out_array[12 + outpos] = ((in_array[15 + inpos]) >> (25 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[13 + outpos] = ((in_array[16 + inpos]) >> (25 - 9)) | ((in_array[17 + inpos]) << 9);
    out_array[14 + outpos] = ((in_array[17 + inpos]) >> (25 - 2))
        | ((in_array[18 + inpos]) << 2)
        | ((in_array[19 + inpos]) << 27);
    out_array[15 + outpos] = ((in_array[19 + inpos]) >> (25 - 20)) | ((in_array[20 + inpos]) << 20);
    out_array[16 + outpos] = ((in_array[20 + inpos]) >> (25 - 13)) | ((in_array[21 + inpos]) << 13);
    out_array[17 + outpos] = ((in_array[21 + inpos]) >> (25 - 6))
        | ((in_array[22 + inpos]) << 6)
        | ((in_array[23 + inpos]) << 31);
    out_array[18 + outpos] = ((in_array[23 + inpos]) >> (25 - 24)) | ((in_array[24 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[24 + inpos]) >> (25 - 17)) | ((in_array[25 + inpos]) << 17);
    out_array[20 + outpos] = ((in_array[25 + inpos]) >> (25 - 10)) | ((in_array[26 + inpos]) << 10);
    out_array[21 + outpos] = ((in_array[26 + inpos]) >> (25 - 3))
        | ((in_array[27 + inpos]) << 3)
        | ((in_array[28 + inpos]) << 28);
    out_array[22 + outpos] = ((in_array[28 + inpos]) >> (25 - 21)) | ((in_array[29 + inpos]) << 21);
    out_array[23 + outpos] = ((in_array[29 + inpos]) >> (25 - 14)) | ((in_array[30 + inpos]) << 14);
    out_array[24 + outpos] = ((in_array[30 + inpos]) >> (25 - 7)) | ((in_array[31 + inpos]) << 7);
}

pub fn fastpackwithoutmask26(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 26);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (26 - 20)) | ((in_array[2 + inpos]) << 20);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (26 - 14)) | ((in_array[3 + inpos]) << 14);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (26 - 8)) | ((in_array[4 + inpos]) << 8);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (26 - 2))
        | ((in_array[5 + inpos]) << 2)
        | ((in_array[6 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[6 + inpos]) >> (26 - 22)) | ((in_array[7 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[7 + inpos]) >> (26 - 16)) | ((in_array[8 + inpos]) << 16);
    out_array[7 + outpos] = ((in_array[8 + inpos]) >> (26 - 10)) | ((in_array[9 + inpos]) << 10);
    out_array[8 + outpos] = ((in_array[9 + inpos]) >> (26 - 4))
        | ((in_array[10 + inpos]) << 4)
        | ((in_array[11 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[11 + inpos]) >> (26 - 24)) | ((in_array[12 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[12 + inpos]) >> (26 - 18)) | ((in_array[13 + inpos]) << 18);
    out_array[11 + outpos] = ((in_array[13 + inpos]) >> (26 - 12)) | ((in_array[14 + inpos]) << 12);
    out_array[12 + outpos] = ((in_array[14 + inpos]) >> (26 - 6)) | ((in_array[15 + inpos]) << 6);
    out_array[13 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 26);
    out_array[14 + outpos] = ((in_array[17 + inpos]) >> (26 - 20)) | ((in_array[18 + inpos]) << 20);
    out_array[15 + outpos] = ((in_array[18 + inpos]) >> (26 - 14)) | ((in_array[19 + inpos]) << 14);
    out_array[16 + outpos] = ((in_array[19 + inpos]) >> (26 - 8)) | ((in_array[20 + inpos]) << 8);
    out_array[17 + outpos] = ((in_array[20 + inpos]) >> (26 - 2))
        | ((in_array[21 + inpos]) << 2)
        | ((in_array[22 + inpos]) << 28);
    out_array[18 + outpos] = ((in_array[22 + inpos]) >> (26 - 22)) | ((in_array[23 + inpos]) << 22);
    out_array[19 + outpos] = ((in_array[23 + inpos]) >> (26 - 16)) | ((in_array[24 + inpos]) << 16);
    out_array[20 + outpos] = ((in_array[24 + inpos]) >> (26 - 10)) | ((in_array[25 + inpos]) << 10);
    out_array[21 + outpos] = ((in_array[25 + inpos]) >> (26 - 4))
        | ((in_array[26 + inpos]) << 4)
        | ((in_array[27 + inpos]) << 30);
    out_array[22 + outpos] = ((in_array[27 + inpos]) >> (26 - 24)) | ((in_array[28 + inpos]) << 24);
    out_array[23 + outpos] = ((in_array[28 + inpos]) >> (26 - 18)) | ((in_array[29 + inpos]) << 18);
    out_array[24 + outpos] = ((in_array[29 + inpos]) >> (26 - 12)) | ((in_array[30 + inpos]) << 12);
    out_array[25 + outpos] = ((in_array[30 + inpos]) >> (26 - 6)) | ((in_array[31 + inpos]) << 6);
}

pub fn fastpackwithoutmask27(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 27);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (27 - 22)) | ((in_array[2 + inpos]) << 22);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (27 - 17)) | ((in_array[3 + inpos]) << 17);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (27 - 12)) | ((in_array[4 + inpos]) << 12);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (27 - 7)) | ((in_array[5 + inpos]) << 7);
    out_array[5 + outpos] = ((in_array[5 + inpos]) >> (27 - 2))
        | ((in_array[6 + inpos]) << 2)
        | ((in_array[7 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[7 + inpos]) >> (27 - 24)) | ((in_array[8 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[8 + inpos]) >> (27 - 19)) | ((in_array[9 + inpos]) << 19);
    out_array[8 + outpos] = ((in_array[9 + inpos]) >> (27 - 14)) | ((in_array[10 + inpos]) << 14);
    out_array[9 + outpos] = ((in_array[10 + inpos]) >> (27 - 9)) | ((in_array[11 + inpos]) << 9);
    out_array[10 + outpos] = ((in_array[11 + inpos]) >> (27 - 4))
        | ((in_array[12 + inpos]) << 4)
        | ((in_array[13 + inpos]) << 31);
    out_array[11 + outpos] = ((in_array[13 + inpos]) >> (27 - 26)) | ((in_array[14 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[14 + inpos]) >> (27 - 21)) | ((in_array[15 + inpos]) << 21);
    out_array[13 + outpos] = ((in_array[15 + inpos]) >> (27 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[16 + inpos]) >> (27 - 11)) | ((in_array[17 + inpos]) << 11);
    out_array[15 + outpos] = ((in_array[17 + inpos]) >> (27 - 6)) | ((in_array[18 + inpos]) << 6);
    out_array[16 + outpos] = ((in_array[18 + inpos]) >> (27 - 1))
        | ((in_array[19 + inpos]) << 1)
        | ((in_array[20 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[20 + inpos]) >> (27 - 23)) | ((in_array[21 + inpos]) << 23);
    out_array[18 + outpos] = ((in_array[21 + inpos]) >> (27 - 18)) | ((in_array[22 + inpos]) << 18);
    out_array[19 + outpos] = ((in_array[22 + inpos]) >> (27 - 13)) | ((in_array[23 + inpos]) << 13);
    out_array[20 + outpos] = ((in_array[23 + inpos]) >> (27 - 8)) | ((in_array[24 + inpos]) << 8);
    out_array[21 + outpos] = ((in_array[24 + inpos]) >> (27 - 3))
        | ((in_array[25 + inpos]) << 3)
        | ((in_array[26 + inpos]) << 30);
    out_array[22 + outpos] = ((in_array[26 + inpos]) >> (27 - 25)) | ((in_array[27 + inpos]) << 25);
    out_array[23 + outpos] = ((in_array[27 + inpos]) >> (27 - 20)) | ((in_array[28 + inpos]) << 20);
    out_array[24 + outpos] = ((in_array[28 + inpos]) >> (27 - 15)) | ((in_array[29 + inpos]) << 15);
    out_array[25 + outpos] = ((in_array[29 + inpos]) >> (27 - 10)) | ((in_array[30 + inpos]) << 10);
    out_array[26 + outpos] = ((in_array[30 + inpos]) >> (27 - 5)) | ((in_array[31 + inpos]) << 5);
}

pub fn fastpackwithoutmask28(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (28 - 24)) | ((in_array[2 + inpos]) << 24);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (28 - 20)) | ((in_array[3 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (28 - 16)) | ((in_array[4 + inpos]) << 16);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (28 - 12)) | ((in_array[5 + inpos]) << 12);
    out_array[5 + outpos] = ((in_array[5 + inpos]) >> (28 - 8)) | ((in_array[6 + inpos]) << 8);
    out_array[6 + outpos] = ((in_array[6 + inpos]) >> (28 - 4)) | ((in_array[7 + inpos]) << 4);
    out_array[7 + outpos] = in_array[8 + inpos] | ((in_array[9 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[9 + inpos]) >> (28 - 24)) | ((in_array[10 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[10 + inpos]) >> (28 - 20)) | ((in_array[11 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[11 + inpos]) >> (28 - 16)) | ((in_array[12 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[12 + inpos]) >> (28 - 12)) | ((in_array[13 + inpos]) << 12);
    out_array[12 + outpos] = ((in_array[13 + inpos]) >> (28 - 8)) | ((in_array[14 + inpos]) << 8);
    out_array[13 + outpos] = ((in_array[14 + inpos]) >> (28 - 4)) | ((in_array[15 + inpos]) << 4);
    out_array[14 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 28);
    out_array[15 + outpos] = ((in_array[17 + inpos]) >> (28 - 24)) | ((in_array[18 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[18 + inpos]) >> (28 - 20)) | ((in_array[19 + inpos]) << 20);
    out_array[17 + outpos] = ((in_array[19 + inpos]) >> (28 - 16)) | ((in_array[20 + inpos]) << 16);
    out_array[18 + outpos] = ((in_array[20 + inpos]) >> (28 - 12)) | ((in_array[21 + inpos]) << 12);
    out_array[19 + outpos] = ((in_array[21 + inpos]) >> (28 - 8)) | ((in_array[22 + inpos]) << 8);
    out_array[20 + outpos] = ((in_array[22 + inpos]) >> (28 - 4)) | ((in_array[23 + inpos]) << 4);
    out_array[21 + outpos] = in_array[24 + inpos] | ((in_array[25 + inpos]) << 28);
    out_array[22 + outpos] = ((in_array[25 + inpos]) >> (28 - 24)) | ((in_array[26 + inpos]) << 24);
    out_array[23 + outpos] = ((in_array[26 + inpos]) >> (28 - 20)) | ((in_array[27 + inpos]) << 20);
    out_array[24 + outpos] = ((in_array[27 + inpos]) >> (28 - 16)) | ((in_array[28 + inpos]) << 16);
    out_array[25 + outpos] = ((in_array[28 + inpos]) >> (28 - 12)) | ((in_array[29 + inpos]) << 12);
    out_array[26 + outpos] = ((in_array[29 + inpos]) >> (28 - 8)) | ((in_array[30 + inpos]) << 8);
    out_array[27 + outpos] = ((in_array[30 + inpos]) >> (28 - 4)) | ((in_array[31 + inpos]) << 4);
}

pub fn fastpackwithoutmask29(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 29);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (29 - 26)) | ((in_array[2 + inpos]) << 26);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (29 - 23)) | ((in_array[3 + inpos]) << 23);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (29 - 20)) | ((in_array[4 + inpos]) << 20);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (29 - 17)) | ((in_array[5 + inpos]) << 17);
    out_array[5 + outpos] = ((in_array[5 + inpos]) >> (29 - 14)) | ((in_array[6 + inpos]) << 14);
    out_array[6 + outpos] = ((in_array[6 + inpos]) >> (29 - 11)) | ((in_array[7 + inpos]) << 11);
    out_array[7 + outpos] = ((in_array[7 + inpos]) >> (29 - 8)) | ((in_array[8 + inpos]) << 8);
    out_array[8 + outpos] = ((in_array[8 + inpos]) >> (29 - 5)) | ((in_array[9 + inpos]) << 5);
    out_array[9 + outpos] = ((in_array[9 + inpos]) >> (29 - 2))
        | ((in_array[10 + inpos]) << 2)
        | ((in_array[11 + inpos]) << 31);
    out_array[10 + outpos] = ((in_array[11 + inpos]) >> (29 - 28)) | ((in_array[12 + inpos]) << 28);
    out_array[11 + outpos] = ((in_array[12 + inpos]) >> (29 - 25)) | ((in_array[13 + inpos]) << 25);
    out_array[12 + outpos] = ((in_array[13 + inpos]) >> (29 - 22)) | ((in_array[14 + inpos]) << 22);
    out_array[13 + outpos] = ((in_array[14 + inpos]) >> (29 - 19)) | ((in_array[15 + inpos]) << 19);
    out_array[14 + outpos] = ((in_array[15 + inpos]) >> (29 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[15 + outpos] = ((in_array[16 + inpos]) >> (29 - 13)) | ((in_array[17 + inpos]) << 13);
    out_array[16 + outpos] = ((in_array[17 + inpos]) >> (29 - 10)) | ((in_array[18 + inpos]) << 10);
    out_array[17 + outpos] = ((in_array[18 + inpos]) >> (29 - 7)) | ((in_array[19 + inpos]) << 7);
    out_array[18 + outpos] = ((in_array[19 + inpos]) >> (29 - 4)) | ((in_array[20 + inpos]) << 4);
    out_array[19 + outpos] = ((in_array[20 + inpos]) >> (29 - 1))
        | ((in_array[21 + inpos]) << 1)
        | ((in_array[22 + inpos]) << 30);
    out_array[20 + outpos] = ((in_array[22 + inpos]) >> (29 - 27)) | ((in_array[23 + inpos]) << 27);
    out_array[21 + outpos] = ((in_array[23 + inpos]) >> (29 - 24)) | ((in_array[24 + inpos]) << 24);
    out_array[22 + outpos] = ((in_array[24 + inpos]) >> (29 - 21)) | ((in_array[25 + inpos]) << 21);
    out_array[23 + outpos] = ((in_array[25 + inpos]) >> (29 - 18)) | ((in_array[26 + inpos]) << 18);
    out_array[24 + outpos] = ((in_array[26 + inpos]) >> (29 - 15)) | ((in_array[27 + inpos]) << 15);
    out_array[25 + outpos] = ((in_array[27 + inpos]) >> (29 - 12)) | ((in_array[28 + inpos]) << 12);
    out_array[26 + outpos] = ((in_array[28 + inpos]) >> (29 - 9)) | ((in_array[29 + inpos]) << 9);
    out_array[27 + outpos] = ((in_array[29 + inpos]) >> (29 - 6)) | ((in_array[30 + inpos]) << 6);
    out_array[28 + outpos] = ((in_array[30 + inpos]) >> (29 - 3)) | ((in_array[31 + inpos]) << 3);
}

pub fn fastpackwithoutmask3(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 3)
        | ((in_array[2 + inpos]) << 6)
        | ((in_array[3 + inpos]) << 9)
        | ((in_array[4 + inpos]) << 12)
        | ((in_array[5 + inpos]) << 15)
        | ((in_array[6 + inpos]) << 18)
        | ((in_array[7 + inpos]) << 21)
        | ((in_array[8 + inpos]) << 24)
        | ((in_array[9 + inpos]) << 27)
        | ((in_array[10 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[10 + inpos]) >> (3 - 1))
        | ((in_array[11 + inpos]) << 1)
        | ((in_array[12 + inpos]) << 4)
        | ((in_array[13 + inpos]) << 7)
        | ((in_array[14 + inpos]) << 10)
        | ((in_array[15 + inpos]) << 13)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 19)
        | ((in_array[18 + inpos]) << 22)
        | ((in_array[19 + inpos]) << 25)
        | ((in_array[20 + inpos]) << 28)
        | ((in_array[21 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[21 + inpos]) >> (3 - 2))
        | ((in_array[22 + inpos]) << 2)
        | ((in_array[23 + inpos]) << 5)
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 11)
        | ((in_array[26 + inpos]) << 14)
        | ((in_array[27 + inpos]) << 17)
        | ((in_array[28 + inpos]) << 20)
        | ((in_array[29 + inpos]) << 23)
        | ((in_array[30 + inpos]) << 26)
        | ((in_array[31 + inpos]) << 29);
}

pub fn fastpackwithoutmask30(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (30 - 28)) | ((in_array[2 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (30 - 26)) | ((in_array[3 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (30 - 24)) | ((in_array[4 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (30 - 22)) | ((in_array[5 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[5 + inpos]) >> (30 - 20)) | ((in_array[6 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[6 + inpos]) >> (30 - 18)) | ((in_array[7 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[7 + inpos]) >> (30 - 16)) | ((in_array[8 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[8 + inpos]) >> (30 - 14)) | ((in_array[9 + inpos]) << 14);
    out_array[9 + outpos] = ((in_array[9 + inpos]) >> (30 - 12)) | ((in_array[10 + inpos]) << 12);
    out_array[10 + outpos] = ((in_array[10 + inpos]) >> (30 - 10)) | ((in_array[11 + inpos]) << 10);
    out_array[11 + outpos] = ((in_array[11 + inpos]) >> (30 - 8)) | ((in_array[12 + inpos]) << 8);
    out_array[12 + outpos] = ((in_array[12 + inpos]) >> (30 - 6)) | ((in_array[13 + inpos]) << 6);
    out_array[13 + outpos] = ((in_array[13 + inpos]) >> (30 - 4)) | ((in_array[14 + inpos]) << 4);
    out_array[14 + outpos] = ((in_array[14 + inpos]) >> (30 - 2)) | ((in_array[15 + inpos]) << 2);
    out_array[15 + outpos] = in_array[16 + inpos] | ((in_array[17 + inpos]) << 30);
    out_array[16 + outpos] = ((in_array[17 + inpos]) >> (30 - 28)) | ((in_array[18 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[18 + inpos]) >> (30 - 26)) | ((in_array[19 + inpos]) << 26);
    out_array[18 + outpos] = ((in_array[19 + inpos]) >> (30 - 24)) | ((in_array[20 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[20 + inpos]) >> (30 - 22)) | ((in_array[21 + inpos]) << 22);
    out_array[20 + outpos] = ((in_array[21 + inpos]) >> (30 - 20)) | ((in_array[22 + inpos]) << 20);
    out_array[21 + outpos] = ((in_array[22 + inpos]) >> (30 - 18)) | ((in_array[23 + inpos]) << 18);
    out_array[22 + outpos] = ((in_array[23 + inpos]) >> (30 - 16)) | ((in_array[24 + inpos]) << 16);
    out_array[23 + outpos] = ((in_array[24 + inpos]) >> (30 - 14)) | ((in_array[25 + inpos]) << 14);
    out_array[24 + outpos] = ((in_array[25 + inpos]) >> (30 - 12)) | ((in_array[26 + inpos]) << 12);
    out_array[25 + outpos] = ((in_array[26 + inpos]) >> (30 - 10)) | ((in_array[27 + inpos]) << 10);
    out_array[26 + outpos] = ((in_array[27 + inpos]) >> (30 - 8)) | ((in_array[28 + inpos]) << 8);
    out_array[27 + outpos] = ((in_array[28 + inpos]) >> (30 - 6)) | ((in_array[29 + inpos]) << 6);
    out_array[28 + outpos] = ((in_array[29 + inpos]) >> (30 - 4)) | ((in_array[30 + inpos]) << 4);
    out_array[29 + outpos] = ((in_array[30 + inpos]) >> (30 - 2)) | ((in_array[31 + inpos]) << 2);
}

pub fn fastpackwithoutmask31(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] | ((in_array[1 + inpos]) << 31);
    out_array[1 + outpos] = ((in_array[1 + inpos]) >> (31 - 30)) | ((in_array[2 + inpos]) << 30);
    out_array[2 + outpos] = ((in_array[2 + inpos]) >> (31 - 29)) | ((in_array[3 + inpos]) << 29);
    out_array[3 + outpos] = ((in_array[3 + inpos]) >> (31 - 28)) | ((in_array[4 + inpos]) << 28);
    out_array[4 + outpos] = ((in_array[4 + inpos]) >> (31 - 27)) | ((in_array[5 + inpos]) << 27);
    out_array[5 + outpos] = ((in_array[5 + inpos]) >> (31 - 26)) | ((in_array[6 + inpos]) << 26);
    out_array[6 + outpos] = ((in_array[6 + inpos]) >> (31 - 25)) | ((in_array[7 + inpos]) << 25);
    out_array[7 + outpos] = ((in_array[7 + inpos]) >> (31 - 24)) | ((in_array[8 + inpos]) << 24);
    out_array[8 + outpos] = ((in_array[8 + inpos]) >> (31 - 23)) | ((in_array[9 + inpos]) << 23);
    out_array[9 + outpos] = ((in_array[9 + inpos]) >> (31 - 22)) | ((in_array[10 + inpos]) << 22);
    out_array[10 + outpos] = ((in_array[10 + inpos]) >> (31 - 21)) | ((in_array[11 + inpos]) << 21);
    out_array[11 + outpos] = ((in_array[11 + inpos]) >> (31 - 20)) | ((in_array[12 + inpos]) << 20);
    out_array[12 + outpos] = ((in_array[12 + inpos]) >> (31 - 19)) | ((in_array[13 + inpos]) << 19);
    out_array[13 + outpos] = ((in_array[13 + inpos]) >> (31 - 18)) | ((in_array[14 + inpos]) << 18);
    out_array[14 + outpos] = ((in_array[14 + inpos]) >> (31 - 17)) | ((in_array[15 + inpos]) << 17);
    out_array[15 + outpos] = ((in_array[15 + inpos]) >> (31 - 16)) | ((in_array[16 + inpos]) << 16);
    out_array[16 + outpos] = ((in_array[16 + inpos]) >> (31 - 15)) | ((in_array[17 + inpos]) << 15);
    out_array[17 + outpos] = ((in_array[17 + inpos]) >> (31 - 14)) | ((in_array[18 + inpos]) << 14);
    out_array[18 + outpos] = ((in_array[18 + inpos]) >> (31 - 13)) | ((in_array[19 + inpos]) << 13);
    out_array[19 + outpos] = ((in_array[19 + inpos]) >> (31 - 12)) | ((in_array[20 + inpos]) << 12);
    out_array[20 + outpos] = ((in_array[20 + inpos]) >> (31 - 11)) | ((in_array[21 + inpos]) << 11);
    out_array[21 + outpos] = ((in_array[21 + inpos]) >> (31 - 10)) | ((in_array[22 + inpos]) << 10);
    out_array[22 + outpos] = ((in_array[22 + inpos]) >> (31 - 9)) | ((in_array[23 + inpos]) << 9);
    out_array[23 + outpos] = ((in_array[23 + inpos]) >> (31 - 8)) | ((in_array[24 + inpos]) << 8);
    out_array[24 + outpos] = ((in_array[24 + inpos]) >> (31 - 7)) | ((in_array[25 + inpos]) << 7);
    out_array[25 + outpos] = ((in_array[25 + inpos]) >> (31 - 6)) | ((in_array[26 + inpos]) << 6);
    out_array[26 + outpos] = ((in_array[26 + inpos]) >> (31 - 5)) | ((in_array[27 + inpos]) << 5);
    out_array[27 + outpos] = ((in_array[27 + inpos]) >> (31 - 4)) | ((in_array[28 + inpos]) << 4);
    out_array[28 + outpos] = ((in_array[28 + inpos]) >> (31 - 3)) | ((in_array[29 + inpos]) << 3);
    out_array[29 + outpos] = ((in_array[29 + inpos]) >> (31 - 2)) | ((in_array[30 + inpos]) << 2);
    out_array[30 + outpos] = ((in_array[30 + inpos]) >> (31 - 1)) | ((in_array[31 + inpos]) << 1);
}

pub fn fastpackwithoutmask32(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos..outpos + 32].copy_from_slice(&in_array[inpos..inpos + 32]);
}

pub fn fastpackwithoutmask4(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 4)
        | ((in_array[2 + inpos]) << 8)
        | ((in_array[3 + inpos]) << 12)
        | ((in_array[4 + inpos]) << 16)
        | ((in_array[5 + inpos]) << 20)
        | ((in_array[6 + inpos]) << 24)
        | ((in_array[7 + inpos]) << 28);
    out_array[1 + outpos] = in_array[8 + inpos]
        | ((in_array[9 + inpos]) << 4)
        | ((in_array[10 + inpos]) << 8)
        | ((in_array[11 + inpos]) << 12)
        | ((in_array[12 + inpos]) << 16)
        | ((in_array[13 + inpos]) << 20)
        | ((in_array[14 + inpos]) << 24)
        | ((in_array[15 + inpos]) << 28);
    out_array[2 + outpos] = in_array[16 + inpos]
        | ((in_array[17 + inpos]) << 4)
        | ((in_array[18 + inpos]) << 8)
        | ((in_array[19 + inpos]) << 12)
        | ((in_array[20 + inpos]) << 16)
        | ((in_array[21 + inpos]) << 20)
        | ((in_array[22 + inpos]) << 24)
        | ((in_array[23 + inpos]) << 28);
    out_array[3 + outpos] = in_array[24 + inpos]
        | ((in_array[25 + inpos]) << 4)
        | ((in_array[26 + inpos]) << 8)
        | ((in_array[27 + inpos]) << 12)
        | ((in_array[28 + inpos]) << 16)
        | ((in_array[29 + inpos]) << 20)
        | ((in_array[30 + inpos]) << 24)
        | ((in_array[31 + inpos]) << 28);
}

pub fn fastpackwithoutmask5(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 5)
        | ((in_array[2 + inpos]) << 10)
        | ((in_array[3 + inpos]) << 15)
        | ((in_array[4 + inpos]) << 20)
        | ((in_array[5 + inpos]) << 25)
        | ((in_array[6 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[6 + inpos]) >> (5 - 3))
        | ((in_array[7 + inpos]) << 3)
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 13)
        | ((in_array[10 + inpos]) << 18)
        | ((in_array[11 + inpos]) << 23)
        | ((in_array[12 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[12 + inpos]) >> (5 - 1))
        | ((in_array[13 + inpos]) << 1)
        | ((in_array[14 + inpos]) << 6)
        | ((in_array[15 + inpos]) << 11)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 21)
        | ((in_array[18 + inpos]) << 26)
        | ((in_array[19 + inpos]) << 31);
    out_array[3 + outpos] = ((in_array[19 + inpos]) >> (5 - 4))
        | ((in_array[20 + inpos]) << 4)
        | ((in_array[21 + inpos]) << 9)
        | ((in_array[22 + inpos]) << 14)
        | ((in_array[23 + inpos]) << 19)
        | ((in_array[24 + inpos]) << 24)
        | ((in_array[25 + inpos]) << 29);
    out_array[4 + outpos] = ((in_array[25 + inpos]) >> (5 - 2))
        | ((in_array[26 + inpos]) << 2)
        | ((in_array[27 + inpos]) << 7)
        | ((in_array[28 + inpos]) << 12)
        | ((in_array[29 + inpos]) << 17)
        | ((in_array[30 + inpos]) << 22)
        | ((in_array[31 + inpos]) << 27);
}

pub fn fastpackwithoutmask6(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 6)
        | ((in_array[2 + inpos]) << 12)
        | ((in_array[3 + inpos]) << 18)
        | ((in_array[4 + inpos]) << 24)
        | ((in_array[5 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[5 + inpos]) >> (6 - 4))
        | ((in_array[6 + inpos]) << 4)
        | ((in_array[7 + inpos]) << 10)
        | ((in_array[8 + inpos]) << 16)
        | ((in_array[9 + inpos]) << 22)
        | ((in_array[10 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[10 + inpos]) >> (6 - 2))
        | ((in_array[11 + inpos]) << 2)
        | ((in_array[12 + inpos]) << 8)
        | ((in_array[13 + inpos]) << 14)
        | ((in_array[14 + inpos]) << 20)
        | ((in_array[15 + inpos]) << 26);
    out_array[3 + outpos] = in_array[16 + inpos]
        | ((in_array[17 + inpos]) << 6)
        | ((in_array[18 + inpos]) << 12)
        | ((in_array[19 + inpos]) << 18)
        | ((in_array[20 + inpos]) << 24)
        | ((in_array[21 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[21 + inpos]) >> (6 - 4))
        | ((in_array[22 + inpos]) << 4)
        | ((in_array[23 + inpos]) << 10)
        | ((in_array[24 + inpos]) << 16)
        | ((in_array[25 + inpos]) << 22)
        | ((in_array[26 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[26 + inpos]) >> (6 - 2))
        | ((in_array[27 + inpos]) << 2)
        | ((in_array[28 + inpos]) << 8)
        | ((in_array[29 + inpos]) << 14)
        | ((in_array[30 + inpos]) << 20)
        | ((in_array[31 + inpos]) << 26);
}

pub fn fastpackwithoutmask7(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 7)
        | ((in_array[2 + inpos]) << 14)
        | ((in_array[3 + inpos]) << 21)
        | ((in_array[4 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[4 + inpos]) >> (7 - 3))
        | ((in_array[5 + inpos]) << 3)
        | ((in_array[6 + inpos]) << 10)
        | ((in_array[7 + inpos]) << 17)
        | ((in_array[8 + inpos]) << 24)
        | ((in_array[9 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[9 + inpos]) >> (7 - 6))
        | ((in_array[10 + inpos]) << 6)
        | ((in_array[11 + inpos]) << 13)
        | ((in_array[12 + inpos]) << 20)
        | ((in_array[13 + inpos]) << 27);
    out_array[3 + outpos] = ((in_array[13 + inpos]) >> (7 - 2))
        | ((in_array[14 + inpos]) << 2)
        | ((in_array[15 + inpos]) << 9)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 23)
        | ((in_array[18 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[18 + inpos]) >> (7 - 5))
        | ((in_array[19 + inpos]) << 5)
        | ((in_array[20 + inpos]) << 12)
        | ((in_array[21 + inpos]) << 19)
        | ((in_array[22 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[22 + inpos]) >> (7 - 1))
        | ((in_array[23 + inpos]) << 1)
        | ((in_array[24 + inpos]) << 8)
        | ((in_array[25 + inpos]) << 15)
        | ((in_array[26 + inpos]) << 22)
        | ((in_array[27 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[27 + inpos]) >> (7 - 4))
        | ((in_array[28 + inpos]) << 4)
        | ((in_array[29 + inpos]) << 11)
        | ((in_array[30 + inpos]) << 18)
        | ((in_array[31 + inpos]) << 25);
}

pub fn fastpackwithoutmask8(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 8)
        | ((in_array[2 + inpos]) << 16)
        | ((in_array[3 + inpos]) << 24);
    out_array[1 + outpos] = in_array[4 + inpos]
        | ((in_array[5 + inpos]) << 8)
        | ((in_array[6 + inpos]) << 16)
        | ((in_array[7 + inpos]) << 24);
    out_array[2 + outpos] = in_array[8 + inpos]
        | ((in_array[9 + inpos]) << 8)
        | ((in_array[10 + inpos]) << 16)
        | ((in_array[11 + inpos]) << 24);
    out_array[3 + outpos] = in_array[12 + inpos]
        | ((in_array[13 + inpos]) << 8)
        | ((in_array[14 + inpos]) << 16)
        | ((in_array[15 + inpos]) << 24);
    out_array[4 + outpos] = in_array[16 + inpos]
        | ((in_array[17 + inpos]) << 8)
        | ((in_array[18 + inpos]) << 16)
        | ((in_array[19 + inpos]) << 24);
    out_array[5 + outpos] = in_array[20 + inpos]
        | ((in_array[21 + inpos]) << 8)
        | ((in_array[22 + inpos]) << 16)
        | ((in_array[23 + inpos]) << 24);
    out_array[6 + outpos] = in_array[24 + inpos]
        | ((in_array[25 + inpos]) << 8)
        | ((in_array[26 + inpos]) << 16)
        | ((in_array[27 + inpos]) << 24);
    out_array[7 + outpos] = in_array[28 + inpos]
        | ((in_array[29 + inpos]) << 8)
        | ((in_array[30 + inpos]) << 16)
        | ((in_array[31 + inpos]) << 24);
}

pub fn fastpackwithoutmask9(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos]
        | ((in_array[1 + inpos]) << 9)
        | ((in_array[2 + inpos]) << 18)
        | ((in_array[3 + inpos]) << 27);
    out_array[1 + outpos] = ((in_array[3 + inpos]) >> (9 - 4))
        | ((in_array[4 + inpos]) << 4)
        | ((in_array[5 + inpos]) << 13)
        | ((in_array[6 + inpos]) << 22)
        | ((in_array[7 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[7 + inpos]) >> (9 - 8))
        | ((in_array[8 + inpos]) << 8)
        | ((in_array[9 + inpos]) << 17)
        | ((in_array[10 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[10 + inpos]) >> (9 - 3))
        | ((in_array[11 + inpos]) << 3)
        | ((in_array[12 + inpos]) << 12)
        | ((in_array[13 + inpos]) << 21)
        | ((in_array[14 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[14 + inpos]) >> (9 - 7))
        | ((in_array[15 + inpos]) << 7)
        | ((in_array[16 + inpos]) << 16)
        | ((in_array[17 + inpos]) << 25);
    out_array[5 + outpos] = ((in_array[17 + inpos]) >> (9 - 2))
        | ((in_array[18 + inpos]) << 2)
        | ((in_array[19 + inpos]) << 11)
        | ((in_array[20 + inpos]) << 20)
        | ((in_array[21 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[21 + inpos]) >> (9 - 6))
        | ((in_array[22 + inpos]) << 6)
        | ((in_array[23 + inpos]) << 15)
        | ((in_array[24 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[24 + inpos]) >> (9 - 1))
        | ((in_array[25 + inpos]) << 1)
        | ((in_array[26 + inpos]) << 10)
        | ((in_array[27 + inpos]) << 19)
        | ((in_array[28 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[28 + inpos]) >> (9 - 5))
        | ((in_array[29 + inpos]) << 5)
        | ((in_array[30 + inpos]) << 14)
        | ((in_array[31 + inpos]) << 23);
}
