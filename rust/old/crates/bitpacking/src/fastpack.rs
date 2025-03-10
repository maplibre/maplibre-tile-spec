
/// Pack the 32 integers
pub fn fastpack(
    in_array: &[u32],
    inpos: usize,
    out_array: &mut [u32],
    outpos: usize,
    bit: u8,
) {
    match bit {
        0 => fastpack0(in_array, inpos, out_array, outpos),
        1 => fastpack1(in_array, inpos, out_array, outpos),
        2 => fastpack2(in_array, inpos, out_array, outpos),
        3 => fastpack3(in_array, inpos, out_array, outpos),
        4 => fastpack4(in_array, inpos, out_array, outpos),
        5 => fastpack5(in_array, inpos, out_array, outpos),
        6 => fastpack6(in_array, inpos, out_array, outpos),
        7 => fastpack7(in_array, inpos, out_array, outpos),
        8 => fastpack8(in_array, inpos, out_array, outpos),
        9 => fastpack9(in_array, inpos, out_array, outpos),
        10 => fastpack10(in_array, inpos, out_array, outpos),
        11 => fastpack11(in_array, inpos, out_array, outpos),
        12 => fastpack12(in_array, inpos, out_array, outpos),
        13 => fastpack13(in_array, inpos, out_array, outpos),
        14 => fastpack14(in_array, inpos, out_array, outpos),
        15 => fastpack15(in_array, inpos, out_array, outpos),
        16 => fastpack16(in_array, inpos, out_array, outpos),
        17 => fastpack17(in_array, inpos, out_array, outpos),
        18 => fastpack18(in_array, inpos, out_array, outpos),
        19 => fastpack19(in_array, inpos, out_array, outpos),
        20 => fastpack20(in_array, inpos, out_array, outpos),
        21 => fastpack21(in_array, inpos, out_array, outpos),
        22 => fastpack22(in_array, inpos, out_array, outpos),
        23 => fastpack23(in_array, inpos, out_array, outpos),
        24 => fastpack24(in_array, inpos, out_array, outpos),
        25 => fastpack25(in_array, inpos, out_array, outpos),
        26 => fastpack26(in_array, inpos, out_array, outpos),
        27 => fastpack27(in_array, inpos, out_array, outpos),
        28 => fastpack28(in_array, inpos, out_array, outpos),
        29 => fastpack29(in_array, inpos, out_array, outpos),
        30 => fastpack30(in_array, inpos, out_array, outpos),
        31 => fastpack31(in_array, inpos, out_array, outpos),
        32 => fastpack32(in_array, inpos, out_array, outpos),
        _ => panic!("Unsupported bit width.")
    }
}

pub fn fastpack0(_in_array: &[u32], _inpos: usize, _out_array: &mut [u32], _outpos: usize) {/* nothing */}

pub fn fastpack1(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 1)
        | ((in_array[1 + inpos] & 1) << 1)
        | ((in_array[2 + inpos] & 1) << 2)
        | ((in_array[3 + inpos] & 1) << 3)
        | ((in_array[4 + inpos] & 1) << 4)
        | ((in_array[5 + inpos] & 1) << 5)
        | ((in_array[6 + inpos] & 1) << 6)
        | ((in_array[7 + inpos] & 1) << 7)
        | ((in_array[8 + inpos] & 1) << 8)
        | ((in_array[9 + inpos] & 1) << 9)
        | ((in_array[10 + inpos] & 1) << 10)
        | ((in_array[11 + inpos] & 1) << 11)
        | ((in_array[12 + inpos] & 1) << 12)
        | ((in_array[13 + inpos] & 1) << 13)
        | ((in_array[14 + inpos] & 1) << 14)
        | ((in_array[15 + inpos] & 1) << 15)
        | ((in_array[16 + inpos] & 1) << 16)
        | ((in_array[17 + inpos] & 1) << 17)
        | ((in_array[18 + inpos] & 1) << 18)
        | ((in_array[19 + inpos] & 1) << 19)
        | ((in_array[20 + inpos] & 1) << 20)
        | ((in_array[21 + inpos] & 1) << 21)
        | ((in_array[22 + inpos] & 1) << 22)
        | ((in_array[23 + inpos] & 1) << 23)
        | ((in_array[24 + inpos] & 1) << 24)
        | ((in_array[25 + inpos] & 1) << 25)
        | ((in_array[26 + inpos] & 1) << 26)
        | ((in_array[27 + inpos] & 1) << 27)
        | ((in_array[28 + inpos] & 1) << 28)
        | ((in_array[29 + inpos] & 1) << 29)
        | ((in_array[30 + inpos] & 1) << 30)
        | ((in_array[31 + inpos]) << 31);
}

pub fn fastpack10(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 1023)
        | ((in_array[1 + inpos] & 1023) << 10)
        | ((in_array[2 + inpos] & 1023) << 20)
        | ((in_array[3 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[3 + inpos] & 1023) >> (10 - 8))
        | ((in_array[4 + inpos] & 1023) << 8)
        | ((in_array[5 + inpos] & 1023) << 18)
        | ((in_array[6 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[6 + inpos] & 1023) >> (10 - 6))
        | ((in_array[7 + inpos] & 1023) << 6)
        | ((in_array[8 + inpos] & 1023) << 16)
        | ((in_array[9 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[9 + inpos] & 1023) >> (10 - 4))
        | ((in_array[10 + inpos] & 1023) << 4)
        | ((in_array[11 + inpos] & 1023) << 14)
        | ((in_array[12 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[12 + inpos] & 1023) >> (10 - 2))
        | ((in_array[13 + inpos] & 1023) << 2)
        | ((in_array[14 + inpos] & 1023) << 12)
        | ((in_array[15 + inpos]) << 22);
    out_array[5 + outpos] = (in_array[16 + inpos] & 1023)
        | ((in_array[17 + inpos] & 1023) << 10)
        | ((in_array[18 + inpos] & 1023) << 20)
        | ((in_array[19 + inpos]) << 30);
    out_array[6 + outpos] = ((in_array[19 + inpos] & 1023) >> (10 - 8))
        | ((in_array[20 + inpos] & 1023) << 8)
        | ((in_array[21 + inpos] & 1023) << 18)
        | ((in_array[22 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[22 + inpos] & 1023) >> (10 - 6))
        | ((in_array[23 + inpos] & 1023) << 6)
        | ((in_array[24 + inpos] & 1023) << 16)
        | ((in_array[25 + inpos]) << 26);
    out_array[8 + outpos] = ((in_array[25 + inpos] & 1023) >> (10 - 4))
        | ((in_array[26 + inpos] & 1023) << 4)
        | ((in_array[27 + inpos] & 1023) << 14)
        | ((in_array[28 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[28 + inpos] & 1023) >> (10 - 2))
        | ((in_array[29 + inpos] & 1023) << 2)
        | ((in_array[30 + inpos] & 1023) << 12)
        | ((in_array[31 + inpos]) << 22);
}

pub fn fastpack11(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 2047)
        | ((in_array[1 + inpos] & 2047) << 11)
        | ((in_array[2 + inpos]) << 22);
    out_array[1 + outpos] = ((in_array[2 + inpos] & 2047) >> (11 - 1))
        | ((in_array[3 + inpos] & 2047) << 1)
        | ((in_array[4 + inpos] & 2047) << 12)
        | ((in_array[5 + inpos]) << 23);
    out_array[2 + outpos] = ((in_array[5 + inpos] & 2047) >> (11 - 2))
        | ((in_array[6 + inpos] & 2047) << 2)
        | ((in_array[7 + inpos] & 2047) << 13)
        | ((in_array[8 + inpos]) << 24);
    out_array[3 + outpos] = ((in_array[8 + inpos] & 2047) >> (11 - 3))
        | ((in_array[9 + inpos] & 2047) << 3)
        | ((in_array[10 + inpos] & 2047) << 14)
        | ((in_array[11 + inpos]) << 25);
    out_array[4 + outpos] = ((in_array[11 + inpos] & 2047) >> (11 - 4))
        | ((in_array[12 + inpos] & 2047) << 4)
        | ((in_array[13 + inpos] & 2047) << 15)
        | ((in_array[14 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[14 + inpos] & 2047) >> (11 - 5))
        | ((in_array[15 + inpos] & 2047) << 5)
        | ((in_array[16 + inpos] & 2047) << 16)
        | ((in_array[17 + inpos]) << 27);
    out_array[6 + outpos] = ((in_array[17 + inpos] & 2047) >> (11 - 6))
        | ((in_array[18 + inpos] & 2047) << 6)
        | ((in_array[19 + inpos] & 2047) << 17)
        | ((in_array[20 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[20 + inpos] & 2047) >> (11 - 7))
        | ((in_array[21 + inpos] & 2047) << 7)
        | ((in_array[22 + inpos] & 2047) << 18)
        | ((in_array[23 + inpos]) << 29);
    out_array[8 + outpos] = ((in_array[23 + inpos] & 2047) >> (11 - 8))
        | ((in_array[24 + inpos] & 2047) << 8)
        | ((in_array[25 + inpos] & 2047) << 19)
        | ((in_array[26 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[26 + inpos] & 2047) >> (11 - 9))
        | ((in_array[27 + inpos] & 2047) << 9)
        | ((in_array[28 + inpos] & 2047) << 20)
        | ((in_array[29 + inpos]) << 31);
    out_array[10 + outpos] = ((in_array[29 + inpos] & 2047) >> (11 - 10))
        | ((in_array[30 + inpos] & 2047) << 10)
        | ((in_array[31 + inpos]) << 21);
}

pub fn fastpack12(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 4095)
        | ((in_array[1 + inpos] & 4095) << 12)
        | ((in_array[2 + inpos]) << 24);
    out_array[1 + outpos] = ((in_array[2 + inpos] & 4095) >> (12 - 4))
        | ((in_array[3 + inpos] & 4095) << 4)
        | ((in_array[4 + inpos] & 4095) << 16)
        | ((in_array[5 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[5 + inpos] & 4095) >> (12 - 8))
        | ((in_array[6 + inpos] & 4095) << 8)
        | ((in_array[7 + inpos]) << 20);
    out_array[3 + outpos] = (in_array[8 + inpos] & 4095)
        | ((in_array[9 + inpos] & 4095) << 12)
        | ((in_array[10 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[10 + inpos] & 4095) >> (12 - 4))
        | ((in_array[11 + inpos] & 4095) << 4)
        | ((in_array[12 + inpos] & 4095) << 16)
        | ((in_array[13 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[13 + inpos] & 4095) >> (12 - 8))
        | ((in_array[14 + inpos] & 4095) << 8)
        | ((in_array[15 + inpos]) << 20);
    out_array[6 + outpos] = (in_array[16 + inpos] & 4095)
        | ((in_array[17 + inpos] & 4095) << 12)
        | ((in_array[18 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[18 + inpos] & 4095) >> (12 - 4))
        | ((in_array[19 + inpos] & 4095) << 4)
        | ((in_array[20 + inpos] & 4095) << 16)
        | ((in_array[21 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[21 + inpos] & 4095) >> (12 - 8))
        | ((in_array[22 + inpos] & 4095) << 8)
        | ((in_array[23 + inpos]) << 20);
    out_array[9 + outpos] = (in_array[24 + inpos] & 4095)
        | ((in_array[25 + inpos] & 4095) << 12)
        | ((in_array[26 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[26 + inpos] & 4095) >> (12 - 4))
        | ((in_array[27 + inpos] & 4095) << 4)
        | ((in_array[28 + inpos] & 4095) << 16)
        | ((in_array[29 + inpos]) << 28);
    out_array[11 + outpos] = ((in_array[29 + inpos] & 4095) >> (12 - 8))
        | ((in_array[30 + inpos] & 4095) << 8)
        | ((in_array[31 + inpos]) << 20);
}

pub fn fastpack13(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 8191)
        | ((in_array[1 + inpos] & 8191) << 13)
        | ((in_array[2 + inpos]) << 26);
    out_array[1 + outpos] = ((in_array[2 + inpos] & 8191) >> (13 - 7))
        | ((in_array[3 + inpos] & 8191) << 7)
        | ((in_array[4 + inpos]) << 20);
    out_array[2 + outpos] = ((in_array[4 + inpos] & 8191) >> (13 - 1))
        | ((in_array[5 + inpos] & 8191) << 1)
        | ((in_array[6 + inpos] & 8191) << 14)
        | ((in_array[7 + inpos]) << 27);
    out_array[3 + outpos] = ((in_array[7 + inpos] & 8191) >> (13 - 8))
        | ((in_array[8 + inpos] & 8191) << 8)
        | ((in_array[9 + inpos]) << 21);
    out_array[4 + outpos] = ((in_array[9 + inpos] & 8191) >> (13 - 2))
        | ((in_array[10 + inpos] & 8191) << 2)
        | ((in_array[11 + inpos] & 8191) << 15)
        | ((in_array[12 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[12 + inpos] & 8191) >> (13 - 9))
        | ((in_array[13 + inpos] & 8191) << 9)
        | ((in_array[14 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[14 + inpos] & 8191) >> (13 - 3))
        | ((in_array[15 + inpos] & 8191) << 3)
        | ((in_array[16 + inpos] & 8191) << 16)
        | ((in_array[17 + inpos]) << 29);
    out_array[7 + outpos] = ((in_array[17 + inpos] & 8191) >> (13 - 10))
        | ((in_array[18 + inpos] & 8191) << 10)
        | ((in_array[19 + inpos]) << 23);
    out_array[8 + outpos] = ((in_array[19 + inpos] & 8191) >> (13 - 4))
        | ((in_array[20 + inpos] & 8191) << 4)
        | ((in_array[21 + inpos] & 8191) << 17)
        | ((in_array[22 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[22 + inpos] & 8191) >> (13 - 11))
        | ((in_array[23 + inpos] & 8191) << 11)
        | ((in_array[24 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[24 + inpos] & 8191) >> (13 - 5))
        | ((in_array[25 + inpos] & 8191) << 5)
        | ((in_array[26 + inpos] & 8191) << 18)
        | ((in_array[27 + inpos]) << 31);
    out_array[11 + outpos] = ((in_array[27 + inpos] & 8191) >> (13 - 12))
        | ((in_array[28 + inpos] & 8191) << 12)
        | ((in_array[29 + inpos]) << 25);
    out_array[12 + outpos] = ((in_array[29 + inpos] & 8191) >> (13 - 6))
        | ((in_array[30 + inpos] & 8191) << 6)
        | ((in_array[31 + inpos]) << 19);
}

pub fn fastpack14(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 16383)
        | ((in_array[1 + inpos] & 16383) << 14)
        | ((in_array[2 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[2 + inpos] & 16383) >> (14 - 10))
        | ((in_array[3 + inpos] & 16383) << 10)
        | ((in_array[4 + inpos]) << 24);
    out_array[2 + outpos] = ((in_array[4 + inpos] & 16383) >> (14 - 6))
        | ((in_array[5 + inpos] & 16383) << 6)
        | ((in_array[6 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[6 + inpos] & 16383) >> (14 - 2))
        | ((in_array[7 + inpos] & 16383) << 2)
        | ((in_array[8 + inpos] & 16383) << 16)
        | ((in_array[9 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[9 + inpos] & 16383) >> (14 - 12))
        | ((in_array[10 + inpos] & 16383) << 12)
        | ((in_array[11 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[11 + inpos] & 16383) >> (14 - 8))
        | ((in_array[12 + inpos] & 16383) << 8)
        | ((in_array[13 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[13 + inpos] & 16383) >> (14 - 4))
        | ((in_array[14 + inpos] & 16383) << 4)
        | ((in_array[15 + inpos]) << 18);
    out_array[7 + outpos] = (in_array[16 + inpos] & 16383)
        | ((in_array[17 + inpos] & 16383) << 14)
        | ((in_array[18 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[18 + inpos] & 16383) >> (14 - 10))
        | ((in_array[19 + inpos] & 16383) << 10)
        | ((in_array[20 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[20 + inpos] & 16383) >> (14 - 6))
        | ((in_array[21 + inpos] & 16383) << 6)
        | ((in_array[22 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[22 + inpos] & 16383) >> (14 - 2))
        | ((in_array[23 + inpos] & 16383) << 2)
        | ((in_array[24 + inpos] & 16383) << 16)
        | ((in_array[25 + inpos]) << 30);
    out_array[11 + outpos] = ((in_array[25 + inpos] & 16383) >> (14 - 12))
        | ((in_array[26 + inpos] & 16383) << 12)
        | ((in_array[27 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[27 + inpos] & 16383) >> (14 - 8))
        | ((in_array[28 + inpos] & 16383) << 8)
        | ((in_array[29 + inpos]) << 22);
    out_array[13 + outpos] = ((in_array[29 + inpos] & 16383) >> (14 - 4))
        | ((in_array[30 + inpos] & 16383) << 4)
        | ((in_array[31 + inpos]) << 18);
}

pub fn fastpack15(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 32767)
        | ((in_array[1 + inpos] & 32767) << 15)
        | ((in_array[2 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[2 + inpos] & 32767) >> (15 - 13))
        | ((in_array[3 + inpos] & 32767) << 13)
        | ((in_array[4 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[4 + inpos] & 32767) >> (15 - 11))
        | ((in_array[5 + inpos] & 32767) << 11)
        | ((in_array[6 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[6 + inpos] & 32767) >> (15 - 9))
        | ((in_array[7 + inpos] & 32767) << 9)
        | ((in_array[8 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[8 + inpos] & 32767) >> (15 - 7))
        | ((in_array[9 + inpos] & 32767) << 7)
        | ((in_array[10 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[10 + inpos] & 32767) >> (15 - 5))
        | ((in_array[11 + inpos] & 32767) << 5)
        | ((in_array[12 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[12 + inpos] & 32767) >> (15 - 3))
        | ((in_array[13 + inpos] & 32767) << 3)
        | ((in_array[14 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[14 + inpos] & 32767) >> (15 - 1))
        | ((in_array[15 + inpos] & 32767) << 1)
        | ((in_array[16 + inpos] & 32767) << 16)
        | ((in_array[17 + inpos]) << 31);
    out_array[8 + outpos] = ((in_array[17 + inpos] & 32767) >> (15 - 14))
        | ((in_array[18 + inpos] & 32767) << 14)
        | ((in_array[19 + inpos]) << 29);
    out_array[9 + outpos] = ((in_array[19 + inpos] & 32767) >> (15 - 12))
        | ((in_array[20 + inpos] & 32767) << 12)
        | ((in_array[21 + inpos]) << 27);
    out_array[10 + outpos] = ((in_array[21 + inpos] & 32767) >> (15 - 10))
        | ((in_array[22 + inpos] & 32767) << 10)
        | ((in_array[23 + inpos]) << 25);
    out_array[11 + outpos] = ((in_array[23 + inpos] & 32767) >> (15 - 8))
        | ((in_array[24 + inpos] & 32767) << 8)
        | ((in_array[25 + inpos]) << 23);
    out_array[12 + outpos] = ((in_array[25 + inpos] & 32767) >> (15 - 6))
        | ((in_array[26 + inpos] & 32767) << 6)
        | ((in_array[27 + inpos]) << 21);
    out_array[13 + outpos] = ((in_array[27 + inpos] & 32767) >> (15 - 4))
        | ((in_array[28 + inpos] & 32767) << 4)
        | ((in_array[29 + inpos]) << 19);
    out_array[14 + outpos] = ((in_array[29 + inpos] & 32767) >> (15 - 2))
        | ((in_array[30 + inpos] & 32767) << 2)
        | ((in_array[31 + inpos]) << 17);
}

pub fn fastpack16(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 65535)
        | ((in_array[1 + inpos]) << 16);
    out_array[1 + outpos] = (in_array[2 + inpos] & 65535)
        | ((in_array[3 + inpos]) << 16);
    out_array[2 + outpos] = (in_array[4 + inpos] & 65535)
        | ((in_array[5 + inpos]) << 16);
    out_array[3 + outpos] = (in_array[6 + inpos] & 65535)
        | ((in_array[7 + inpos]) << 16);
    out_array[4 + outpos] = (in_array[8 + inpos] & 65535)
        | ((in_array[9 + inpos]) << 16);
    out_array[5 + outpos] = (in_array[10 + inpos] & 65535)
        | ((in_array[11 + inpos]) << 16);
    out_array[6 + outpos] = (in_array[12 + inpos] & 65535)
        | ((in_array[13 + inpos]) << 16);
    out_array[7 + outpos] = (in_array[14 + inpos] & 65535)
        | ((in_array[15 + inpos]) << 16);
    out_array[8 + outpos] = (in_array[16 + inpos] & 65535)
        | ((in_array[17 + inpos]) << 16);
    out_array[9 + outpos] = (in_array[18 + inpos] & 65535)
        | ((in_array[19 + inpos]) << 16);
    out_array[10 + outpos] = (in_array[20 + inpos] & 65535)
        | ((in_array[21 + inpos]) << 16);
    out_array[11 + outpos] = (in_array[22 + inpos] & 65535)
        | ((in_array[23 + inpos]) << 16);
    out_array[12 + outpos] = (in_array[24 + inpos] & 65535)
        | ((in_array[25 + inpos]) << 16);
    out_array[13 + outpos] = (in_array[26 + inpos] & 65535)
        | ((in_array[27 + inpos]) << 16);
    out_array[14 + outpos] = (in_array[28 + inpos] & 65535)
        | ((in_array[29 + inpos]) << 16);
    out_array[15 + outpos] = (in_array[30 + inpos] & 65535)
        | ((in_array[31 + inpos]) << 16);
}

pub fn fastpack17(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 131071)
        | ((in_array[1 + inpos]) << 17);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 131071) >> (17 - 2))
        | ((in_array[2 + inpos] & 131071) << 2)
        | ((in_array[3 + inpos]) << 19);
    out_array[2 + outpos] = ((in_array[3 + inpos] & 131071) >> (17 - 4))
        | ((in_array[4 + inpos] & 131071) << 4)
        | ((in_array[5 + inpos]) << 21);
    out_array[3 + outpos] = ((in_array[5 + inpos] & 131071) >> (17 - 6))
        | ((in_array[6 + inpos] & 131071) << 6)
        | ((in_array[7 + inpos]) << 23);
    out_array[4 + outpos] = ((in_array[7 + inpos] & 131071) >> (17 - 8))
        | ((in_array[8 + inpos] & 131071) << 8)
        | ((in_array[9 + inpos]) << 25);
    out_array[5 + outpos] = ((in_array[9 + inpos] & 131071) >> (17 - 10))
        | ((in_array[10 + inpos] & 131071) << 10)
        | ((in_array[11 + inpos]) << 27);
    out_array[6 + outpos] = ((in_array[11 + inpos] & 131071) >> (17 - 12))
        | ((in_array[12 + inpos] & 131071) << 12)
        | ((in_array[13 + inpos]) << 29);
    out_array[7 + outpos] = ((in_array[13 + inpos] & 131071) >> (17 - 14))
        | ((in_array[14 + inpos] & 131071) << 14)
        | ((in_array[15 + inpos]) << 31);
    out_array[8 + outpos] = ((in_array[15 + inpos] & 131071) >> (17 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[9 + outpos] = ((in_array[16 + inpos] & 131071) >> (17 - 1))
        | ((in_array[17 + inpos] & 131071) << 1)
        | ((in_array[18 + inpos]) << 18);
    out_array[10 + outpos] = ((in_array[18 + inpos] & 131071) >> (17 - 3))
        | ((in_array[19 + inpos] & 131071) << 3)
        | ((in_array[20 + inpos]) << 20);
    out_array[11 + outpos] = ((in_array[20 + inpos] & 131071) >> (17 - 5))
        | ((in_array[21 + inpos] & 131071) << 5)
        | ((in_array[22 + inpos]) << 22);
    out_array[12 + outpos] = ((in_array[22 + inpos] & 131071) >> (17 - 7))
        | ((in_array[23 + inpos] & 131071) << 7)
        | ((in_array[24 + inpos]) << 24);
    out_array[13 + outpos] = ((in_array[24 + inpos] & 131071) >> (17 - 9))
        | ((in_array[25 + inpos] & 131071) << 9)
        | ((in_array[26 + inpos]) << 26);
    out_array[14 + outpos] = ((in_array[26 + inpos] & 131071) >> (17 - 11))
        | ((in_array[27 + inpos] & 131071) << 11)
        | ((in_array[28 + inpos]) << 28);
    out_array[15 + outpos] = ((in_array[28 + inpos] & 131071) >> (17 - 13))
        | ((in_array[29 + inpos] & 131071) << 13)
        | ((in_array[30 + inpos]) << 30);
    out_array[16 + outpos] = ((in_array[30 + inpos] & 131071) >> (17 - 15))
        | ((in_array[31 + inpos]) << 15);
}

pub fn fastpack18(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 262143)
        | ((in_array[1 + inpos]) << 18);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 262143) >> (18 - 4))
        | ((in_array[2 + inpos] & 262143) << 4)
        | ((in_array[3 + inpos]) << 22);
    out_array[2 + outpos] = ((in_array[3 + inpos] & 262143) >> (18 - 8))
        | ((in_array[4 + inpos] & 262143) << 8)
        | ((in_array[5 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[5 + inpos] & 262143) >> (18 - 12))
        | ((in_array[6 + inpos] & 262143) << 12)
        | ((in_array[7 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[7 + inpos] & 262143) >> (18 - 16))
        | ((in_array[8 + inpos]) << 16);
    out_array[5 + outpos] = ((in_array[8 + inpos] & 262143) >> (18 - 2))
        | ((in_array[9 + inpos] & 262143) << 2)
        | ((in_array[10 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[10 + inpos] & 262143) >> (18 - 6))
        | ((in_array[11 + inpos] & 262143) << 6)
        | ((in_array[12 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[12 + inpos] & 262143) >> (18 - 10))
        | ((in_array[13 + inpos] & 262143) << 10)
        | ((in_array[14 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[14 + inpos] & 262143) >> (18 - 14))
        | ((in_array[15 + inpos]) << 14);
    out_array[9 + outpos] = (in_array[16 + inpos] & 262143)
        | ((in_array[17 + inpos]) << 18);
    out_array[10 + outpos] = ((in_array[17 + inpos] & 262143) >> (18 - 4))
        | ((in_array[18 + inpos] & 262143) << 4)
        | ((in_array[19 + inpos]) << 22);
    out_array[11 + outpos] = ((in_array[19 + inpos] & 262143) >> (18 - 8))
        | ((in_array[20 + inpos] & 262143) << 8)
        | ((in_array[21 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[21 + inpos] & 262143) >> (18 - 12))
        | ((in_array[22 + inpos] & 262143) << 12)
        | ((in_array[23 + inpos]) << 30);
    out_array[13 + outpos] = ((in_array[23 + inpos] & 262143) >> (18 - 16))
        | ((in_array[24 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[24 + inpos] & 262143) >> (18 - 2))
        | ((in_array[25 + inpos] & 262143) << 2)
        | ((in_array[26 + inpos]) << 20);
    out_array[15 + outpos] = ((in_array[26 + inpos] & 262143) >> (18 - 6))
        | ((in_array[27 + inpos] & 262143) << 6)
        | ((in_array[28 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[28 + inpos] & 262143) >> (18 - 10))
        | ((in_array[29 + inpos] & 262143) << 10)
        | ((in_array[30 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[30 + inpos] & 262143) >> (18 - 14))
        | ((in_array[31 + inpos]) << 14);
}

pub fn fastpack19(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 524287)
        | ((in_array[1 + inpos]) << 19);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 524287) >> (19 - 6))
        | ((in_array[2 + inpos] & 524287) << 6)
        | ((in_array[3 + inpos]) << 25);
    out_array[2 + outpos] = ((in_array[3 + inpos] & 524287) >> (19 - 12))
        | ((in_array[4 + inpos] & 524287) << 12)
        | ((in_array[5 + inpos]) << 31);
    out_array[3 + outpos] = ((in_array[5 + inpos] & 524287) >> (19 - 18))
        | ((in_array[6 + inpos]) << 18);
    out_array[4 + outpos] = ((in_array[6 + inpos] & 524287) >> (19 - 5))
        | ((in_array[7 + inpos] & 524287) << 5)
        | ((in_array[8 + inpos]) << 24);
    out_array[5 + outpos] = ((in_array[8 + inpos] & 524287) >> (19 - 11))
        | ((in_array[9 + inpos] & 524287) << 11)
        | ((in_array[10 + inpos]) << 30);
    out_array[6 + outpos] = ((in_array[10 + inpos] & 524287) >> (19 - 17))
        | ((in_array[11 + inpos]) << 17);
    out_array[7 + outpos] = ((in_array[11 + inpos] & 524287) >> (19 - 4))
        | ((in_array[12 + inpos] & 524287) << 4)
        | ((in_array[13 + inpos]) << 23);
    out_array[8 + outpos] = ((in_array[13 + inpos] & 524287) >> (19 - 10))
        | ((in_array[14 + inpos] & 524287) << 10)
        | ((in_array[15 + inpos]) << 29);
    out_array[9 + outpos] = ((in_array[15 + inpos] & 524287) >> (19 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[10 + outpos] = ((in_array[16 + inpos] & 524287) >> (19 - 3))
        | ((in_array[17 + inpos] & 524287) << 3)
        | ((in_array[18 + inpos]) << 22);
    out_array[11 + outpos] = ((in_array[18 + inpos] & 524287) >> (19 - 9))
        | ((in_array[19 + inpos] & 524287) << 9)
        | ((in_array[20 + inpos]) << 28);
    out_array[12 + outpos] = ((in_array[20 + inpos] & 524287) >> (19 - 15))
        | ((in_array[21 + inpos]) << 15);
    out_array[13 + outpos] = ((in_array[21 + inpos] & 524287) >> (19 - 2))
        | ((in_array[22 + inpos] & 524287) << 2)
        | ((in_array[23 + inpos]) << 21);
    out_array[14 + outpos] = ((in_array[23 + inpos] & 524287) >> (19 - 8))
        | ((in_array[24 + inpos] & 524287) << 8)
        | ((in_array[25 + inpos]) << 27);
    out_array[15 + outpos] = ((in_array[25 + inpos] & 524287) >> (19 - 14))
        | ((in_array[26 + inpos]) << 14);
    out_array[16 + outpos] = ((in_array[26 + inpos] & 524287) >> (19 - 1))
        | ((in_array[27 + inpos] & 524287) << 1)
        | ((in_array[28 + inpos]) << 20);
    out_array[17 + outpos] = ((in_array[28 + inpos] & 524287) >> (19 - 7))
        | ((in_array[29 + inpos] & 524287) << 7)
        | ((in_array[30 + inpos]) << 26);
    out_array[18 + outpos] = ((in_array[30 + inpos] & 524287) >> (19 - 13))
        | ((in_array[31 + inpos]) << 13);
}

pub fn fastpack2(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = in_array[inpos] & 3
        | (in_array[1 + inpos] & 3) << 2
        | (in_array[2 + inpos] & 3) << 4
        | (in_array[3 + inpos] & 3) << 6
        | (in_array[4 + inpos] & 3) << 8
        | (in_array[5 + inpos] & 3) << 10
        | (in_array[6 + inpos] & 3) << 12
        | (in_array[7 + inpos] & 3) << 14
        | (in_array[8 + inpos] & 3) << 16
        | (in_array[9 + inpos] & 3) << 18
        | (in_array[10 + inpos] & 3) << 20
        | (in_array[11 + inpos] & 3) << 22
        | (in_array[12 + inpos] & 3) << 24
        | (in_array[13 + inpos] & 3) << 26
        | (in_array[14 + inpos] & 3) << 28
        | (in_array[15 + inpos]) << 30;
    out_array[1 + outpos] = in_array[16 + inpos] & 3
        | (in_array[17 + inpos] & 3) << 2
        | (in_array[18 + inpos] & 3) << 4
        | (in_array[19 + inpos] & 3) << 6
        | (in_array[20 + inpos] & 3) << 8
        | (in_array[21 + inpos] & 3) << 10
        | (in_array[22 + inpos] & 3) << 12
        | (in_array[23 + inpos] & 3) << 14
        | (in_array[24 + inpos] & 3) << 16
        | (in_array[25 + inpos] & 3) << 18
        | (in_array[26 + inpos] & 3) << 20
        | (in_array[27 + inpos] & 3) << 22
        | (in_array[28 + inpos] & 3) << 24
        | (in_array[29 + inpos] & 3) << 26
        | (in_array[30 + inpos] & 3) << 28
        | in_array[31 + inpos] << 30;

    // out[1 + outpos] = in[16 + inpos] & 3 
    //     | (in[17 + inpos] & 3) << 2 
    //     | (in[18 + inpos] & 3) << 4 
    //     | (in[19 + inpos] & 3) << 6 
    //     | (in[20 + inpos] & 3) << 8 
    //     | (in[21 + inpos] & 3) << 10 
    //     | (in[22 + inpos] & 3) << 12 
    //     | (in[23 + inpos] & 3) << 14 
    //     | (in[24 + inpos] & 3) << 16 
    //     | (in[25 + inpos] & 3) << 18 
    //     | (in[26 + inpos] & 3) << 20 
    //     | (in[27 + inpos] & 3) << 22 
    //     | (in[28 + inpos] & 3) << 24 
    //     | (in[29 + inpos] & 3) << 26 
    //     | (in[30 + inpos] & 3) << 28 
    //     | in[31 + inpos] << 30;

}

pub fn fastpack20(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 1048575)
        | ((in_array[1 + inpos]) << 20);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 1048575) >> (20 - 8))
        | ((in_array[2 + inpos] & 1048575) << 8)
        | ((in_array[3 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[3 + inpos] & 1048575) >> (20 - 16))
        | ((in_array[4 + inpos]) << 16);
    out_array[3 + outpos] = ((in_array[4 + inpos] & 1048575) >> (20 - 4))
        | ((in_array[5 + inpos] & 1048575) << 4)
        | ((in_array[6 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[6 + inpos] & 1048575) >> (20 - 12))
        | ((in_array[7 + inpos]) << 12);
    out_array[5 + outpos] = (in_array[8 + inpos] & 1048575)
        | ((in_array[9 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[9 + inpos] & 1048575) >> (20 - 8))
        | ((in_array[10 + inpos] & 1048575) << 8)
        | ((in_array[11 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[11 + inpos] & 1048575) >> (20 - 16))
        | ((in_array[12 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[12 + inpos] & 1048575) >> (20 - 4))
        | ((in_array[13 + inpos] & 1048575) << 4)
        | ((in_array[14 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[14 + inpos] & 1048575) >> (20 - 12))
        | ((in_array[15 + inpos]) << 12);
    out_array[10 + outpos] = (in_array[16 + inpos] & 1048575)
        | ((in_array[17 + inpos]) << 20);
    out_array[11 + outpos] = ((in_array[17 + inpos] & 1048575) >> (20 - 8))
        | ((in_array[18 + inpos] & 1048575) << 8)
        | ((in_array[19 + inpos]) << 28);
    out_array[12 + outpos] = ((in_array[19 + inpos] & 1048575) >> (20 - 16))
        | ((in_array[20 + inpos]) << 16);
    out_array[13 + outpos] = ((in_array[20 + inpos] & 1048575) >> (20 - 4))
        | ((in_array[21 + inpos] & 1048575) << 4)
        | ((in_array[22 + inpos]) << 24);
    out_array[14 + outpos] = ((in_array[22 + inpos] & 1048575) >> (20 - 12))
        | ((in_array[23 + inpos]) << 12);
    out_array[15 + outpos] = (in_array[24 + inpos] & 1048575)
        | ((in_array[25 + inpos]) << 20);
    out_array[16 + outpos] = ((in_array[25 + inpos] & 1048575) >> (20 - 8))
        | ((in_array[26 + inpos] & 1048575) << 8)
        | ((in_array[27 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[27 + inpos] & 1048575) >> (20 - 16))
        | ((in_array[28 + inpos]) << 16);
    out_array[18 + outpos] = ((in_array[28 + inpos] & 1048575) >> (20 - 4))
        | ((in_array[29 + inpos] & 1048575) << 4)
        | ((in_array[30 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[30 + inpos] & 1048575) >> (20 - 12))
        | ((in_array[31 + inpos]) << 12);
}

pub fn fastpack21(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 2097151)
        | ((in_array[1 + inpos]) << 21);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 2097151) >> (21 - 10))
        | ((in_array[2 + inpos] & 2097151) << 10)
        | ((in_array[3 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[3 + inpos] & 2097151) >> (21 - 20))
        | ((in_array[4 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[4 + inpos] & 2097151) >> (21 - 9))
        | ((in_array[5 + inpos] & 2097151) << 9)
        | ((in_array[6 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[6 + inpos] & 2097151) >> (21 - 19))
        | ((in_array[7 + inpos]) << 19);
    out_array[5 + outpos] = ((in_array[7 + inpos] & 2097151) >> (21 - 8))
        | ((in_array[8 + inpos] & 2097151) << 8)
        | ((in_array[9 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[9 + inpos] & 2097151) >> (21 - 18))
        | ((in_array[10 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[10 + inpos] & 2097151) >> (21 - 7))
        | ((in_array[11 + inpos] & 2097151) << 7)
        | ((in_array[12 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[12 + inpos] & 2097151) >> (21 - 17))
        | ((in_array[13 + inpos]) << 17);
    out_array[9 + outpos] = ((in_array[13 + inpos] & 2097151) >> (21 - 6))
        | ((in_array[14 + inpos] & 2097151) << 6)
        | ((in_array[15 + inpos]) << 27);
    out_array[10 + outpos] = ((in_array[15 + inpos] & 2097151) >> (21 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[16 + inpos] & 2097151) >> (21 - 5))
        | ((in_array[17 + inpos] & 2097151) << 5)
        | ((in_array[18 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[18 + inpos] & 2097151) >> (21 - 15))
        | ((in_array[19 + inpos]) << 15);
    out_array[13 + outpos] = ((in_array[19 + inpos] & 2097151) >> (21 - 4))
        | ((in_array[20 + inpos] & 2097151) << 4)
        | ((in_array[21 + inpos]) << 25);
    out_array[14 + outpos] = ((in_array[21 + inpos] & 2097151) >> (21 - 14))
        | ((in_array[22 + inpos]) << 14);
    out_array[15 + outpos] = ((in_array[22 + inpos] & 2097151) >> (21 - 3))
        | ((in_array[23 + inpos] & 2097151) << 3)
        | ((in_array[24 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[24 + inpos] & 2097151) >> (21 - 13))
        | ((in_array[25 + inpos]) << 13);
    out_array[17 + outpos] = ((in_array[25 + inpos] & 2097151) >> (21 - 2))
        | ((in_array[26 + inpos] & 2097151) << 2)
        | ((in_array[27 + inpos]) << 23);
    out_array[18 + outpos] = ((in_array[27 + inpos] & 2097151) >> (21 - 12))
        | ((in_array[28 + inpos]) << 12);
    out_array[19 + outpos] = ((in_array[28 + inpos] & 2097151) >> (21 - 1))
        | ((in_array[29 + inpos] & 2097151) << 1)
        | ((in_array[30 + inpos]) << 22);
    out_array[20 + outpos] = ((in_array[30 + inpos] & 2097151) >> (21 - 11))
        | ((in_array[31 + inpos]) << 11);
}

pub fn fastpack22(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 4194303)
        | ((in_array[1 + inpos]) << 22);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 4194303) >> (22 - 12))
        | ((in_array[2 + inpos]) << 12);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 4194303) >> (22 - 2))
        | ((in_array[3 + inpos] & 4194303) << 2)
        | ((in_array[4 + inpos]) << 24);
    out_array[3 + outpos] = ((in_array[4 + inpos] & 4194303) >> (22 - 14))
        | ((in_array[5 + inpos]) << 14);
    out_array[4 + outpos] = ((in_array[5 + inpos] & 4194303) >> (22 - 4))
        | ((in_array[6 + inpos] & 4194303) << 4)
        | ((in_array[7 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[7 + inpos] & 4194303) >> (22 - 16))
        | ((in_array[8 + inpos]) << 16);
    out_array[6 + outpos] = ((in_array[8 + inpos] & 4194303) >> (22 - 6))
        | ((in_array[9 + inpos] & 4194303) << 6)
        | ((in_array[10 + inpos]) << 28);
    out_array[7 + outpos] = ((in_array[10 + inpos] & 4194303) >> (22 - 18))
        | ((in_array[11 + inpos]) << 18);
    out_array[8 + outpos] = ((in_array[11 + inpos] & 4194303) >> (22 - 8))
        | ((in_array[12 + inpos] & 4194303) << 8)
        | ((in_array[13 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[13 + inpos] & 4194303) >> (22 - 20))
        | ((in_array[14 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[14 + inpos] & 4194303) >> (22 - 10))
        | ((in_array[15 + inpos]) << 10);
    out_array[11 + outpos] = (in_array[16 + inpos] & 4194303)
        | ((in_array[17 + inpos]) << 22);
    out_array[12 + outpos] = ((in_array[17 + inpos] & 4194303) >> (22 - 12))
        | ((in_array[18 + inpos]) << 12);
    out_array[13 + outpos] = ((in_array[18 + inpos] & 4194303) >> (22 - 2))
        | ((in_array[19 + inpos] & 4194303) << 2)
        | ((in_array[20 + inpos]) << 24);
    out_array[14 + outpos] = ((in_array[20 + inpos] & 4194303) >> (22 - 14))
        | ((in_array[21 + inpos]) << 14);
    out_array[15 + outpos] = ((in_array[21 + inpos] & 4194303) >> (22 - 4))
        | ((in_array[22 + inpos] & 4194303) << 4)
        | ((in_array[23 + inpos]) << 26);
    out_array[16 + outpos] = ((in_array[23 + inpos] & 4194303) >> (22 - 16))
        | ((in_array[24 + inpos]) << 16);
    out_array[17 + outpos] = ((in_array[24 + inpos] & 4194303) >> (22 - 6))
        | ((in_array[25 + inpos] & 4194303) << 6)
        | ((in_array[26 + inpos]) << 28);
    out_array[18 + outpos] = ((in_array[26 + inpos] & 4194303) >> (22 - 18))
        | ((in_array[27 + inpos]) << 18);
    out_array[19 + outpos] = ((in_array[27 + inpos] & 4194303) >> (22 - 8))
        | ((in_array[28 + inpos] & 4194303) << 8)
        | ((in_array[29 + inpos]) << 30);
    out_array[20 + outpos] = ((in_array[29 + inpos] & 4194303) >> (22 - 20))
        | ((in_array[30 + inpos]) << 20);
    out_array[21 + outpos] = ((in_array[30 + inpos] & 4194303) >> (22 - 10))
        | ((in_array[31 + inpos]) << 10);
}

pub fn fastpack23(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 8388607)
        | ((in_array[1 + inpos]) << 23);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 8388607) >> (23 - 14))
        | ((in_array[2 + inpos]) << 14);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 8388607) >> (23 - 5))
        | ((in_array[3 + inpos] & 8388607) << 5)
        | ((in_array[4 + inpos]) << 28);
    out_array[3 + outpos] = ((in_array[4 + inpos] & 8388607) >> (23 - 19))
        | ((in_array[5 + inpos]) << 19);
    out_array[4 + outpos] = ((in_array[5 + inpos] & 8388607) >> (23 - 10))
        | ((in_array[6 + inpos]) << 10);
    out_array[5 + outpos] = ((in_array[6 + inpos] & 8388607) >> (23 - 1))
        | ((in_array[7 + inpos] & 8388607) << 1)
        | ((in_array[8 + inpos]) << 24);
    out_array[6 + outpos] = ((in_array[8 + inpos] & 8388607) >> (23 - 15))
        | ((in_array[9 + inpos]) << 15);
    out_array[7 + outpos] = ((in_array[9 + inpos] & 8388607) >> (23 - 6))
        | ((in_array[10 + inpos] & 8388607) << 6)
        | ((in_array[11 + inpos]) << 29);
    out_array[8 + outpos] = ((in_array[11 + inpos] & 8388607) >> (23 - 20))
        | ((in_array[12 + inpos]) << 20);
    out_array[9 + outpos] = ((in_array[12 + inpos] & 8388607) >> (23 - 11))
        | ((in_array[13 + inpos]) << 11);
    out_array[10 + outpos] = ((in_array[13 + inpos] & 8388607) >> (23 - 2))
        | ((in_array[14 + inpos] & 8388607) << 2)
        | ((in_array[15 + inpos]) << 25);
    out_array[11 + outpos] = ((in_array[15 + inpos] & 8388607) >> (23 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[12 + outpos] = ((in_array[16 + inpos] & 8388607) >> (23 - 7))
        | ((in_array[17 + inpos] & 8388607) << 7)
        | ((in_array[18 + inpos]) << 30);
    out_array[13 + outpos] = ((in_array[18 + inpos] & 8388607) >> (23 - 21))
        | ((in_array[19 + inpos]) << 21);
    out_array[14 + outpos] = ((in_array[19 + inpos] & 8388607) >> (23 - 12))
        | ((in_array[20 + inpos]) << 12);
    out_array[15 + outpos] = ((in_array[20 + inpos] & 8388607) >> (23 - 3))
        | ((in_array[21 + inpos] & 8388607) << 3)
        | ((in_array[22 + inpos]) << 26);
    out_array[16 + outpos] = ((in_array[22 + inpos] & 8388607) >> (23 - 17))
        | ((in_array[23 + inpos]) << 17);
    out_array[17 + outpos] = ((in_array[23 + inpos] & 8388607) >> (23 - 8))
        | ((in_array[24 + inpos] & 8388607) << 8)
        | ((in_array[25 + inpos]) << 31);
    out_array[18 + outpos] = ((in_array[25 + inpos] & 8388607) >> (23 - 22))
        | ((in_array[26 + inpos]) << 22);
    out_array[19 + outpos] = ((in_array[26 + inpos] & 8388607) >> (23 - 13))
        | ((in_array[27 + inpos]) << 13);
    out_array[20 + outpos] = ((in_array[27 + inpos] & 8388607) >> (23 - 4))
        | ((in_array[28 + inpos] & 8388607) << 4)
        | ((in_array[29 + inpos]) << 27);
    out_array[21 + outpos] = ((in_array[29 + inpos] & 8388607) >> (23 - 18))
        | ((in_array[30 + inpos]) << 18);
    out_array[22 + outpos] = ((in_array[30 + inpos] & 8388607) >> (23 - 9))
        | ((in_array[31 + inpos]) << 9);
}

pub fn fastpack24(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 16777215)
        | ((in_array[1 + inpos]) << 24);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[2 + inpos]) << 16);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[3 + inpos]) << 8);
    out_array[3 + outpos] = (in_array[4 + inpos] & 16777215)
        | ((in_array[5 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[5 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[6 + inpos]) << 16);
    out_array[5 + outpos] = ((in_array[6 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[7 + inpos]) << 8);
    out_array[6 + outpos] = (in_array[8 + inpos] & 16777215)
        | ((in_array[9 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[9 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[10 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[10 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[11 + inpos]) << 8);
    out_array[9 + outpos] = (in_array[12 + inpos] & 16777215)
        | ((in_array[13 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[13 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[14 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[14 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[15 + inpos]) << 8);
    out_array[12 + outpos] = (in_array[16 + inpos] & 16777215)
        | ((in_array[17 + inpos]) << 24);
    out_array[13 + outpos] = ((in_array[17 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[18 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[18 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[19 + inpos]) << 8);
    out_array[15 + outpos] = (in_array[20 + inpos] & 16777215)
        | ((in_array[21 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[21 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[22 + inpos]) << 16);
    out_array[17 + outpos] = ((in_array[22 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[23 + inpos]) << 8);
    out_array[18 + outpos] = (in_array[24 + inpos] & 16777215)
        | ((in_array[25 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[25 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[26 + inpos]) << 16);
    out_array[20 + outpos] = ((in_array[26 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[27 + inpos]) << 8);
    out_array[21 + outpos] = (in_array[28 + inpos] & 16777215)
        | ((in_array[29 + inpos]) << 24);
    out_array[22 + outpos] = ((in_array[29 + inpos] & 16777215) >> (24 - 16))
        | ((in_array[30 + inpos]) << 16);
    out_array[23 + outpos] = ((in_array[30 + inpos] & 16777215) >> (24 - 8))
        | ((in_array[31 + inpos]) << 8);
}

pub fn fastpack25(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 33554431)
        | ((in_array[1 + inpos]) << 25);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 33554431) >> (25 - 18))
        | ((in_array[2 + inpos]) << 18);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 33554431) >> (25 - 11))
        | ((in_array[3 + inpos]) << 11);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 33554431) >> (25 - 4))
        | ((in_array[4 + inpos] & 33554431) << 4)
        | ((in_array[5 + inpos]) << 29);
    out_array[4 + outpos] = ((in_array[5 + inpos] & 33554431) >> (25 - 22))
        | ((in_array[6 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[6 + inpos] & 33554431) >> (25 - 15))
        | ((in_array[7 + inpos]) << 15);
    out_array[6 + outpos] = ((in_array[7 + inpos] & 33554431) >> (25 - 8))
        | ((in_array[8 + inpos]) << 8);
    out_array[7 + outpos] = ((in_array[8 + inpos] & 33554431) >> (25 - 1))
        | ((in_array[9 + inpos] & 33554431) << 1)
        | ((in_array[10 + inpos]) << 26);
    out_array[8 + outpos] = ((in_array[10 + inpos] & 33554431) >> (25 - 19))
        | ((in_array[11 + inpos]) << 19);
    out_array[9 + outpos] = ((in_array[11 + inpos] & 33554431) >> (25 - 12))
        | ((in_array[12 + inpos]) << 12);
    out_array[10 + outpos] = ((in_array[12 + inpos] & 33554431) >> (25 - 5))
        | ((in_array[13 + inpos] & 33554431) << 5)
        | ((in_array[14 + inpos]) << 30);
    out_array[11 + outpos] = ((in_array[14 + inpos] & 33554431) >> (25 - 23))
        | ((in_array[15 + inpos]) << 23);
    out_array[12 + outpos] = ((in_array[15 + inpos] & 33554431) >> (25 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[13 + outpos] = ((in_array[16 + inpos] & 33554431) >> (25 - 9))
        | ((in_array[17 + inpos]) << 9);
    out_array[14 + outpos] = ((in_array[17 + inpos] & 33554431) >> (25 - 2))
        | ((in_array[18 + inpos] & 33554431) << 2)
        | ((in_array[19 + inpos]) << 27);
    out_array[15 + outpos] = ((in_array[19 + inpos] & 33554431) >> (25 - 20))
        | ((in_array[20 + inpos]) << 20);
    out_array[16 + outpos] = ((in_array[20 + inpos] & 33554431) >> (25 - 13))
        | ((in_array[21 + inpos]) << 13);
    out_array[17 + outpos] = ((in_array[21 + inpos] & 33554431) >> (25 - 6))
        | ((in_array[22 + inpos] & 33554431) << 6)
        | ((in_array[23 + inpos]) << 31);
    out_array[18 + outpos] = ((in_array[23 + inpos] & 33554431) >> (25 - 24))
        | ((in_array[24 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[24 + inpos] & 33554431) >> (25 - 17))
        | ((in_array[25 + inpos]) << 17);
    out_array[20 + outpos] = ((in_array[25 + inpos] & 33554431) >> (25 - 10))
        | ((in_array[26 + inpos]) << 10);
    out_array[21 + outpos] = ((in_array[26 + inpos] & 33554431) >> (25 - 3))
        | ((in_array[27 + inpos] & 33554431) << 3)
        | ((in_array[28 + inpos]) << 28);
    out_array[22 + outpos] = ((in_array[28 + inpos] & 33554431) >> (25 - 21))
        | ((in_array[29 + inpos]) << 21);
    out_array[23 + outpos] = ((in_array[29 + inpos] & 33554431) >> (25 - 14))
        | ((in_array[30 + inpos]) << 14);
    out_array[24 + outpos] = ((in_array[30 + inpos] & 33554431) >> (25 - 7))
        | ((in_array[31 + inpos]) << 7);
}

pub fn fastpack26(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 67108863)
        | ((in_array[1 + inpos]) << 26);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 67108863) >> (26 - 20))
        | ((in_array[2 + inpos]) << 20);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 67108863) >> (26 - 14))
        | ((in_array[3 + inpos]) << 14);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 67108863) >> (26 - 8))
        | ((in_array[4 + inpos]) << 8);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 67108863) >> (26 - 2))
        | ((in_array[5 + inpos] & 67108863) << 2)
        | ((in_array[6 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[6 + inpos] & 67108863) >> (26 - 22))
        | ((in_array[7 + inpos]) << 22);
    out_array[6 + outpos] = ((in_array[7 + inpos] & 67108863) >> (26 - 16))
        | ((in_array[8 + inpos]) << 16);
    out_array[7 + outpos] = ((in_array[8 + inpos] & 67108863) >> (26 - 10))
        | ((in_array[9 + inpos]) << 10);
    out_array[8 + outpos] = ((in_array[9 + inpos] & 67108863) >> (26 - 4))
        | ((in_array[10 + inpos] & 67108863) << 4)
        | ((in_array[11 + inpos]) << 30);
    out_array[9 + outpos] = ((in_array[11 + inpos] & 67108863) >> (26 - 24))
        | ((in_array[12 + inpos]) << 24);
    out_array[10 + outpos] = ((in_array[12 + inpos] & 67108863) >> (26 - 18))
        | ((in_array[13 + inpos]) << 18);
    out_array[11 + outpos] = ((in_array[13 + inpos] & 67108863) >> (26 - 12))
        | ((in_array[14 + inpos]) << 12);
    out_array[12 + outpos] = ((in_array[14 + inpos] & 67108863) >> (26 - 6))
        | ((in_array[15 + inpos]) << 6);
    out_array[13 + outpos] = (in_array[16 + inpos] & 67108863)
        | ((in_array[17 + inpos]) << 26);
    out_array[14 + outpos] = ((in_array[17 + inpos] & 67108863) >> (26 - 20))
        | ((in_array[18 + inpos]) << 20);
    out_array[15 + outpos] = ((in_array[18 + inpos] & 67108863) >> (26 - 14))
        | ((in_array[19 + inpos]) << 14);
    out_array[16 + outpos] = ((in_array[19 + inpos] & 67108863) >> (26 - 8))
        | ((in_array[20 + inpos]) << 8);
    out_array[17 + outpos] = ((in_array[20 + inpos] & 67108863) >> (26 - 2))
        | ((in_array[21 + inpos] & 67108863) << 2)
        | ((in_array[22 + inpos]) << 28);
    out_array[18 + outpos] = ((in_array[22 + inpos] & 67108863) >> (26 - 22))
        | ((in_array[23 + inpos]) << 22);
    out_array[19 + outpos] = ((in_array[23 + inpos] & 67108863) >> (26 - 16))
        | ((in_array[24 + inpos]) << 16);
    out_array[20 + outpos] = ((in_array[24 + inpos] & 67108863) >> (26 - 10))
        | ((in_array[25 + inpos]) << 10);
    out_array[21 + outpos] = ((in_array[25 + inpos] & 67108863) >> (26 - 4))
        | ((in_array[26 + inpos] & 67108863) << 4)
        | ((in_array[27 + inpos]) << 30);
    out_array[22 + outpos] = ((in_array[27 + inpos] & 67108863) >> (26 - 24))
        | ((in_array[28 + inpos]) << 24);
    out_array[23 + outpos] = ((in_array[28 + inpos] & 67108863) >> (26 - 18))
        | ((in_array[29 + inpos]) << 18);
    out_array[24 + outpos] = ((in_array[29 + inpos] & 67108863) >> (26 - 12))
        | ((in_array[30 + inpos]) << 12);
    out_array[25 + outpos] = ((in_array[30 + inpos] & 67108863) >> (26 - 6))
        | ((in_array[31 + inpos]) << 6);
}

pub fn fastpack27(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 134217727)
        | ((in_array[1 + inpos]) << 27);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 134217727) >> (27 - 22))
        | ((in_array[2 + inpos]) << 22);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 134217727) >> (27 - 17))
        | ((in_array[3 + inpos]) << 17);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 134217727) >> (27 - 12))
        | ((in_array[4 + inpos]) << 12);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 134217727) >> (27 - 7))
        | ((in_array[5 + inpos]) << 7);
    out_array[5 + outpos] = ((in_array[5 + inpos] & 134217727) >> (27 - 2))
        | ((in_array[6 + inpos] & 134217727) << 2)
        | ((in_array[7 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[7 + inpos] & 134217727) >> (27 - 24))
        | ((in_array[8 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[8 + inpos] & 134217727) >> (27 - 19))
        | ((in_array[9 + inpos]) << 19);
    out_array[8 + outpos] = ((in_array[9 + inpos] & 134217727) >> (27 - 14))
        | ((in_array[10 + inpos]) << 14);
    out_array[9 + outpos] = ((in_array[10 + inpos] & 134217727) >> (27 - 9))
        | ((in_array[11 + inpos]) << 9);
    out_array[10 + outpos] = ((in_array[11 + inpos] & 134217727) >> (27 - 4))
        | ((in_array[12 + inpos] & 134217727) << 4)
        | ((in_array[13 + inpos]) << 31);
    out_array[11 + outpos] = ((in_array[13 + inpos] & 134217727) >> (27 - 26))
        | ((in_array[14 + inpos]) << 26);
    out_array[12 + outpos] = ((in_array[14 + inpos] & 134217727) >> (27 - 21))
        | ((in_array[15 + inpos]) << 21);
    out_array[13 + outpos] = ((in_array[15 + inpos] & 134217727) >> (27 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[14 + outpos] = ((in_array[16 + inpos] & 134217727) >> (27 - 11))
        | ((in_array[17 + inpos]) << 11);
    out_array[15 + outpos] = ((in_array[17 + inpos] & 134217727) >> (27 - 6))
        | ((in_array[18 + inpos]) << 6);
    out_array[16 + outpos] = ((in_array[18 + inpos] & 134217727) >> (27 - 1))
        | ((in_array[19 + inpos] & 134217727) << 1)
        | ((in_array[20 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[20 + inpos] & 134217727) >> (27 - 23))
        | ((in_array[21 + inpos]) << 23);
    out_array[18 + outpos] = ((in_array[21 + inpos] & 134217727) >> (27 - 18))
        | ((in_array[22 + inpos]) << 18);
    out_array[19 + outpos] = ((in_array[22 + inpos] & 134217727) >> (27 - 13))
        | ((in_array[23 + inpos]) << 13);
    out_array[20 + outpos] = ((in_array[23 + inpos] & 134217727) >> (27 - 8))
        | ((in_array[24 + inpos]) << 8);
    out_array[21 + outpos] = ((in_array[24 + inpos] & 134217727) >> (27 - 3))
        | ((in_array[25 + inpos] & 134217727) << 3)
        | ((in_array[26 + inpos]) << 30);
    out_array[22 + outpos] = ((in_array[26 + inpos] & 134217727) >> (27 - 25))
        | ((in_array[27 + inpos]) << 25);
    out_array[23 + outpos] = ((in_array[27 + inpos] & 134217727) >> (27 - 20))
        | ((in_array[28 + inpos]) << 20);
    out_array[24 + outpos] = ((in_array[28 + inpos] & 134217727) >> (27 - 15))
        | ((in_array[29 + inpos]) << 15);
    out_array[25 + outpos] = ((in_array[29 + inpos] & 134217727) >> (27 - 10))
        | ((in_array[30 + inpos]) << 10);
    out_array[26 + outpos] = ((in_array[30 + inpos] & 134217727) >> (27 - 5))
        | ((in_array[31 + inpos]) << 5);
}

pub fn fastpack28(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 268435455)
        | ((in_array[1 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 268435455) >> (28 - 24))
        | ((in_array[2 + inpos]) << 24);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 268435455) >> (28 - 20))
        | ((in_array[3 + inpos]) << 20);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 268435455) >> (28 - 16))
        | ((in_array[4 + inpos]) << 16);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 268435455) >> (28 - 12))
        | ((in_array[5 + inpos]) << 12);
    out_array[5 + outpos] = ((in_array[5 + inpos] & 268435455) >> (28 - 8))
        | ((in_array[6 + inpos]) << 8);
    out_array[6 + outpos] = ((in_array[6 + inpos] & 268435455) >> (28 - 4))
        | ((in_array[7 + inpos]) << 4);
    out_array[7 + outpos] = (in_array[8 + inpos] & 268435455)
        | ((in_array[9 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[9 + inpos] & 268435455) >> (28 - 24))
        | ((in_array[10 + inpos]) << 24);
    out_array[9 + outpos] = ((in_array[10 + inpos] & 268435455) >> (28 - 20))
        | ((in_array[11 + inpos]) << 20);
    out_array[10 + outpos] = ((in_array[11 + inpos] & 268435455) >> (28 - 16))
        | ((in_array[12 + inpos]) << 16);
    out_array[11 + outpos] = ((in_array[12 + inpos] & 268435455) >> (28 - 12))
        | ((in_array[13 + inpos]) << 12);
    out_array[12 + outpos] = ((in_array[13 + inpos] & 268435455) >> (28 - 8))
        | ((in_array[14 + inpos]) << 8);
    out_array[13 + outpos] = ((in_array[14 + inpos] & 268435455) >> (28 - 4))
        | ((in_array[15 + inpos]) << 4);
    out_array[14 + outpos] = (in_array[16 + inpos] & 268435455)
        | ((in_array[17 + inpos]) << 28);
    out_array[15 + outpos] = ((in_array[17 + inpos] & 268435455) >> (28 - 24))
        | ((in_array[18 + inpos]) << 24);
    out_array[16 + outpos] = ((in_array[18 + inpos] & 268435455) >> (28 - 20))
        | ((in_array[19 + inpos]) << 20);
    out_array[17 + outpos] = ((in_array[19 + inpos] & 268435455) >> (28 - 16))
        | ((in_array[20 + inpos]) << 16);
    out_array[18 + outpos] = ((in_array[20 + inpos] & 268435455) >> (28 - 12))
        | ((in_array[21 + inpos]) << 12);
    out_array[19 + outpos] = ((in_array[21 + inpos] & 268435455) >> (28 - 8))
        | ((in_array[22 + inpos]) << 8);
    out_array[20 + outpos] = ((in_array[22 + inpos] & 268435455) >> (28 - 4))
        | ((in_array[23 + inpos]) << 4);
    out_array[21 + outpos] = (in_array[24 + inpos] & 268435455)
        | ((in_array[25 + inpos]) << 28);
    out_array[22 + outpos] = ((in_array[25 + inpos] & 268435455) >> (28 - 24))
        | ((in_array[26 + inpos]) << 24);
    out_array[23 + outpos] = ((in_array[26 + inpos] & 268435455) >> (28 - 20))
        | ((in_array[27 + inpos]) << 20);
    out_array[24 + outpos] = ((in_array[27 + inpos] & 268435455) >> (28 - 16))
        | ((in_array[28 + inpos]) << 16);
    out_array[25 + outpos] = ((in_array[28 + inpos] & 268435455) >> (28 - 12))
        | ((in_array[29 + inpos]) << 12);
    out_array[26 + outpos] = ((in_array[29 + inpos] & 268435455) >> (28 - 8))
        | ((in_array[30 + inpos]) << 8);
    out_array[27 + outpos] = ((in_array[30 + inpos] & 268435455) >> (28 - 4))
        | ((in_array[31 + inpos]) << 4);
}

pub fn fastpack29(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 536870911)
        | ((in_array[1 + inpos]) << 29);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 536870911) >> (29 - 26))
        | ((in_array[2 + inpos]) << 26);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 536870911) >> (29 - 23))
        | ((in_array[3 + inpos]) << 23);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 536870911) >> (29 - 20))
        | ((in_array[4 + inpos]) << 20);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 536870911) >> (29 - 17))
        | ((in_array[5 + inpos]) << 17);
    out_array[5 + outpos] = ((in_array[5 + inpos] & 536870911) >> (29 - 14))
        | ((in_array[6 + inpos]) << 14);
    out_array[6 + outpos] = ((in_array[6 + inpos] & 536870911) >> (29 - 11))
        | ((in_array[7 + inpos]) << 11);
    out_array[7 + outpos] = ((in_array[7 + inpos] & 536870911) >> (29 - 8))
        | ((in_array[8 + inpos]) << 8);
    out_array[8 + outpos] = ((in_array[8 + inpos] & 536870911) >> (29 - 5))
        | ((in_array[9 + inpos]) << 5);
    out_array[9 + outpos] = ((in_array[9 + inpos] & 536870911) >> (29 - 2))
        | ((in_array[10 + inpos] & 536870911) << 2)
        | ((in_array[11 + inpos]) << 31);
    out_array[10 + outpos] = ((in_array[11 + inpos] & 536870911) >> (29 - 28))
        | ((in_array[12 + inpos]) << 28);
    out_array[11 + outpos] = ((in_array[12 + inpos] & 536870911) >> (29 - 25))
        | ((in_array[13 + inpos]) << 25);
    out_array[12 + outpos] = ((in_array[13 + inpos] & 536870911) >> (29 - 22))
        | ((in_array[14 + inpos]) << 22);
    out_array[13 + outpos] = ((in_array[14 + inpos] & 536870911) >> (29 - 19))
        | ((in_array[15 + inpos]) << 19);
    out_array[14 + outpos] = ((in_array[15 + inpos] & 536870911) >> (29 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[15 + outpos] = ((in_array[16 + inpos] & 536870911) >> (29 - 13))
        | ((in_array[17 + inpos]) << 13);
    out_array[16 + outpos] = ((in_array[17 + inpos] & 536870911) >> (29 - 10))
        | ((in_array[18 + inpos]) << 10);
    out_array[17 + outpos] = ((in_array[18 + inpos] & 536870911) >> (29 - 7))
        | ((in_array[19 + inpos]) << 7);
    out_array[18 + outpos] = ((in_array[19 + inpos] & 536870911) >> (29 - 4))
        | ((in_array[20 + inpos]) << 4);
    out_array[19 + outpos] = ((in_array[20 + inpos] & 536870911) >> (29 - 1))
        | ((in_array[21 + inpos] & 536870911) << 1)
        | ((in_array[22 + inpos]) << 30);
    out_array[20 + outpos] = ((in_array[22 + inpos] & 536870911) >> (29 - 27))
        | ((in_array[23 + inpos]) << 27);
    out_array[21 + outpos] = ((in_array[23 + inpos] & 536870911) >> (29 - 24))
        | ((in_array[24 + inpos]) << 24);
    out_array[22 + outpos] = ((in_array[24 + inpos] & 536870911) >> (29 - 21))
        | ((in_array[25 + inpos]) << 21);
    out_array[23 + outpos] = ((in_array[25 + inpos] & 536870911) >> (29 - 18))
        | ((in_array[26 + inpos]) << 18);
    out_array[24 + outpos] = ((in_array[26 + inpos] & 536870911) >> (29 - 15))
        | ((in_array[27 + inpos]) << 15);
    out_array[25 + outpos] = ((in_array[27 + inpos] & 536870911) >> (29 - 12))
        | ((in_array[28 + inpos]) << 12);
    out_array[26 + outpos] = ((in_array[28 + inpos] & 536870911) >> (29 - 9))
        | ((in_array[29 + inpos]) << 9);
    out_array[27 + outpos] = ((in_array[29 + inpos] & 536870911) >> (29 - 6))
        | ((in_array[30 + inpos]) << 6);
    out_array[28 + outpos] = ((in_array[30 + inpos] & 536870911) >> (29 - 3))
        | ((in_array[31 + inpos]) << 3);
}

pub fn fastpack3(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 7)
        | ((in_array[1 + inpos] & 7) << 3)
        | ((in_array[2 + inpos] & 7) << 6)
        | ((in_array[3 + inpos] & 7) << 9)
        | ((in_array[4 + inpos] & 7) << 12)
        | ((in_array[5 + inpos] & 7) << 15)
        | ((in_array[6 + inpos] & 7) << 18)
        | ((in_array[7 + inpos] & 7) << 21)
        | ((in_array[8 + inpos] & 7) << 24)
        | ((in_array[9 + inpos] & 7) << 27)
        | ((in_array[10 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[10 + inpos] & 7) >> (3 - 1))
        | ((in_array[11 + inpos] & 7) << 1)
        | ((in_array[12 + inpos] & 7) << 4)
        | ((in_array[13 + inpos] & 7) << 7)
        | ((in_array[14 + inpos] & 7) << 10)
        | ((in_array[15 + inpos] & 7) << 13)
        | ((in_array[16 + inpos] & 7) << 16)
        | ((in_array[17 + inpos] & 7) << 19)
        | ((in_array[18 + inpos] & 7) << 22)
        | ((in_array[19 + inpos] & 7) << 25)
        | ((in_array[20 + inpos] & 7) << 28)
        | ((in_array[21 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[21 + inpos] & 7) >> (3 - 2))
        | ((in_array[22 + inpos] & 7) << 2)
        | ((in_array[23 + inpos] & 7) << 5)
        | ((in_array[24 + inpos] & 7) << 8)
        | ((in_array[25 + inpos] & 7) << 11)
        | ((in_array[26 + inpos] & 7) << 14)
        | ((in_array[27 + inpos] & 7) << 17)
        | ((in_array[28 + inpos] & 7) << 20)
        | ((in_array[29 + inpos] & 7) << 23)
        | ((in_array[30 + inpos] & 7) << 26)
        | ((in_array[31 + inpos]) << 29);
}

pub fn fastpack30(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 1073741823)
        | ((in_array[1 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 1073741823) >> (30 - 28))
        | ((in_array[2 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 1073741823) >> (30 - 26))
        | ((in_array[3 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 1073741823) >> (30 - 24))
        | ((in_array[4 + inpos]) << 24);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 1073741823) >> (30 - 22))
        | ((in_array[5 + inpos]) << 22);
    out_array[5 + outpos] = ((in_array[5 + inpos] & 1073741823) >> (30 - 20))
        | ((in_array[6 + inpos]) << 20);
    out_array[6 + outpos] = ((in_array[6 + inpos] & 1073741823) >> (30 - 18))
        | ((in_array[7 + inpos]) << 18);
    out_array[7 + outpos] = ((in_array[7 + inpos] & 1073741823) >> (30 - 16))
        | ((in_array[8 + inpos]) << 16);
    out_array[8 + outpos] = ((in_array[8 + inpos] & 1073741823) >> (30 - 14))
        | ((in_array[9 + inpos]) << 14);
    out_array[9 + outpos] = ((in_array[9 + inpos] & 1073741823) >> (30 - 12))
        | ((in_array[10 + inpos]) << 12);
    out_array[10 + outpos] = ((in_array[10 + inpos] & 1073741823) >> (30 - 10))
        | ((in_array[11 + inpos]) << 10);
    out_array[11 + outpos] = ((in_array[11 + inpos] & 1073741823) >> (30 - 8))
        | ((in_array[12 + inpos]) << 8);
    out_array[12 + outpos] = ((in_array[12 + inpos] & 1073741823) >> (30 - 6))
        | ((in_array[13 + inpos]) << 6);
    out_array[13 + outpos] = ((in_array[13 + inpos] & 1073741823) >> (30 - 4))
        | ((in_array[14 + inpos]) << 4);
    out_array[14 + outpos] = ((in_array[14 + inpos] & 1073741823) >> (30 - 2))
        | ((in_array[15 + inpos]) << 2);
    out_array[15 + outpos] = (in_array[16 + inpos] & 1073741823)
        | ((in_array[17 + inpos]) << 30);
    out_array[16 + outpos] = ((in_array[17 + inpos] & 1073741823) >> (30 - 28))
        | ((in_array[18 + inpos]) << 28);
    out_array[17 + outpos] = ((in_array[18 + inpos] & 1073741823) >> (30 - 26))
        | ((in_array[19 + inpos]) << 26);
    out_array[18 + outpos] = ((in_array[19 + inpos] & 1073741823) >> (30 - 24))
        | ((in_array[20 + inpos]) << 24);
    out_array[19 + outpos] = ((in_array[20 + inpos] & 1073741823) >> (30 - 22))
        | ((in_array[21 + inpos]) << 22);
    out_array[20 + outpos] = ((in_array[21 + inpos] & 1073741823) >> (30 - 20))
        | ((in_array[22 + inpos]) << 20);
    out_array[21 + outpos] = ((in_array[22 + inpos] & 1073741823) >> (30 - 18))
        | ((in_array[23 + inpos]) << 18);
    out_array[22 + outpos] = ((in_array[23 + inpos] & 1073741823) >> (30 - 16))
        | ((in_array[24 + inpos]) << 16);
    out_array[23 + outpos] = ((in_array[24 + inpos] & 1073741823) >> (30 - 14))
        | ((in_array[25 + inpos]) << 14);
    out_array[24 + outpos] = ((in_array[25 + inpos] & 1073741823) >> (30 - 12))
        | ((in_array[26 + inpos]) << 12);
    out_array[25 + outpos] = ((in_array[26 + inpos] & 1073741823) >> (30 - 10))
        | ((in_array[27 + inpos]) << 10);
    out_array[26 + outpos] = ((in_array[27 + inpos] & 1073741823) >> (30 - 8))
        | ((in_array[28 + inpos]) << 8);
    out_array[27 + outpos] = ((in_array[28 + inpos] & 1073741823) >> (30 - 6))
        | ((in_array[29 + inpos]) << 6);
    out_array[28 + outpos] = ((in_array[29 + inpos] & 1073741823) >> (30 - 4))
        | ((in_array[30 + inpos]) << 4);
    out_array[29 + outpos] = ((in_array[30 + inpos] & 1073741823) >> (30 - 2))
        | ((in_array[31 + inpos]) << 2);
}

pub fn fastpack31(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 2147483647)
        | ((in_array[1 + inpos]) << 31);
    out_array[1 + outpos] = ((in_array[1 + inpos] & 2147483647) >> (31 - 30))
        | ((in_array[2 + inpos]) << 30);
    out_array[2 + outpos] = ((in_array[2 + inpos] & 2147483647) >> (31 - 29))
        | ((in_array[3 + inpos]) << 29);
    out_array[3 + outpos] = ((in_array[3 + inpos] & 2147483647) >> (31 - 28))
        | ((in_array[4 + inpos]) << 28);
    out_array[4 + outpos] = ((in_array[4 + inpos] & 2147483647) >> (31 - 27))
        | ((in_array[5 + inpos]) << 27);
    out_array[5 + outpos] = ((in_array[5 + inpos] & 2147483647) >> (31 - 26))
        | ((in_array[6 + inpos]) << 26);
    out_array[6 + outpos] = ((in_array[6 + inpos] & 2147483647) >> (31 - 25))
        | ((in_array[7 + inpos]) << 25);
    out_array[7 + outpos] = ((in_array[7 + inpos] & 2147483647) >> (31 - 24))
        | ((in_array[8 + inpos]) << 24);
    out_array[8 + outpos] = ((in_array[8 + inpos] & 2147483647) >> (31 - 23))
        | ((in_array[9 + inpos]) << 23);
    out_array[9 + outpos] = ((in_array[9 + inpos] & 2147483647) >> (31 - 22))
        | ((in_array[10 + inpos]) << 22);
    out_array[10 + outpos] = ((in_array[10 + inpos] & 2147483647) >> (31 - 21))
        | ((in_array[11 + inpos]) << 21);
    out_array[11 + outpos] = ((in_array[11 + inpos] & 2147483647) >> (31 - 20))
        | ((in_array[12 + inpos]) << 20);
    out_array[12 + outpos] = ((in_array[12 + inpos] & 2147483647) >> (31 - 19))
        | ((in_array[13 + inpos]) << 19);
    out_array[13 + outpos] = ((in_array[13 + inpos] & 2147483647) >> (31 - 18))
        | ((in_array[14 + inpos]) << 18);
    out_array[14 + outpos] = ((in_array[14 + inpos] & 2147483647) >> (31 - 17))
        | ((in_array[15 + inpos]) << 17);
    out_array[15 + outpos] = ((in_array[15 + inpos] & 2147483647) >> (31 - 16))
        | ((in_array[16 + inpos]) << 16);
    out_array[16 + outpos] = ((in_array[16 + inpos] & 2147483647) >> (31 - 15))
        | ((in_array[17 + inpos]) << 15);
    out_array[17 + outpos] = ((in_array[17 + inpos] & 2147483647) >> (31 - 14))
        | ((in_array[18 + inpos]) << 14);
    out_array[18 + outpos] = ((in_array[18 + inpos] & 2147483647) >> (31 - 13))
        | ((in_array[19 + inpos]) << 13);
    out_array[19 + outpos] = ((in_array[19 + inpos] & 2147483647) >> (31 - 12))
        | ((in_array[20 + inpos]) << 12);
    out_array[20 + outpos] = ((in_array[20 + inpos] & 2147483647) >> (31 - 11))
        | ((in_array[21 + inpos]) << 11);
    out_array[21 + outpos] = ((in_array[21 + inpos] & 2147483647) >> (31 - 10))
        | ((in_array[22 + inpos]) << 10);
    out_array[22 + outpos] = ((in_array[22 + inpos] & 2147483647) >> (31 - 9))
        | ((in_array[23 + inpos]) << 9);
    out_array[23 + outpos] = ((in_array[23 + inpos] & 2147483647) >> (31 - 8))
        | ((in_array[24 + inpos]) << 8);
    out_array[24 + outpos] = ((in_array[24 + inpos] & 2147483647) >> (31 - 7))
        | ((in_array[25 + inpos]) << 7);
    out_array[25 + outpos] = ((in_array[25 + inpos] & 2147483647) >> (31 - 6))
        | ((in_array[26 + inpos]) << 6);
    out_array[26 + outpos] = ((in_array[26 + inpos] & 2147483647) >> (31 - 5))
        | ((in_array[27 + inpos]) << 5);
    out_array[27 + outpos] = ((in_array[27 + inpos] & 2147483647) >> (31 - 4))
        | ((in_array[28 + inpos]) << 4);
    out_array[28 + outpos] = ((in_array[28 + inpos] & 2147483647) >> (31 - 3))
        | ((in_array[29 + inpos]) << 3);
    out_array[29 + outpos] = ((in_array[29 + inpos] & 2147483647) >> (31 - 2))
        | ((in_array[30 + inpos]) << 2);
    out_array[30 + outpos] = ((in_array[30 + inpos] & 2147483647) >> (31 - 1))
        | ((in_array[31 + inpos]) << 1);
}

pub fn fastpack32(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos..outpos + 32].copy_from_slice(&in_array[inpos..inpos + 32]);
}

pub fn fastpack4(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 15)
        | ((in_array[1 + inpos] & 15) << 4)
        | ((in_array[2 + inpos] & 15) << 8)
        | ((in_array[3 + inpos] & 15) << 12)
        | ((in_array[4 + inpos] & 15) << 16)
        | ((in_array[5 + inpos] & 15) << 20)
        | ((in_array[6 + inpos] & 15) << 24)
        | ((in_array[7 + inpos]) << 28);
    out_array[1 + outpos] = (in_array[8 + inpos] & 15)
        | ((in_array[9 + inpos] & 15) << 4)
        | ((in_array[10 + inpos] & 15) << 8)
        | ((in_array[11 + inpos] & 15) << 12)
        | ((in_array[12 + inpos] & 15) << 16)
        | ((in_array[13 + inpos] & 15) << 20)
        | ((in_array[14 + inpos] & 15) << 24)
        | ((in_array[15 + inpos]) << 28);
    out_array[2 + outpos] = (in_array[16 + inpos] & 15)
        | ((in_array[17 + inpos] & 15) << 4)
        | ((in_array[18 + inpos] & 15) << 8)
        | ((in_array[19 + inpos] & 15) << 12)
        | ((in_array[20 + inpos] & 15) << 16)
        | ((in_array[21 + inpos] & 15) << 20)
        | ((in_array[22 + inpos] & 15) << 24)
        | ((in_array[23 + inpos]) << 28);
    out_array[3 + outpos] = (in_array[24 + inpos] & 15)
        | ((in_array[25 + inpos] & 15) << 4)
        | ((in_array[26 + inpos] & 15) << 8)
        | ((in_array[27 + inpos] & 15) << 12)
        | ((in_array[28 + inpos] & 15) << 16)
        | ((in_array[29 + inpos] & 15) << 20)
        | ((in_array[30 + inpos] & 15) << 24)
        | ((in_array[31 + inpos]) << 28);
}

pub fn fastpack5(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 31)
        | ((in_array[1 + inpos] & 31) << 5)
        | ((in_array[2 + inpos] & 31) << 10)
        | ((in_array[3 + inpos] & 31) << 15)
        | ((in_array[4 + inpos] & 31) << 20)
        | ((in_array[5 + inpos] & 31) << 25)
        | ((in_array[6 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[6 + inpos] & 31) >> (5 - 3))
        | ((in_array[7 + inpos] & 31) << 3)
        | ((in_array[8 + inpos] & 31) << 8)
        | ((in_array[9 + inpos] & 31) << 13)
        | ((in_array[10 + inpos] & 31) << 18)
        | ((in_array[11 + inpos] & 31) << 23)
        | ((in_array[12 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[12 + inpos] & 31) >> (5 - 1))
        | ((in_array[13 + inpos] & 31) << 1)
        | ((in_array[14 + inpos] & 31) << 6)
        | ((in_array[15 + inpos] & 31) << 11)
        | ((in_array[16 + inpos] & 31) << 16)
        | ((in_array[17 + inpos] & 31) << 21)
        | ((in_array[18 + inpos] & 31) << 26)
        | ((in_array[19 + inpos]) << 31);
    out_array[3 + outpos] = ((in_array[19 + inpos] & 31) >> (5 - 4))
        | ((in_array[20 + inpos] & 31) << 4)
        | ((in_array[21 + inpos] & 31) << 9)
        | ((in_array[22 + inpos] & 31) << 14)
        | ((in_array[23 + inpos] & 31) << 19)
        | ((in_array[24 + inpos] & 31) << 24)
        | ((in_array[25 + inpos]) << 29);
    out_array[4 + outpos] = ((in_array[25 + inpos] & 31) >> (5 - 2))
        | ((in_array[26 + inpos] & 31) << 2)
        | ((in_array[27 + inpos] & 31) << 7)
        | ((in_array[28 + inpos] & 31) << 12)
        | ((in_array[29 + inpos] & 31) << 17)
        | ((in_array[30 + inpos] & 31) << 22)
        | ((in_array[31 + inpos]) << 27);
}

pub fn fastpack6(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 63)
        | ((in_array[1 + inpos] & 63) << 6)
        | ((in_array[2 + inpos] & 63) << 12)
        | ((in_array[3 + inpos] & 63) << 18)
        | ((in_array[4 + inpos] & 63) << 24)
        | ((in_array[5 + inpos]) << 30);
    out_array[1 + outpos] = ((in_array[5 + inpos] & 63) >> (6 - 4))
        | ((in_array[6 + inpos] & 63) << 4)
        | ((in_array[7 + inpos] & 63) << 10)
        | ((in_array[8 + inpos] & 63) << 16)
        | ((in_array[9 + inpos] & 63) << 22)
        | ((in_array[10 + inpos]) << 28);
    out_array[2 + outpos] = ((in_array[10 + inpos] & 63) >> (6 - 2))
        | ((in_array[11 + inpos] & 63) << 2)
        | ((in_array[12 + inpos] & 63) << 8)
        | ((in_array[13 + inpos] & 63) << 14)
        | ((in_array[14 + inpos] & 63) << 20)
        | ((in_array[15 + inpos]) << 26);
    out_array[3 + outpos] = (in_array[16 + inpos] & 63)
        | ((in_array[17 + inpos] & 63) << 6)
        | ((in_array[18 + inpos] & 63) << 12)
        | ((in_array[19 + inpos] & 63) << 18)
        | ((in_array[20 + inpos] & 63) << 24)
        | ((in_array[21 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[21 + inpos] & 63) >> (6 - 4))
        | ((in_array[22 + inpos] & 63) << 4)
        | ((in_array[23 + inpos] & 63) << 10)
        | ((in_array[24 + inpos] & 63) << 16)
        | ((in_array[25 + inpos] & 63) << 22)
        | ((in_array[26 + inpos]) << 28);
    out_array[5 + outpos] = ((in_array[26 + inpos] & 63) >> (6 - 2))
        | ((in_array[27 + inpos] & 63) << 2)
        | ((in_array[28 + inpos] & 63) << 8)
        | ((in_array[29 + inpos] & 63) << 14)
        | ((in_array[30 + inpos] & 63) << 20)
        | ((in_array[31 + inpos]) << 26);
}

pub fn fastpack7(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 127)
        | ((in_array[1 + inpos] & 127) << 7)
        | ((in_array[2 + inpos] & 127) << 14)
        | ((in_array[3 + inpos] & 127) << 21)
        | ((in_array[4 + inpos]) << 28);
    out_array[1 + outpos] = ((in_array[4 + inpos] & 127) >> (7 - 3))
        | ((in_array[5 + inpos] & 127) << 3)
        | ((in_array[6 + inpos] & 127) << 10)
        | ((in_array[7 + inpos] & 127) << 17)
        | ((in_array[8 + inpos] & 127) << 24)
        | ((in_array[9 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[9 + inpos] & 127) >> (7 - 6))
        | ((in_array[10 + inpos] & 127) << 6)
        | ((in_array[11 + inpos] & 127) << 13)
        | ((in_array[12 + inpos] & 127) << 20)
        | ((in_array[13 + inpos]) << 27);
    out_array[3 + outpos] = ((in_array[13 + inpos] & 127) >> (7 - 2))
        | ((in_array[14 + inpos] & 127) << 2)
        | ((in_array[15 + inpos] & 127) << 9)
        | ((in_array[16 + inpos] & 127) << 16)
        | ((in_array[17 + inpos] & 127) << 23)
        | ((in_array[18 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[18 + inpos] & 127) >> (7 - 5))
        | ((in_array[19 + inpos] & 127) << 5)
        | ((in_array[20 + inpos] & 127) << 12)
        | ((in_array[21 + inpos] & 127) << 19)
        | ((in_array[22 + inpos]) << 26);
    out_array[5 + outpos] = ((in_array[22 + inpos] & 127) >> (7 - 1))
        | ((in_array[23 + inpos] & 127) << 1)
        | ((in_array[24 + inpos] & 127) << 8)
        | ((in_array[25 + inpos] & 127) << 15)
        | ((in_array[26 + inpos] & 127) << 22)
        | ((in_array[27 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[27 + inpos] & 127) >> (7 - 4))
        | ((in_array[28 + inpos] & 127) << 4)
        | ((in_array[29 + inpos] & 127) << 11)
        | ((in_array[30 + inpos] & 127) << 18)
        | ((in_array[31 + inpos]) << 25);
}

pub fn fastpack8(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 255)
        | ((in_array[1 + inpos] & 255) << 8)
        | ((in_array[2 + inpos] & 255) << 16)
        | ((in_array[3 + inpos]) << 24);
    out_array[1 + outpos] = (in_array[4 + inpos] & 255)
        | ((in_array[5 + inpos] & 255) << 8)
        | ((in_array[6 + inpos] & 255) << 16)
        | ((in_array[7 + inpos]) << 24);
    out_array[2 + outpos] = (in_array[8 + inpos] & 255)
        | ((in_array[9 + inpos] & 255) << 8)
        | ((in_array[10 + inpos] & 255) << 16)
        | ((in_array[11 + inpos]) << 24);
    out_array[3 + outpos] = (in_array[12 + inpos] & 255)
        | ((in_array[13 + inpos] & 255) << 8)
        | ((in_array[14 + inpos] & 255) << 16)
        | ((in_array[15 + inpos]) << 24);
    out_array[4 + outpos] = (in_array[16 + inpos] & 255)
        | ((in_array[17 + inpos] & 255) << 8)
        | ((in_array[18 + inpos] & 255) << 16)
        | ((in_array[19 + inpos]) << 24);
    out_array[5 + outpos] = (in_array[20 + inpos] & 255)
        | ((in_array[21 + inpos] & 255) << 8)
        | ((in_array[22 + inpos] & 255) << 16)
        | ((in_array[23 + inpos]) << 24);
    out_array[6 + outpos] = (in_array[24 + inpos] & 255)
        | ((in_array[25 + inpos] & 255) << 8)
        | ((in_array[26 + inpos] & 255) << 16)
        | ((in_array[27 + inpos]) << 24);
    out_array[7 + outpos] = (in_array[28 + inpos] & 255)
        | ((in_array[29 + inpos] & 255) << 8)
        | ((in_array[30 + inpos] & 255) << 16)
        | ((in_array[31 + inpos]) << 24);
}

pub fn fastpack9(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
    out_array[outpos] = (in_array[inpos] & 511)
        | ((in_array[1 + inpos] & 511) << 9)
        | ((in_array[2 + inpos] & 511) << 18)
        | ((in_array[3 + inpos]) << 27);
    out_array[1 + outpos] = ((in_array[3 + inpos] & 511) >> (9 - 4))
        | ((in_array[4 + inpos] & 511) << 4)
        | ((in_array[5 + inpos] & 511) << 13)
        | ((in_array[6 + inpos] & 511) << 22)
        | ((in_array[7 + inpos]) << 31);
    out_array[2 + outpos] = ((in_array[7 + inpos] & 511) >> (9 - 8))
        | ((in_array[8 + inpos] & 511) << 8)
        | ((in_array[9 + inpos] & 511) << 17)
        | ((in_array[10 + inpos]) << 26);
    out_array[3 + outpos] = ((in_array[10 + inpos] & 511) >> (9 - 3))
        | ((in_array[11 + inpos] & 511) << 3)
        | ((in_array[12 + inpos] & 511) << 12)
        | ((in_array[13 + inpos] & 511) << 21)
        | ((in_array[14 + inpos]) << 30);
    out_array[4 + outpos] = ((in_array[14 + inpos] & 511) >> (9 - 7))
        | ((in_array[15 + inpos] & 511) << 7)
        | ((in_array[16 + inpos] & 511) << 16)
        | ((in_array[17 + inpos]) << 25);
    out_array[5 + outpos] = ((in_array[17 + inpos] & 511) >> (9 - 2))
        | ((in_array[18 + inpos] & 511) << 2)
        | ((in_array[19 + inpos] & 511) << 11)
        | ((in_array[20 + inpos] & 511) << 20)
        | ((in_array[21 + inpos]) << 29);
    out_array[6 + outpos] = ((in_array[21 + inpos] & 511) >> (9 - 6))
        | ((in_array[22 + inpos] & 511) << 6)
        | ((in_array[23 + inpos] & 511) << 15)
        | ((in_array[24 + inpos]) << 24);
    out_array[7 + outpos] = ((in_array[24 + inpos] & 511) >> (9 - 1))
        | ((in_array[25 + inpos] & 511) << 1)
        | ((in_array[26 + inpos] & 511) << 10)
        | ((in_array[27 + inpos] & 511) << 19)
        | ((in_array[28 + inpos]) << 28);
    out_array[8 + outpos] = ((in_array[28 + inpos] & 511) >> (9 - 5))
        | ((in_array[29 + inpos] & 511) << 5)
        | ((in_array[30 + inpos] & 511) << 14)
        | ((in_array[31 + inpos]) << 23);
}
