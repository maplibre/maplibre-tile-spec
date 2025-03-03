
pub struct BitpackingDecoder {}
impl BitpackingDecoder {
    /// Unpack the 32 integers
    pub fn fastunpack(
        in_array: &[u32],
        out_array: &mut [u32],
        bit: u8,
    ) {
        match bit {
            0 => Self::fastunpack0(in_array, 0, out_array, 0),
            1 => Self::fastunpack1(in_array, 0, out_array, 0),
            2 => Self::fastunpack2(in_array, 0, out_array, 0),
            3 => Self::fastunpack3(in_array, 0, out_array, 0),
            4 => Self::fastunpack4(in_array, 0, out_array, 0),
            5 => Self::fastunpack5(in_array, 0, out_array, 0),
            6 => Self::fastunpack6(in_array, 0, out_array, 0),
            7 => Self::fastunpack7(in_array, 0, out_array, 0),
            8 => Self::fastunpack8(in_array, 0, out_array, 0),
            9 => Self::fastunpack9(in_array, 0, out_array, 0),
            10 => Self::fastunpack10(in_array, 0, out_array, 0),
            11 => Self::fastunpack11(in_array, 0, out_array, 0),
            12 => Self::fastunpack12(in_array, 0, out_array, 0),
            13 => Self::fastunpack13(in_array, 0, out_array, 0),
            14 => Self::fastunpack14(in_array, 0, out_array, 0),
            15 => Self::fastunpack15(in_array, 0, out_array, 0),
            16 => Self::fastunpack16(in_array, 0, out_array, 0),
            17 => Self::fastunpack17(in_array, 0, out_array, 0),
            18 => Self::fastunpack18(in_array, 0, out_array, 0),
            19 => Self::fastunpack19(in_array, 0, out_array, 0),
            20 => Self::fastunpack20(in_array, 0, out_array, 0),
            21 => Self::fastunpack21(in_array, 0, out_array, 0),
            22 => Self::fastunpack22(in_array, 0, out_array, 0),
            23 => Self::fastunpack23(in_array, 0, out_array, 0),
            24 => Self::fastunpack24(in_array, 0, out_array, 0),
            25 => Self::fastunpack25(in_array, 0, out_array, 0),
            26 => Self::fastunpack26(in_array, 0, out_array, 0),
            27 => Self::fastunpack27(in_array, 0, out_array, 0),
            28 => Self::fastunpack28(in_array, 0, out_array, 0),
            29 => Self::fastunpack29(in_array, 0, out_array, 0),
            30 => Self::fastunpack30(in_array, 0, out_array, 0),
            31 => Self::fastunpack31(in_array, 0, out_array, 0),
            32 => Self::fastunpack32(in_array, 0, out_array, 0),
            _ => panic!("Unsupported bit width."),
        }
    }

    pub fn fastunpack0(_in_array: &[u32], _inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos..outpos + 32].fill(0);
    }

    pub fn fastunpack1(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 1;
        out_array[1 + outpos] = (in_array[inpos] >> 1) & 1;
        out_array[2 + outpos] = (in_array[inpos] >> 2) & 1;
        out_array[3 + outpos] = (in_array[inpos] >> 3) & 1;
        out_array[4 + outpos] = (in_array[inpos] >> 4) & 1;
        out_array[5 + outpos] = (in_array[inpos] >> 5) & 1;
        out_array[6 + outpos] = (in_array[inpos] >> 6) & 1;
        out_array[7 + outpos] = (in_array[inpos] >> 7) & 1;
        out_array[8 + outpos] = (in_array[inpos] >> 8) & 1;
        out_array[9 + outpos] = (in_array[inpos] >> 9) & 1;
        out_array[10 + outpos] = (in_array[inpos] >> 10) & 1;
        out_array[11 + outpos] = (in_array[inpos] >> 11) & 1;
        out_array[12 + outpos] = (in_array[inpos] >> 12) & 1;
        out_array[13 + outpos] = (in_array[inpos] >> 13) & 1;
        out_array[14 + outpos] = (in_array[inpos] >> 14) & 1;
        out_array[15 + outpos] = (in_array[inpos] >> 15) & 1;
        out_array[16 + outpos] = (in_array[inpos] >> 16) & 1;
        out_array[17 + outpos] = (in_array[inpos] >> 17) & 1;
        out_array[18 + outpos] = (in_array[inpos] >> 18) & 1;
        out_array[19 + outpos] = (in_array[inpos] >> 19) & 1;
        out_array[20 + outpos] = (in_array[inpos] >> 20) & 1;
        out_array[21 + outpos] = (in_array[inpos] >> 21) & 1;
        out_array[22 + outpos] = (in_array[inpos] >> 22) & 1;
        out_array[23 + outpos] = (in_array[inpos] >> 23) & 1;
        out_array[24 + outpos] = (in_array[inpos] >> 24) & 1;
        out_array[25 + outpos] = (in_array[inpos] >> 25) & 1;
        out_array[26 + outpos] = (in_array[inpos] >> 26) & 1;
        out_array[27 + outpos] = (in_array[inpos] >> 27) & 1;
        out_array[28 + outpos] = (in_array[inpos] >> 28) & 1;
        out_array[29 + outpos] = (in_array[inpos] >> 29) & 1;
        out_array[30 + outpos] = (in_array[inpos] >> 30) & 1;
        out_array[31 + outpos] = in_array[inpos] >> 31;
    }

    pub fn fastunpack10(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 1023;
        out_array[1 + outpos] = (in_array[inpos] >> 10) & 1023;
        out_array[2 + outpos] = (in_array[inpos] >> 20) & 1023;
        out_array[3 + outpos] = (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 255) << (10 - 8));
        out_array[4 + outpos] = (in_array[1 + inpos] >> 8) & 1023;
        out_array[5 + outpos] = (in_array[1 + inpos] >> 18) & 1023;
        out_array[6 + outpos] = (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 63) << (10 - 6));
        out_array[7 + outpos] = (in_array[2 + inpos] >> 6) & 1023;
        out_array[8 + outpos] = (in_array[2 + inpos] >> 16) & 1023;
        out_array[9 + outpos] = (in_array[2 + inpos] >> 26) | ((in_array[3 + inpos] & 15) << (10 - 4));
        out_array[10 + outpos] = (in_array[3 + inpos] >> 4) & 1023;
        out_array[11 + outpos] = (in_array[3 + inpos] >> 14) & 1023;
        out_array[12 + outpos] = (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 3) << (10 - 2));
        out_array[13 + outpos] = (in_array[4 + inpos] >> 2) & 1023;
        out_array[14 + outpos] = (in_array[4 + inpos] >> 12) & 1023;
        out_array[15 + outpos] = in_array[4 + inpos] >> 22;
        out_array[16 + outpos] = (in_array[5 + inpos] >> 0) & 1023;
        out_array[17 + outpos] = (in_array[5 + inpos] >> 10) & 1023;
        out_array[18 + outpos] = (in_array[5 + inpos] >> 20) & 1023;
        out_array[19 + outpos] =
            (in_array[5 + inpos] >> 30) | ((in_array[6 + inpos] & 255) << (10 - 8));
        out_array[20 + outpos] = (in_array[6 + inpos] >> 8) & 1023;
        out_array[21 + outpos] = (in_array[6 + inpos] >> 18) & 1023;
        out_array[22 + outpos] = (in_array[6 + inpos] >> 28) | ((in_array[7 + inpos] & 63) << (10 - 6));
        out_array[23 + outpos] = (in_array[7 + inpos] >> 6) & 1023;
        out_array[24 + outpos] = (in_array[7 + inpos] >> 16) & 1023;
        out_array[25 + outpos] = (in_array[7 + inpos] >> 26) | ((in_array[8 + inpos] & 15) << (10 - 4));
        out_array[26 + outpos] = (in_array[8 + inpos] >> 4) & 1023;
        out_array[27 + outpos] = (in_array[8 + inpos] >> 14) & 1023;
        out_array[28 + outpos] = (in_array[8 + inpos] >> 24) | ((in_array[9 + inpos] & 3) << (10 - 2));
        out_array[29 + outpos] = (in_array[9 + inpos] >> 2) & 1023;
        out_array[30 + outpos] = (in_array[9 + inpos] >> 12) & 1023;
        out_array[31 + outpos] = in_array[9 + inpos] >> 22;
    }

    pub fn fastunpack11(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 2047;
        out_array[1 + outpos] = (in_array[inpos] >> 11) & 2047;
        out_array[2 + outpos] = (in_array[inpos] >> 22) | ((in_array[1 + inpos] & 1) << (11 - 1));
        out_array[3 + outpos] = (in_array[1 + inpos] >> 1) & 2047;
        out_array[4 + outpos] = (in_array[1 + inpos] >> 12) & 2047;
        out_array[5 + outpos] = (in_array[1 + inpos] >> 23) | ((in_array[2 + inpos] & 3) << (11 - 2));
        out_array[6 + outpos] = (in_array[2 + inpos] >> 2) & 2047;
        out_array[7 + outpos] = (in_array[2 + inpos] >> 13) & 2047;
        out_array[8 + outpos] = (in_array[2 + inpos] >> 24) | ((in_array[3 + inpos] & 7) << (11 - 3));
        out_array[9 + outpos] = (in_array[3 + inpos] >> 3) & 2047;
        out_array[10 + outpos] = (in_array[3 + inpos] >> 14) & 2047;
        out_array[11 + outpos] = (in_array[3 + inpos] >> 25) | ((in_array[4 + inpos] & 15) << (11 - 4));
        out_array[12 + outpos] = (in_array[4 + inpos] >> 4) & 2047;
        out_array[13 + outpos] = (in_array[4 + inpos] >> 15) & 2047;
        out_array[14 + outpos] = (in_array[4 + inpos] >> 26) | ((in_array[5 + inpos] & 31) << (11 - 5));
        out_array[15 + outpos] = (in_array[5 + inpos] >> 5) & 2047;
        out_array[16 + outpos] = (in_array[5 + inpos] >> 16) & 2047;
        out_array[17 + outpos] = (in_array[5 + inpos] >> 27) | ((in_array[6 + inpos] & 63) << (11 - 6));
        out_array[18 + outpos] = (in_array[6 + inpos] >> 6) & 2047;
        out_array[19 + outpos] = (in_array[6 + inpos] >> 17) & 2047;
        out_array[20 + outpos] =
            (in_array[6 + inpos] >> 28) | ((in_array[7 + inpos] & 127) << (11 - 7));
        out_array[21 + outpos] = (in_array[7 + inpos] >> 7) & 2047;
        out_array[22 + outpos] = (in_array[7 + inpos] >> 18) & 2047;
        out_array[23 + outpos] =
            (in_array[7 + inpos] >> 29) | ((in_array[8 + inpos] & 255) << (11 - 8));
        out_array[24 + outpos] = (in_array[8 + inpos] >> 8) & 2047;
        out_array[25 + outpos] = (in_array[8 + inpos] >> 19) & 2047;
        out_array[26 + outpos] =
            (in_array[8 + inpos] >> 30) | ((in_array[9 + inpos] & 511) << (11 - 9));
        out_array[27 + outpos] = (in_array[9 + inpos] >> 9) & 2047;
        out_array[28 + outpos] = (in_array[9 + inpos] >> 20) & 2047;
        out_array[29 + outpos] =
            (in_array[9 + inpos] >> 31) | ((in_array[10 + inpos] & 1023) << (11 - 10));
        out_array[30 + outpos] = (in_array[10 + inpos] >> 10) & 2047;
        out_array[31 + outpos] = in_array[10 + inpos] >> 21;
    }

    pub fn fastunpack12(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 4095;
        out_array[1 + outpos] = (in_array[inpos] >> 12) & 4095;
        out_array[2 + outpos] = (in_array[inpos] >> 24) | ((in_array[1 + inpos] & 15) << (12 - 4));
        out_array[3 + outpos] = (in_array[1 + inpos] >> 4) & 4095;
        out_array[4 + outpos] = (in_array[1 + inpos] >> 16) & 4095;
        out_array[5 + outpos] = (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 255) << (12 - 8));
        out_array[6 + outpos] = (in_array[2 + inpos] >> 8) & 4095;
        out_array[7 + outpos] = in_array[2 + inpos] >> 20;
        out_array[8 + outpos] = (in_array[3 + inpos] >> 0) & 4095;
        out_array[9 + outpos] = (in_array[3 + inpos] >> 12) & 4095;
        out_array[10 + outpos] = (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 15) << (12 - 4));
        out_array[11 + outpos] = (in_array[4 + inpos] >> 4) & 4095;
        out_array[12 + outpos] = (in_array[4 + inpos] >> 16) & 4095;
        out_array[13 + outpos] =
            (in_array[4 + inpos] >> 28) | ((in_array[5 + inpos] & 255) << (12 - 8));
        out_array[14 + outpos] = (in_array[5 + inpos] >> 8) & 4095;
        out_array[15 + outpos] = in_array[5 + inpos] >> 20;
        out_array[16 + outpos] = (in_array[6 + inpos] >> 0) & 4095;
        out_array[17 + outpos] = (in_array[6 + inpos] >> 12) & 4095;
        out_array[18 + outpos] = (in_array[6 + inpos] >> 24) | ((in_array[7 + inpos] & 15) << (12 - 4));
        out_array[19 + outpos] = (in_array[7 + inpos] >> 4) & 4095;
        out_array[20 + outpos] = (in_array[7 + inpos] >> 16) & 4095;
        out_array[21 + outpos] =
            (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 255) << (12 - 8));
        out_array[22 + outpos] = (in_array[8 + inpos] >> 8) & 4095;
        out_array[23 + outpos] = in_array[8 + inpos] >> 20;
        out_array[24 + outpos] = (in_array[9 + inpos] >> 0) & 4095;
        out_array[25 + outpos] = (in_array[9 + inpos] >> 12) & 4095;
        out_array[26 + outpos] =
            (in_array[9 + inpos] >> 24) | ((in_array[10 + inpos] & 15) << (12 - 4));
        out_array[27 + outpos] = (in_array[10 + inpos] >> 4) & 4095;
        out_array[28 + outpos] = (in_array[10 + inpos] >> 16) & 4095;
        out_array[29 + outpos] =
            (in_array[10 + inpos] >> 28) | ((in_array[11 + inpos] & 255) << (12 - 8));
        out_array[30 + outpos] = (in_array[11 + inpos] >> 8) & 4095;
        out_array[31 + outpos] = in_array[11 + inpos] >> 20;
    }

    pub fn fastunpack13(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 8191;
        out_array[1 + outpos] = (in_array[inpos] >> 13) & 8191;
        out_array[2 + outpos] = (in_array[inpos] >> 26) | ((in_array[1 + inpos] & 127) << (13 - 7));
        out_array[3 + outpos] = (in_array[1 + inpos] >> 7) & 8191;
        out_array[4 + outpos] = (in_array[1 + inpos] >> 20) | ((in_array[2 + inpos] & 1) << (13 - 1));
        out_array[5 + outpos] = (in_array[2 + inpos] >> 1) & 8191;
        out_array[6 + outpos] = (in_array[2 + inpos] >> 14) & 8191;
        out_array[7 + outpos] = (in_array[2 + inpos] >> 27) | ((in_array[3 + inpos] & 255) << (13 - 8));
        out_array[8 + outpos] = (in_array[3 + inpos] >> 8) & 8191;
        out_array[9 + outpos] = (in_array[3 + inpos] >> 21) | ((in_array[4 + inpos] & 3) << (13 - 2));
        out_array[10 + outpos] = (in_array[4 + inpos] >> 2) & 8191;
        out_array[11 + outpos] = (in_array[4 + inpos] >> 15) & 8191;
        out_array[12 + outpos] =
            (in_array[4 + inpos] >> 28) | ((in_array[5 + inpos] & 511) << (13 - 9));
        out_array[13 + outpos] = (in_array[5 + inpos] >> 9) & 8191;
        out_array[14 + outpos] = (in_array[5 + inpos] >> 22) | ((in_array[6 + inpos] & 7) << (13 - 3));
        out_array[15 + outpos] = (in_array[6 + inpos] >> 3) & 8191;
        out_array[16 + outpos] = (in_array[6 + inpos] >> 16) & 8191;
        out_array[17 + outpos] =
            (in_array[6 + inpos] >> 29) | ((in_array[7 + inpos] & 1023) << (13 - 10));
        out_array[18 + outpos] = (in_array[7 + inpos] >> 10) & 8191;
        out_array[19 + outpos] = (in_array[7 + inpos] >> 23) | ((in_array[8 + inpos] & 15) << (13 - 4));
        out_array[20 + outpos] = (in_array[8 + inpos] >> 4) & 8191;
        out_array[21 + outpos] = (in_array[8 + inpos] >> 17) & 8191;
        out_array[22 + outpos] =
            (in_array[8 + inpos] >> 30) | ((in_array[9 + inpos] & 2047) << (13 - 11));
        out_array[23 + outpos] = (in_array[9 + inpos] >> 11) & 8191;
        out_array[24 + outpos] =
            (in_array[9 + inpos] >> 24) | ((in_array[10 + inpos] & 31) << (13 - 5));
        out_array[25 + outpos] = (in_array[10 + inpos] >> 5) & 8191;
        out_array[26 + outpos] = (in_array[10 + inpos] >> 18) & 8191;
        out_array[27 + outpos] =
            (in_array[10 + inpos] >> 31) | ((in_array[11 + inpos] & 4095) << (13 - 12));
        out_array[28 + outpos] = (in_array[11 + inpos] >> 12) & 8191;
        out_array[29 + outpos] =
            (in_array[11 + inpos] >> 25) | ((in_array[12 + inpos] & 63) << (13 - 6));
        out_array[30 + outpos] = (in_array[12 + inpos] >> 6) & 8191;
        out_array[31 + outpos] = in_array[12 + inpos] >> 19;
    }

    pub fn fastunpack14(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 16383;
        out_array[1 + outpos] = (in_array[inpos] >> 14) & 16383;
        out_array[2 + outpos] = (in_array[inpos] >> 28) | ((in_array[1 + inpos] & 1023) << (14 - 10));
        out_array[3 + outpos] = (in_array[1 + inpos] >> 10) & 16383;
        out_array[4 + outpos] = (in_array[1 + inpos] >> 24) | ((in_array[2 + inpos] & 63) << (14 - 6));
        out_array[5 + outpos] = (in_array[2 + inpos] >> 6) & 16383;
        out_array[6 + outpos] = (in_array[2 + inpos] >> 20) | ((in_array[3 + inpos] & 3) << (14 - 2));
        out_array[7 + outpos] = (in_array[3 + inpos] >> 2) & 16383;
        out_array[8 + outpos] = (in_array[3 + inpos] >> 16) & 16383;
        out_array[9 + outpos] =
            (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 4095) << (14 - 12));
        out_array[10 + outpos] = (in_array[4 + inpos] >> 12) & 16383;
        out_array[11 + outpos] =
            (in_array[4 + inpos] >> 26) | ((in_array[5 + inpos] & 255) << (14 - 8));
        out_array[12 + outpos] = (in_array[5 + inpos] >> 8) & 16383;
        out_array[13 + outpos] = (in_array[5 + inpos] >> 22) | ((in_array[6 + inpos] & 15) << (14 - 4));
        out_array[14 + outpos] = (in_array[6 + inpos] >> 4) & 16383;
        out_array[15 + outpos] = in_array[6 + inpos] >> 18;
        out_array[16 + outpos] = (in_array[7 + inpos] >> 0) & 16383;
        out_array[17 + outpos] = (in_array[7 + inpos] >> 14) & 16383;
        out_array[18 + outpos] =
            (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 1023) << (14 - 10));
        out_array[19 + outpos] = (in_array[8 + inpos] >> 10) & 16383;
        out_array[20 + outpos] = (in_array[8 + inpos] >> 24) | ((in_array[9 + inpos] & 63) << (14 - 6));
        out_array[21 + outpos] = (in_array[9 + inpos] >> 6) & 16383;
        out_array[22 + outpos] = (in_array[9 + inpos] >> 20) | ((in_array[10 + inpos] & 3) << (14 - 2));
        out_array[23 + outpos] = (in_array[10 + inpos] >> 2) & 16383;
        out_array[24 + outpos] = (in_array[10 + inpos] >> 16) & 16383;
        out_array[25 + outpos] =
            (in_array[10 + inpos] >> 30) | ((in_array[11 + inpos] & 4095) << (14 - 12));
        out_array[26 + outpos] = (in_array[11 + inpos] >> 12) & 16383;
        out_array[27 + outpos] =
            (in_array[11 + inpos] >> 26) | ((in_array[12 + inpos] & 255) << (14 - 8));
        out_array[28 + outpos] = (in_array[12 + inpos] >> 8) & 16383;
        out_array[29 + outpos] =
            (in_array[12 + inpos] >> 22) | ((in_array[13 + inpos] & 15) << (14 - 4));
        out_array[30 + outpos] = (in_array[13 + inpos] >> 4) & 16383;
        out_array[31 + outpos] = in_array[13 + inpos] >> 18;
    }

    pub fn fastunpack15(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 32767;
        out_array[1 + outpos] = (in_array[inpos] >> 15) & 32767;
        out_array[2 + outpos] = (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 8191) << (15 - 13));
        out_array[3 + outpos] = (in_array[1 + inpos] >> 13) & 32767;
        out_array[4 + outpos] =
            (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 2047) << (15 - 11));
        out_array[5 + outpos] = (in_array[2 + inpos] >> 11) & 32767;
        out_array[6 + outpos] = (in_array[2 + inpos] >> 26) | ((in_array[3 + inpos] & 511) << (15 - 9));
        out_array[7 + outpos] = (in_array[3 + inpos] >> 9) & 32767;
        out_array[8 + outpos] = (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 127) << (15 - 7));
        out_array[9 + outpos] = (in_array[4 + inpos] >> 7) & 32767;
        out_array[10 + outpos] = (in_array[4 + inpos] >> 22) | ((in_array[5 + inpos] & 31) << (15 - 5));
        out_array[11 + outpos] = (in_array[5 + inpos] >> 5) & 32767;
        out_array[12 + outpos] = (in_array[5 + inpos] >> 20) | ((in_array[6 + inpos] & 7) << (15 - 3));
        out_array[13 + outpos] = (in_array[6 + inpos] >> 3) & 32767;
        out_array[14 + outpos] = (in_array[6 + inpos] >> 18) | ((in_array[7 + inpos] & 1) << (15 - 1));
        out_array[15 + outpos] = (in_array[7 + inpos] >> 1) & 32767;
        out_array[16 + outpos] = (in_array[7 + inpos] >> 16) & 32767;
        out_array[17 + outpos] =
            (in_array[7 + inpos] >> 31) | ((in_array[8 + inpos] & 16383) << (15 - 14));
        out_array[18 + outpos] = (in_array[8 + inpos] >> 14) & 32767;
        out_array[19 + outpos] =
            (in_array[8 + inpos] >> 29) | ((in_array[9 + inpos] & 4095) << (15 - 12));
        out_array[20 + outpos] = (in_array[9 + inpos] >> 12) & 32767;
        out_array[21 + outpos] =
            (in_array[9 + inpos] >> 27) | ((in_array[10 + inpos] & 1023) << (15 - 10));
        out_array[22 + outpos] = (in_array[10 + inpos] >> 10) & 32767;
        out_array[23 + outpos] =
            (in_array[10 + inpos] >> 25) | ((in_array[11 + inpos] & 255) << (15 - 8));
        out_array[24 + outpos] = (in_array[11 + inpos] >> 8) & 32767;
        out_array[25 + outpos] =
            (in_array[11 + inpos] >> 23) | ((in_array[12 + inpos] & 63) << (15 - 6));
        out_array[26 + outpos] = (in_array[12 + inpos] >> 6) & 32767;
        out_array[27 + outpos] =
            (in_array[12 + inpos] >> 21) | ((in_array[13 + inpos] & 15) << (15 - 4));
        out_array[28 + outpos] = (in_array[13 + inpos] >> 4) & 32767;
        out_array[29 + outpos] =
            (in_array[13 + inpos] >> 19) | ((in_array[14 + inpos] & 3) << (15 - 2));
        out_array[30 + outpos] = (in_array[14 + inpos] >> 2) & 32767;
        out_array[31 + outpos] = in_array[14 + inpos] >> 17;
    }

    pub fn fastunpack16(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 65535;
        out_array[1 + outpos] = in_array[inpos] >> 16;
        out_array[2 + outpos] = (in_array[1 + inpos] >> 0) & 65535;
        out_array[3 + outpos] = in_array[1 + inpos] >> 16;
        out_array[4 + outpos] = (in_array[2 + inpos] >> 0) & 65535;
        out_array[5 + outpos] = in_array[2 + inpos] >> 16;
        out_array[6 + outpos] = (in_array[3 + inpos] >> 0) & 65535;
        out_array[7 + outpos] = in_array[3 + inpos] >> 16;
        out_array[8 + outpos] = (in_array[4 + inpos] >> 0) & 65535;
        out_array[9 + outpos] = in_array[4 + inpos] >> 16;
        out_array[10 + outpos] = (in_array[5 + inpos] >> 0) & 65535;
        out_array[11 + outpos] = in_array[5 + inpos] >> 16;
        out_array[12 + outpos] = (in_array[6 + inpos] >> 0) & 65535;
        out_array[13 + outpos] = in_array[6 + inpos] >> 16;
        out_array[14 + outpos] = (in_array[7 + inpos] >> 0) & 65535;
        out_array[15 + outpos] = in_array[7 + inpos] >> 16;
        out_array[16 + outpos] = (in_array[8 + inpos] >> 0) & 65535;
        out_array[17 + outpos] = in_array[8 + inpos] >> 16;
        out_array[18 + outpos] = (in_array[9 + inpos] >> 0) & 65535;
        out_array[19 + outpos] = in_array[9 + inpos] >> 16;
        out_array[20 + outpos] = (in_array[10 + inpos] >> 0) & 65535;
        out_array[21 + outpos] = in_array[10 + inpos] >> 16;
        out_array[22 + outpos] = (in_array[11 + inpos] >> 0) & 65535;
        out_array[23 + outpos] = in_array[11 + inpos] >> 16;
        out_array[24 + outpos] = (in_array[12 + inpos] >> 0) & 65535;
        out_array[25 + outpos] = in_array[12 + inpos] >> 16;
        out_array[26 + outpos] = (in_array[13 + inpos] >> 0) & 65535;
        out_array[27 + outpos] = in_array[13 + inpos] >> 16;
        out_array[28 + outpos] = (in_array[14 + inpos] >> 0) & 65535;
        out_array[29 + outpos] = in_array[14 + inpos] >> 16;
        out_array[30 + outpos] = (in_array[15 + inpos] >> 0) & 65535;
        out_array[31 + outpos] = in_array[15 + inpos] >> 16;
    }

    pub fn fastunpack17(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 131071;
        out_array[1 + outpos] = (in_array[inpos] >> 17) | ((in_array[1 + inpos] & 3) << (17 - 2));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 2) & 131071;
        out_array[3 + outpos] = (in_array[1 + inpos] >> 19) | ((in_array[2 + inpos] & 15) << (17 - 4));
        out_array[4 + outpos] = (in_array[2 + inpos] >> 4) & 131071;
        out_array[5 + outpos] = (in_array[2 + inpos] >> 21) | ((in_array[3 + inpos] & 63) << (17 - 6));
        out_array[6 + outpos] = (in_array[3 + inpos] >> 6) & 131071;
        out_array[7 + outpos] = (in_array[3 + inpos] >> 23) | ((in_array[4 + inpos] & 255) << (17 - 8));
        out_array[8 + outpos] = (in_array[4 + inpos] >> 8) & 131071;
        out_array[9 + outpos] =
            (in_array[4 + inpos] >> 25) | ((in_array[5 + inpos] & 1023) << (17 - 10));
        out_array[10 + outpos] = (in_array[5 + inpos] >> 10) & 131071;
        out_array[11 + outpos] =
            (in_array[5 + inpos] >> 27) | ((in_array[6 + inpos] & 4095) << (17 - 12));
        out_array[12 + outpos] = (in_array[6 + inpos] >> 12) & 131071;
        out_array[13 + outpos] =
            (in_array[6 + inpos] >> 29) | ((in_array[7 + inpos] & 16383) << (17 - 14));
        out_array[14 + outpos] = (in_array[7 + inpos] >> 14) & 131071;
        out_array[15 + outpos] =
            (in_array[7 + inpos] >> 31) | ((in_array[8 + inpos] & 65535) << (17 - 16));
        out_array[16 + outpos] = (in_array[8 + inpos] >> 16) | ((in_array[9 + inpos] & 1) << (17 - 1));
        out_array[17 + outpos] = (in_array[9 + inpos] >> 1) & 131071;
        out_array[18 + outpos] = (in_array[9 + inpos] >> 18) | ((in_array[10 + inpos] & 7) << (17 - 3));
        out_array[19 + outpos] = (in_array[10 + inpos] >> 3) & 131071;
        out_array[20 + outpos] =
            (in_array[10 + inpos] >> 20) | ((in_array[11 + inpos] & 31) << (17 - 5));
        out_array[21 + outpos] = (in_array[11 + inpos] >> 5) & 131071;
        out_array[22 + outpos] =
            (in_array[11 + inpos] >> 22) | ((in_array[12 + inpos] & 127) << (17 - 7));
        out_array[23 + outpos] = (in_array[12 + inpos] >> 7) & 131071;
        out_array[24 + outpos] =
            (in_array[12 + inpos] >> 24) | ((in_array[13 + inpos] & 511) << (17 - 9));
        out_array[25 + outpos] = (in_array[13 + inpos] >> 9) & 131071;
        out_array[26 + outpos] =
            (in_array[13 + inpos] >> 26) | ((in_array[14 + inpos] & 2047) << (17 - 11));
        out_array[27 + outpos] = (in_array[14 + inpos] >> 11) & 131071;
        out_array[28 + outpos] =
            (in_array[14 + inpos] >> 28) | ((in_array[15 + inpos] & 8191) << (17 - 13));
        out_array[29 + outpos] = (in_array[15 + inpos] >> 13) & 131071;
        out_array[30 + outpos] =
            (in_array[15 + inpos] >> 30) | ((in_array[16 + inpos] & 32767) << (17 - 15));
        out_array[31 + outpos] = in_array[16 + inpos] >> 15;
    }

    pub fn fastunpack18(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 262143;
        out_array[1 + outpos] = (in_array[inpos] >> 18) | ((in_array[1 + inpos] & 15) << (18 - 4));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 4) & 262143;
        out_array[3 + outpos] = (in_array[1 + inpos] >> 22) | ((in_array[2 + inpos] & 255) << (18 - 8));
        out_array[4 + outpos] = (in_array[2 + inpos] >> 8) & 262143;
        out_array[5 + outpos] =
            (in_array[2 + inpos] >> 26) | ((in_array[3 + inpos] & 4095) << (18 - 12));
        out_array[6 + outpos] = (in_array[3 + inpos] >> 12) & 262143;
        out_array[7 + outpos] =
            (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 65535) << (18 - 16));
        out_array[8 + outpos] = (in_array[4 + inpos] >> 16) | ((in_array[5 + inpos] & 3) << (18 - 2));
        out_array[9 + outpos] = (in_array[5 + inpos] >> 2) & 262143;
        out_array[10 + outpos] = (in_array[5 + inpos] >> 20) | ((in_array[6 + inpos] & 63) << (18 - 6));
        out_array[11 + outpos] = (in_array[6 + inpos] >> 6) & 262143;
        out_array[12 + outpos] =
            (in_array[6 + inpos] >> 24) | ((in_array[7 + inpos] & 1023) << (18 - 10));
        out_array[13 + outpos] = (in_array[7 + inpos] >> 10) & 262143;
        out_array[14 + outpos] =
            (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 16383) << (18 - 14));
        out_array[15 + outpos] = in_array[8 + inpos] >> 14;
        out_array[16 + outpos] = (in_array[9 + inpos] >> 0) & 262143;
        out_array[17 + outpos] =
            (in_array[9 + inpos] >> 18) | ((in_array[10 + inpos] & 15) << (18 - 4));
        out_array[18 + outpos] = (in_array[10 + inpos] >> 4) & 262143;
        out_array[19 + outpos] =
            (in_array[10 + inpos] >> 22) | ((in_array[11 + inpos] & 255) << (18 - 8));
        out_array[20 + outpos] = (in_array[11 + inpos] >> 8) & 262143;
        out_array[21 + outpos] =
            (in_array[11 + inpos] >> 26) | ((in_array[12 + inpos] & 4095) << (18 - 12));
        out_array[22 + outpos] = (in_array[12 + inpos] >> 12) & 262143;
        out_array[23 + outpos] =
            (in_array[12 + inpos] >> 30) | ((in_array[13 + inpos] & 65535) << (18 - 16));
        out_array[24 + outpos] =
            (in_array[13 + inpos] >> 16) | ((in_array[14 + inpos] & 3) << (18 - 2));
        out_array[25 + outpos] = (in_array[14 + inpos] >> 2) & 262143;
        out_array[26 + outpos] =
            (in_array[14 + inpos] >> 20) | ((in_array[15 + inpos] & 63) << (18 - 6));
        out_array[27 + outpos] = (in_array[15 + inpos] >> 6) & 262143;
        out_array[28 + outpos] =
            (in_array[15 + inpos] >> 24) | ((in_array[16 + inpos] & 1023) << (18 - 10));
        out_array[29 + outpos] = (in_array[16 + inpos] >> 10) & 262143;
        out_array[30 + outpos] =
            (in_array[16 + inpos] >> 28) | ((in_array[17 + inpos] & 16383) << (18 - 14));
        out_array[31 + outpos] = in_array[17 + inpos] >> 14;
    }

    pub fn fastunpack19(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 524287;
        out_array[1 + outpos] = (in_array[inpos] >> 19) | ((in_array[1 + inpos] & 63) << (19 - 6));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 6) & 524287;
        out_array[3 + outpos] =
            (in_array[1 + inpos] >> 25) | ((in_array[2 + inpos] & 4095) << (19 - 12));
        out_array[4 + outpos] = (in_array[2 + inpos] >> 12) & 524287;
        out_array[5 + outpos] =
            (in_array[2 + inpos] >> 31) | ((in_array[3 + inpos] & 262143) << (19 - 18));
        out_array[6 + outpos] = (in_array[3 + inpos] >> 18) | ((in_array[4 + inpos] & 31) << (19 - 5));
        out_array[7 + outpos] = (in_array[4 + inpos] >> 5) & 524287;
        out_array[8 + outpos] =
            (in_array[4 + inpos] >> 24) | ((in_array[5 + inpos] & 2047) << (19 - 11));
        out_array[9 + outpos] = (in_array[5 + inpos] >> 11) & 524287;
        out_array[10 + outpos] =
            (in_array[5 + inpos] >> 30) | ((in_array[6 + inpos] & 131071) << (19 - 17));
        out_array[11 + outpos] = (in_array[6 + inpos] >> 17) | ((in_array[7 + inpos] & 15) << (19 - 4));
        out_array[12 + outpos] = (in_array[7 + inpos] >> 4) & 524287;
        out_array[13 + outpos] =
            (in_array[7 + inpos] >> 23) | ((in_array[8 + inpos] & 1023) << (19 - 10));
        out_array[14 + outpos] = (in_array[8 + inpos] >> 10) & 524287;
        out_array[15 + outpos] =
            (in_array[8 + inpos] >> 29) | ((in_array[9 + inpos] & 65535) << (19 - 16));
        out_array[16 + outpos] = (in_array[9 + inpos] >> 16) | ((in_array[10 + inpos] & 7) << (19 - 3));
        out_array[17 + outpos] = (in_array[10 + inpos] >> 3) & 524287;
        out_array[18 + outpos] =
            (in_array[10 + inpos] >> 22) | ((in_array[11 + inpos] & 511) << (19 - 9));
        out_array[19 + outpos] = (in_array[11 + inpos] >> 9) & 524287;
        out_array[20 + outpos] =
            (in_array[11 + inpos] >> 28) | ((in_array[12 + inpos] & 32767) << (19 - 15));
        out_array[21 + outpos] =
            (in_array[12 + inpos] >> 15) | ((in_array[13 + inpos] & 3) << (19 - 2));
        out_array[22 + outpos] = (in_array[13 + inpos] >> 2) & 524287;
        out_array[23 + outpos] =
            (in_array[13 + inpos] >> 21) | ((in_array[14 + inpos] & 255) << (19 - 8));
        out_array[24 + outpos] = (in_array[14 + inpos] >> 8) & 524287;
        out_array[25 + outpos] =
            (in_array[14 + inpos] >> 27) | ((in_array[15 + inpos] & 16383) << (19 - 14));
        out_array[26 + outpos] =
            (in_array[15 + inpos] >> 14) | ((in_array[16 + inpos] & 1) << (19 - 1));
        out_array[27 + outpos] = (in_array[16 + inpos] >> 1) & 524287;
        out_array[28 + outpos] =
            (in_array[16 + inpos] >> 20) | ((in_array[17 + inpos] & 127) << (19 - 7));
        out_array[29 + outpos] = (in_array[17 + inpos] >> 7) & 524287;
        out_array[30 + outpos] =
            (in_array[17 + inpos] >> 26) | ((in_array[18 + inpos] & 8191) << (19 - 13));
        out_array[31 + outpos] = in_array[18 + inpos] >> 13;
    }

    pub fn fastunpack2(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 3;
        out_array[1 + outpos] = (in_array[inpos] >> 2) & 3;
        out_array[2 + outpos] = (in_array[inpos] >> 4) & 3;
        out_array[3 + outpos] = (in_array[inpos] >> 6) & 3;
        out_array[4 + outpos] = (in_array[inpos] >> 8) & 3;
        out_array[5 + outpos] = (in_array[inpos] >> 10) & 3;
        out_array[6 + outpos] = (in_array[inpos] >> 12) & 3;
        out_array[7 + outpos] = (in_array[inpos] >> 14) & 3;
        out_array[8 + outpos] = (in_array[inpos] >> 16) & 3;
        out_array[9 + outpos] = (in_array[inpos] >> 18) & 3;
        out_array[10 + outpos] = (in_array[inpos] >> 20) & 3;
        out_array[11 + outpos] = (in_array[inpos] >> 22) & 3;
        out_array[12 + outpos] = (in_array[inpos] >> 24) & 3;
        out_array[13 + outpos] = (in_array[inpos] >> 26) & 3;
        out_array[14 + outpos] = (in_array[inpos] >> 28) & 3;
        out_array[15 + outpos] = in_array[inpos] >> 30;
        out_array[16 + outpos] = (in_array[1 + inpos] >> 0) & 3;
        out_array[17 + outpos] = (in_array[1 + inpos] >> 2) & 3;
        out_array[18 + outpos] = (in_array[1 + inpos] >> 4) & 3;
        out_array[19 + outpos] = (in_array[1 + inpos] >> 6) & 3;
        out_array[20 + outpos] = (in_array[1 + inpos] >> 8) & 3;
        out_array[21 + outpos] = (in_array[1 + inpos] >> 10) & 3;
        out_array[22 + outpos] = (in_array[1 + inpos] >> 12) & 3;
        out_array[23 + outpos] = (in_array[1 + inpos] >> 14) & 3;
        out_array[24 + outpos] = (in_array[1 + inpos] >> 16) & 3;
        out_array[25 + outpos] = (in_array[1 + inpos] >> 18) & 3;
        out_array[26 + outpos] = (in_array[1 + inpos] >> 20) & 3;
        out_array[27 + outpos] = (in_array[1 + inpos] >> 22) & 3;
        out_array[28 + outpos] = (in_array[1 + inpos] >> 24) & 3;
        out_array[29 + outpos] = (in_array[1 + inpos] >> 26) & 3;
        out_array[30 + outpos] = (in_array[1 + inpos] >> 28) & 3;
        out_array[31 + outpos] = in_array[1 + inpos] >> 30;
    }

    pub fn fastunpack20(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 1048575;
        out_array[1 + outpos] = (in_array[inpos] >> 20) | ((in_array[1 + inpos] & 255) << (20 - 8));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 8) & 1048575;
        out_array[3 + outpos] =
            (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 65535) << (20 - 16));
        out_array[4 + outpos] = (in_array[2 + inpos] >> 16) | ((in_array[3 + inpos] & 15) << (20 - 4));
        out_array[5 + outpos] = (in_array[3 + inpos] >> 4) & 1048575;
        out_array[6 + outpos] =
            (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 4095) << (20 - 12));
        out_array[7 + outpos] = in_array[4 + inpos] >> 12;
        out_array[8 + outpos] = (in_array[5 + inpos] >> 0) & 1048575;
        out_array[9 + outpos] = (in_array[5 + inpos] >> 20) | ((in_array[6 + inpos] & 255) << (20 - 8));
        out_array[10 + outpos] = (in_array[6 + inpos] >> 8) & 1048575;
        out_array[11 + outpos] =
            (in_array[6 + inpos] >> 28) | ((in_array[7 + inpos] & 65535) << (20 - 16));
        out_array[12 + outpos] = (in_array[7 + inpos] >> 16) | ((in_array[8 + inpos] & 15) << (20 - 4));
        out_array[13 + outpos] = (in_array[8 + inpos] >> 4) & 1048575;
        out_array[14 + outpos] =
            (in_array[8 + inpos] >> 24) | ((in_array[9 + inpos] & 4095) << (20 - 12));
        out_array[15 + outpos] = in_array[9 + inpos] >> 12;
        out_array[16 + outpos] = (in_array[10 + inpos] >> 0) & 1048575;
        out_array[17 + outpos] =
            (in_array[10 + inpos] >> 20) | ((in_array[11 + inpos] & 255) << (20 - 8));
        out_array[18 + outpos] = (in_array[11 + inpos] >> 8) & 1048575;
        out_array[19 + outpos] =
            (in_array[11 + inpos] >> 28) | ((in_array[12 + inpos] & 65535) << (20 - 16));
        out_array[20 + outpos] =
            (in_array[12 + inpos] >> 16) | ((in_array[13 + inpos] & 15) << (20 - 4));
        out_array[21 + outpos] = (in_array[13 + inpos] >> 4) & 1048575;
        out_array[22 + outpos] =
            (in_array[13 + inpos] >> 24) | ((in_array[14 + inpos] & 4095) << (20 - 12));
        out_array[23 + outpos] = in_array[14 + inpos] >> 12;
        out_array[24 + outpos] = (in_array[15 + inpos] >> 0) & 1048575;
        out_array[25 + outpos] =
            (in_array[15 + inpos] >> 20) | ((in_array[16 + inpos] & 255) << (20 - 8));
        out_array[26 + outpos] = (in_array[16 + inpos] >> 8) & 1048575;
        out_array[27 + outpos] =
            (in_array[16 + inpos] >> 28) | ((in_array[17 + inpos] & 65535) << (20 - 16));
        out_array[28 + outpos] =
            (in_array[17 + inpos] >> 16) | ((in_array[18 + inpos] & 15) << (20 - 4));
        out_array[29 + outpos] = (in_array[18 + inpos] >> 4) & 1048575;
        out_array[30 + outpos] =
            (in_array[18 + inpos] >> 24) | ((in_array[19 + inpos] & 4095) << (20 - 12));
        out_array[31 + outpos] = in_array[19 + inpos] >> 12;
    }

    pub fn fastunpack21(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 2097151;
        out_array[1 + outpos] = (in_array[inpos] >> 21) | ((in_array[1 + inpos] & 1023) << (21 - 10));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 10) & 2097151;
        out_array[3 + outpos] =
            (in_array[1 + inpos] >> 31) | ((in_array[2 + inpos] & 1048575) << (21 - 20));
        out_array[4 + outpos] = (in_array[2 + inpos] >> 20) | ((in_array[3 + inpos] & 511) << (21 - 9));
        out_array[5 + outpos] = (in_array[3 + inpos] >> 9) & 2097151;
        out_array[6 + outpos] =
            (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 524287) << (21 - 19));
        out_array[7 + outpos] = (in_array[4 + inpos] >> 19) | ((in_array[5 + inpos] & 255) << (21 - 8));
        out_array[8 + outpos] = (in_array[5 + inpos] >> 8) & 2097151;
        out_array[9 + outpos] =
            (in_array[5 + inpos] >> 29) | ((in_array[6 + inpos] & 262143) << (21 - 18));
        out_array[10 + outpos] =
            (in_array[6 + inpos] >> 18) | ((in_array[7 + inpos] & 127) << (21 - 7));
        out_array[11 + outpos] = (in_array[7 + inpos] >> 7) & 2097151;
        out_array[12 + outpos] =
            (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 131071) << (21 - 17));
        out_array[13 + outpos] = (in_array[8 + inpos] >> 17) | ((in_array[9 + inpos] & 63) << (21 - 6));
        out_array[14 + outpos] = (in_array[9 + inpos] >> 6) & 2097151;
        out_array[15 + outpos] =
            (in_array[9 + inpos] >> 27) | ((in_array[10 + inpos] & 65535) << (21 - 16));
        out_array[16 + outpos] =
            (in_array[10 + inpos] >> 16) | ((in_array[11 + inpos] & 31) << (21 - 5));
        out_array[17 + outpos] = (in_array[11 + inpos] >> 5) & 2097151;
        out_array[18 + outpos] =
            (in_array[11 + inpos] >> 26) | ((in_array[12 + inpos] & 32767) << (21 - 15));
        out_array[19 + outpos] =
            (in_array[12 + inpos] >> 15) | ((in_array[13 + inpos] & 15) << (21 - 4));
        out_array[20 + outpos] = (in_array[13 + inpos] >> 4) & 2097151;
        out_array[21 + outpos] =
            (in_array[13 + inpos] >> 25) | ((in_array[14 + inpos] & 16383) << (21 - 14));
        out_array[22 + outpos] =
            (in_array[14 + inpos] >> 14) | ((in_array[15 + inpos] & 7) << (21 - 3));
        out_array[23 + outpos] = (in_array[15 + inpos] >> 3) & 2097151;
        out_array[24 + outpos] =
            (in_array[15 + inpos] >> 24) | ((in_array[16 + inpos] & 8191) << (21 - 13));
        out_array[25 + outpos] =
            (in_array[16 + inpos] >> 13) | ((in_array[17 + inpos] & 3) << (21 - 2));
        out_array[26 + outpos] = (in_array[17 + inpos] >> 2) & 2097151;
        out_array[27 + outpos] =
            (in_array[17 + inpos] >> 23) | ((in_array[18 + inpos] & 4095) << (21 - 12));
        out_array[28 + outpos] =
            (in_array[18 + inpos] >> 12) | ((in_array[19 + inpos] & 1) << (21 - 1));
        out_array[29 + outpos] = (in_array[19 + inpos] >> 1) & 2097151;
        out_array[30 + outpos] =
            (in_array[19 + inpos] >> 22) | ((in_array[20 + inpos] & 2047) << (21 - 11));
        out_array[31 + outpos] = in_array[20 + inpos] >> 11;
    }

    pub fn fastunpack22(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 4194303;
        out_array[1 + outpos] = (in_array[inpos] >> 22) | ((in_array[1 + inpos] & 4095) << (22 - 12));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 12) | ((in_array[2 + inpos] & 3) << (22 - 2));
        out_array[3 + outpos] = (in_array[2 + inpos] >> 2) & 4194303;
        out_array[4 + outpos] =
            (in_array[2 + inpos] >> 24) | ((in_array[3 + inpos] & 16383) << (22 - 14));
        out_array[5 + outpos] = (in_array[3 + inpos] >> 14) | ((in_array[4 + inpos] & 15) << (22 - 4));
        out_array[6 + outpos] = (in_array[4 + inpos] >> 4) & 4194303;
        out_array[7 + outpos] =
            (in_array[4 + inpos] >> 26) | ((in_array[5 + inpos] & 65535) << (22 - 16));
        out_array[8 + outpos] = (in_array[5 + inpos] >> 16) | ((in_array[6 + inpos] & 63) << (22 - 6));
        out_array[9 + outpos] = (in_array[6 + inpos] >> 6) & 4194303;
        out_array[10 + outpos] =
            (in_array[6 + inpos] >> 28) | ((in_array[7 + inpos] & 262143) << (22 - 18));
        out_array[11 + outpos] =
            (in_array[7 + inpos] >> 18) | ((in_array[8 + inpos] & 255) << (22 - 8));
        out_array[12 + outpos] = (in_array[8 + inpos] >> 8) & 4194303;
        out_array[13 + outpos] =
            (in_array[8 + inpos] >> 30) | ((in_array[9 + inpos] & 1048575) << (22 - 20));
        out_array[14 + outpos] =
            (in_array[9 + inpos] >> 20) | ((in_array[10 + inpos] & 1023) << (22 - 10));
        out_array[15 + outpos] = in_array[10 + inpos] >> 10;
        out_array[16 + outpos] = (in_array[11 + inpos] >> 0) & 4194303;
        out_array[17 + outpos] =
            (in_array[11 + inpos] >> 22) | ((in_array[12 + inpos] & 4095) << (22 - 12));
        out_array[18 + outpos] =
            (in_array[12 + inpos] >> 12) | ((in_array[13 + inpos] & 3) << (22 - 2));
        out_array[19 + outpos] = (in_array[13 + inpos] >> 2) & 4194303;
        out_array[20 + outpos] =
            (in_array[13 + inpos] >> 24) | ((in_array[14 + inpos] & 16383) << (22 - 14));
        out_array[21 + outpos] =
            (in_array[14 + inpos] >> 14) | ((in_array[15 + inpos] & 15) << (22 - 4));
        out_array[22 + outpos] = (in_array[15 + inpos] >> 4) & 4194303;
        out_array[23 + outpos] =
            (in_array[15 + inpos] >> 26) | ((in_array[16 + inpos] & 65535) << (22 - 16));
        out_array[24 + outpos] =
            (in_array[16 + inpos] >> 16) | ((in_array[17 + inpos] & 63) << (22 - 6));
        out_array[25 + outpos] = (in_array[17 + inpos] >> 6) & 4194303;
        out_array[26 + outpos] =
            (in_array[17 + inpos] >> 28) | ((in_array[18 + inpos] & 262143) << (22 - 18));
        out_array[27 + outpos] =
            (in_array[18 + inpos] >> 18) | ((in_array[19 + inpos] & 255) << (22 - 8));
        out_array[28 + outpos] = (in_array[19 + inpos] >> 8) & 4194303;
        out_array[29 + outpos] =
            (in_array[19 + inpos] >> 30) | ((in_array[20 + inpos] & 1048575) << (22 - 20));
        out_array[30 + outpos] =
            (in_array[20 + inpos] >> 20) | ((in_array[21 + inpos] & 1023) << (22 - 10));
        out_array[31 + outpos] = in_array[21 + inpos] >> 10;
    }

    pub fn fastunpack23(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 8388607;
        out_array[1 + outpos] = (in_array[inpos] >> 23) | ((in_array[1 + inpos] & 16383) << (23 - 14));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 14) | ((in_array[2 + inpos] & 31) << (23 - 5));
        out_array[3 + outpos] = (in_array[2 + inpos] >> 5) & 8388607;
        out_array[4 + outpos] =
            (in_array[2 + inpos] >> 28) | ((in_array[3 + inpos] & 524287) << (23 - 19));
        out_array[5 + outpos] =
            (in_array[3 + inpos] >> 19) | ((in_array[4 + inpos] & 1023) << (23 - 10));
        out_array[6 + outpos] = (in_array[4 + inpos] >> 10) | ((in_array[5 + inpos] & 1) << (23 - 1));
        out_array[7 + outpos] = (in_array[5 + inpos] >> 1) & 8388607;
        out_array[8 + outpos] =
            (in_array[5 + inpos] >> 24) | ((in_array[6 + inpos] & 32767) << (23 - 15));
        out_array[9 + outpos] = (in_array[6 + inpos] >> 15) | ((in_array[7 + inpos] & 63) << (23 - 6));
        out_array[10 + outpos] = (in_array[7 + inpos] >> 6) & 8388607;
        out_array[11 + outpos] =
            (in_array[7 + inpos] >> 29) | ((in_array[8 + inpos] & 1048575) << (23 - 20));
        out_array[12 + outpos] =
            (in_array[8 + inpos] >> 20) | ((in_array[9 + inpos] & 2047) << (23 - 11));
        out_array[13 + outpos] = (in_array[9 + inpos] >> 11) | ((in_array[10 + inpos] & 3) << (23 - 2));
        out_array[14 + outpos] = (in_array[10 + inpos] >> 2) & 8388607;
        out_array[15 + outpos] =
            (in_array[10 + inpos] >> 25) | ((in_array[11 + inpos] & 65535) << (23 - 16));
        out_array[16 + outpos] =
            (in_array[11 + inpos] >> 16) | ((in_array[12 + inpos] & 127) << (23 - 7));
        out_array[17 + outpos] = (in_array[12 + inpos] >> 7) & 8388607;
        out_array[18 + outpos] =
            (in_array[12 + inpos] >> 30) | ((in_array[13 + inpos] & 2097151) << (23 - 21));
        out_array[19 + outpos] =
            (in_array[13 + inpos] >> 21) | ((in_array[14 + inpos] & 4095) << (23 - 12));
        out_array[20 + outpos] =
            (in_array[14 + inpos] >> 12) | ((in_array[15 + inpos] & 7) << (23 - 3));
        out_array[21 + outpos] = (in_array[15 + inpos] >> 3) & 8388607;
        out_array[22 + outpos] =
            (in_array[15 + inpos] >> 26) | ((in_array[16 + inpos] & 131071) << (23 - 17));
        out_array[23 + outpos] =
            (in_array[16 + inpos] >> 17) | ((in_array[17 + inpos] & 255) << (23 - 8));
        out_array[24 + outpos] = (in_array[17 + inpos] >> 8) & 8388607;
        out_array[25 + outpos] =
            (in_array[17 + inpos] >> 31) | ((in_array[18 + inpos] & 4194303) << (23 - 22));
        out_array[26 + outpos] =
            (in_array[18 + inpos] >> 22) | ((in_array[19 + inpos] & 8191) << (23 - 13));
        out_array[27 + outpos] =
            (in_array[19 + inpos] >> 13) | ((in_array[20 + inpos] & 15) << (23 - 4));
        out_array[28 + outpos] = (in_array[20 + inpos] >> 4) & 8388607;
        out_array[29 + outpos] =
            (in_array[20 + inpos] >> 27) | ((in_array[21 + inpos] & 262143) << (23 - 18));
        out_array[30 + outpos] =
            (in_array[21 + inpos] >> 18) | ((in_array[22 + inpos] & 511) << (23 - 9));
        out_array[31 + outpos] = in_array[22 + inpos] >> 9;
    }

    pub fn fastunpack24(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 16777215;
        out_array[1 + outpos] = (in_array[inpos] >> 24) | ((in_array[1 + inpos] & 65535) << (24 - 16));
        out_array[2 + outpos] = (in_array[1 + inpos] >> 16) | ((in_array[2 + inpos] & 255) << (24 - 8));
        out_array[3 + outpos] = in_array[2 + inpos] >> 8;
        out_array[4 + outpos] = (in_array[3 + inpos] >> 0) & 16777215;
        out_array[5 + outpos] =
            (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 65535) << (24 - 16));
        out_array[6 + outpos] = (in_array[4 + inpos] >> 16) | ((in_array[5 + inpos] & 255) << (24 - 8));
        out_array[7 + outpos] = in_array[5 + inpos] >> 8;
        out_array[8 + outpos] = (in_array[6 + inpos] >> 0) & 16777215;
        out_array[9 + outpos] =
            (in_array[6 + inpos] >> 24) | ((in_array[7 + inpos] & 65535) << (24 - 16));
        out_array[10 + outpos] =
            (in_array[7 + inpos] >> 16) | ((in_array[8 + inpos] & 255) << (24 - 8));
        out_array[11 + outpos] = in_array[8 + inpos] >> 8;
        out_array[12 + outpos] = (in_array[9 + inpos] >> 0) & 16777215;
        out_array[13 + outpos] =
            (in_array[9 + inpos] >> 24) | ((in_array[10 + inpos] & 65535) << (24 - 16));
        out_array[14 + outpos] =
            (in_array[10 + inpos] >> 16) | ((in_array[11 + inpos] & 255) << (24 - 8));
        out_array[15 + outpos] = in_array[11 + inpos] >> 8;
        out_array[16 + outpos] = (in_array[12 + inpos] >> 0) & 16777215;
        out_array[17 + outpos] =
            (in_array[12 + inpos] >> 24) | ((in_array[13 + inpos] & 65535) << (24 - 16));
        out_array[18 + outpos] =
            (in_array[13 + inpos] >> 16) | ((in_array[14 + inpos] & 255) << (24 - 8));
        out_array[19 + outpos] = in_array[14 + inpos] >> 8;
        out_array[20 + outpos] = (in_array[15 + inpos] >> 0) & 16777215;
        out_array[21 + outpos] =
            (in_array[15 + inpos] >> 24) | ((in_array[16 + inpos] & 65535) << (24 - 16));
        out_array[22 + outpos] =
            (in_array[16 + inpos] >> 16) | ((in_array[17 + inpos] & 255) << (24 - 8));
        out_array[23 + outpos] = in_array[17 + inpos] >> 8;
        out_array[24 + outpos] = (in_array[18 + inpos] >> 0) & 16777215;
        out_array[25 + outpos] =
            (in_array[18 + inpos] >> 24) | ((in_array[19 + inpos] & 65535) << (24 - 16));
        out_array[26 + outpos] =
            (in_array[19 + inpos] >> 16) | ((in_array[20 + inpos] & 255) << (24 - 8));
        out_array[27 + outpos] = in_array[20 + inpos] >> 8;
        out_array[28 + outpos] = (in_array[21 + inpos] >> 0) & 16777215;
        out_array[29 + outpos] =
            (in_array[21 + inpos] >> 24) | ((in_array[22 + inpos] & 65535) << (24 - 16));
        out_array[30 + outpos] =
            (in_array[22 + inpos] >> 16) | ((in_array[23 + inpos] & 255) << (24 - 8));
        out_array[31 + outpos] = in_array[23 + inpos] >> 8;
    }

    pub fn fastunpack25(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 33554431;
        out_array[1 + outpos] = (in_array[inpos] >> 25) | ((in_array[1 + inpos] & 262143) << (25 - 18));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 18) | ((in_array[2 + inpos] & 2047) << (25 - 11));
        out_array[3 + outpos] = (in_array[2 + inpos] >> 11) | ((in_array[3 + inpos] & 15) << (25 - 4));
        out_array[4 + outpos] = (in_array[3 + inpos] >> 4) & 33554431;
        out_array[5 + outpos] =
            (in_array[3 + inpos] >> 29) | ((in_array[4 + inpos] & 4194303) << (25 - 22));
        out_array[6 + outpos] =
            (in_array[4 + inpos] >> 22) | ((in_array[5 + inpos] & 32767) << (25 - 15));
        out_array[7 + outpos] = (in_array[5 + inpos] >> 15) | ((in_array[6 + inpos] & 255) << (25 - 8));
        out_array[8 + outpos] = (in_array[6 + inpos] >> 8) | ((in_array[7 + inpos] & 1) << (25 - 1));
        out_array[9 + outpos] = (in_array[7 + inpos] >> 1) & 33554431;
        out_array[10 + outpos] =
            (in_array[7 + inpos] >> 26) | ((in_array[8 + inpos] & 524287) << (25 - 19));
        out_array[11 + outpos] =
            (in_array[8 + inpos] >> 19) | ((in_array[9 + inpos] & 4095) << (25 - 12));
        out_array[12 + outpos] =
            (in_array[9 + inpos] >> 12) | ((in_array[10 + inpos] & 31) << (25 - 5));
        out_array[13 + outpos] = (in_array[10 + inpos] >> 5) & 33554431;
        out_array[14 + outpos] =
            (in_array[10 + inpos] >> 30) | ((in_array[11 + inpos] & 8388607) << (25 - 23));
        out_array[15 + outpos] =
            (in_array[11 + inpos] >> 23) | ((in_array[12 + inpos] & 65535) << (25 - 16));
        out_array[16 + outpos] =
            (in_array[12 + inpos] >> 16) | ((in_array[13 + inpos] & 511) << (25 - 9));
        out_array[17 + outpos] = (in_array[13 + inpos] >> 9) | ((in_array[14 + inpos] & 3) << (25 - 2));
        out_array[18 + outpos] = (in_array[14 + inpos] >> 2) & 33554431;
        out_array[19 + outpos] =
            (in_array[14 + inpos] >> 27) | ((in_array[15 + inpos] & 1048575) << (25 - 20));
        out_array[20 + outpos] =
            (in_array[15 + inpos] >> 20) | ((in_array[16 + inpos] & 8191) << (25 - 13));
        out_array[21 + outpos] =
            (in_array[16 + inpos] >> 13) | ((in_array[17 + inpos] & 63) << (25 - 6));
        out_array[22 + outpos] = (in_array[17 + inpos] >> 6) & 33554431;
        out_array[23 + outpos] =
            (in_array[17 + inpos] >> 31) | ((in_array[18 + inpos] & 16777215) << (25 - 24));
        out_array[24 + outpos] =
            (in_array[18 + inpos] >> 24) | ((in_array[19 + inpos] & 131071) << (25 - 17));
        out_array[25 + outpos] =
            (in_array[19 + inpos] >> 17) | ((in_array[20 + inpos] & 1023) << (25 - 10));
        out_array[26 + outpos] =
            (in_array[20 + inpos] >> 10) | ((in_array[21 + inpos] & 7) << (25 - 3));
        out_array[27 + outpos] = (in_array[21 + inpos] >> 3) & 33554431;
        out_array[28 + outpos] =
            (in_array[21 + inpos] >> 28) | ((in_array[22 + inpos] & 2097151) << (25 - 21));
        out_array[29 + outpos] =
            (in_array[22 + inpos] >> 21) | ((in_array[23 + inpos] & 16383) << (25 - 14));
        out_array[30 + outpos] =
            (in_array[23 + inpos] >> 14) | ((in_array[24 + inpos] & 127) << (25 - 7));
        out_array[31 + outpos] = in_array[24 + inpos] >> 7;
    }

    pub fn fastunpack26(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 67108863;
        out_array[1 + outpos] =
            (in_array[inpos] >> 26) | ((in_array[1 + inpos] & 1048575) << (26 - 20));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 20) | ((in_array[2 + inpos] & 16383) << (26 - 14));
        out_array[3 + outpos] = (in_array[2 + inpos] >> 14) | ((in_array[3 + inpos] & 255) << (26 - 8));
        out_array[4 + outpos] = (in_array[3 + inpos] >> 8) | ((in_array[4 + inpos] & 3) << (26 - 2));
        out_array[5 + outpos] = (in_array[4 + inpos] >> 2) & 67108863;
        out_array[6 + outpos] =
            (in_array[4 + inpos] >> 28) | ((in_array[5 + inpos] & 4194303) << (26 - 22));
        out_array[7 + outpos] =
            (in_array[5 + inpos] >> 22) | ((in_array[6 + inpos] & 65535) << (26 - 16));
        out_array[8 + outpos] =
            (in_array[6 + inpos] >> 16) | ((in_array[7 + inpos] & 1023) << (26 - 10));
        out_array[9 + outpos] = (in_array[7 + inpos] >> 10) | ((in_array[8 + inpos] & 15) << (26 - 4));
        out_array[10 + outpos] = (in_array[8 + inpos] >> 4) & 67108863;
        out_array[11 + outpos] =
            (in_array[8 + inpos] >> 30) | ((in_array[9 + inpos] & 16777215) << (26 - 24));
        out_array[12 + outpos] =
            (in_array[9 + inpos] >> 24) | ((in_array[10 + inpos] & 262143) << (26 - 18));
        out_array[13 + outpos] =
            (in_array[10 + inpos] >> 18) | ((in_array[11 + inpos] & 4095) << (26 - 12));
        out_array[14 + outpos] =
            (in_array[11 + inpos] >> 12) | ((in_array[12 + inpos] & 63) << (26 - 6));
        out_array[15 + outpos] = in_array[12 + inpos] >> 6;
        out_array[16 + outpos] = (in_array[13 + inpos] >> 0) & 67108863;
        out_array[17 + outpos] =
            (in_array[13 + inpos] >> 26) | ((in_array[14 + inpos] & 1048575) << (26 - 20));
        out_array[18 + outpos] =
            (in_array[14 + inpos] >> 20) | ((in_array[15 + inpos] & 16383) << (26 - 14));
        out_array[19 + outpos] =
            (in_array[15 + inpos] >> 14) | ((in_array[16 + inpos] & 255) << (26 - 8));
        out_array[20 + outpos] = (in_array[16 + inpos] >> 8) | ((in_array[17 + inpos] & 3) << (26 - 2));
        out_array[21 + outpos] = (in_array[17 + inpos] >> 2) & 67108863;
        out_array[22 + outpos] =
            (in_array[17 + inpos] >> 28) | ((in_array[18 + inpos] & 4194303) << (26 - 22));
        out_array[23 + outpos] =
            (in_array[18 + inpos] >> 22) | ((in_array[19 + inpos] & 65535) << (26 - 16));
        out_array[24 + outpos] =
            (in_array[19 + inpos] >> 16) | ((in_array[20 + inpos] & 1023) << (26 - 10));
        out_array[25 + outpos] =
            (in_array[20 + inpos] >> 10) | ((in_array[21 + inpos] & 15) << (26 - 4));
        out_array[26 + outpos] = (in_array[21 + inpos] >> 4) & 67108863;
        out_array[27 + outpos] =
            (in_array[21 + inpos] >> 30) | ((in_array[22 + inpos] & 16777215) << (26 - 24));
        out_array[28 + outpos] =
            (in_array[22 + inpos] >> 24) | ((in_array[23 + inpos] & 262143) << (26 - 18));
        out_array[29 + outpos] =
            (in_array[23 + inpos] >> 18) | ((in_array[24 + inpos] & 4095) << (26 - 12));
        out_array[30 + outpos] =
            (in_array[24 + inpos] >> 12) | ((in_array[25 + inpos] & 63) << (26 - 6));
        out_array[31 + outpos] = in_array[25 + inpos] >> 6;
    }

    pub fn fastunpack27(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 134217727;
        out_array[1 + outpos] =
            (in_array[inpos] >> 27) | ((in_array[1 + inpos] & 4194303) << (27 - 22));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 22) | ((in_array[2 + inpos] & 131071) << (27 - 17));
        out_array[3 + outpos] =
            (in_array[2 + inpos] >> 17) | ((in_array[3 + inpos] & 4095) << (27 - 12));
        out_array[4 + outpos] = (in_array[3 + inpos] >> 12) | ((in_array[4 + inpos] & 127) << (27 - 7));
        out_array[5 + outpos] = (in_array[4 + inpos] >> 7) | ((in_array[5 + inpos] & 3) << (27 - 2));
        out_array[6 + outpos] = (in_array[5 + inpos] >> 2) & 134217727;
        out_array[7 + outpos] =
            (in_array[5 + inpos] >> 29) | ((in_array[6 + inpos] & 16777215) << (27 - 24));
        out_array[8 + outpos] =
            (in_array[6 + inpos] >> 24) | ((in_array[7 + inpos] & 524287) << (27 - 19));
        out_array[9 + outpos] =
            (in_array[7 + inpos] >> 19) | ((in_array[8 + inpos] & 16383) << (27 - 14));
        out_array[10 + outpos] =
            (in_array[8 + inpos] >> 14) | ((in_array[9 + inpos] & 511) << (27 - 9));
        out_array[11 + outpos] = (in_array[9 + inpos] >> 9) | ((in_array[10 + inpos] & 15) << (27 - 4));
        out_array[12 + outpos] = (in_array[10 + inpos] >> 4) & 134217727;
        out_array[13 + outpos] =
            (in_array[10 + inpos] >> 31) | ((in_array[11 + inpos] & 67108863) << (27 - 26));
        out_array[14 + outpos] =
            (in_array[11 + inpos] >> 26) | ((in_array[12 + inpos] & 2097151) << (27 - 21));
        out_array[15 + outpos] =
            (in_array[12 + inpos] >> 21) | ((in_array[13 + inpos] & 65535) << (27 - 16));
        out_array[16 + outpos] =
            (in_array[13 + inpos] >> 16) | ((in_array[14 + inpos] & 2047) << (27 - 11));
        out_array[17 + outpos] =
            (in_array[14 + inpos] >> 11) | ((in_array[15 + inpos] & 63) << (27 - 6));
        out_array[18 + outpos] = (in_array[15 + inpos] >> 6) | ((in_array[16 + inpos] & 1) << (27 - 1));
        out_array[19 + outpos] = (in_array[16 + inpos] >> 1) & 134217727;
        out_array[20 + outpos] =
            (in_array[16 + inpos] >> 28) | ((in_array[17 + inpos] & 8388607) << (27 - 23));
        out_array[21 + outpos] =
            (in_array[17 + inpos] >> 23) | ((in_array[18 + inpos] & 262143) << (27 - 18));
        out_array[22 + outpos] =
            (in_array[18 + inpos] >> 18) | ((in_array[19 + inpos] & 8191) << (27 - 13));
        out_array[23 + outpos] =
            (in_array[19 + inpos] >> 13) | ((in_array[20 + inpos] & 255) << (27 - 8));
        out_array[24 + outpos] = (in_array[20 + inpos] >> 8) | ((in_array[21 + inpos] & 7) << (27 - 3));
        out_array[25 + outpos] = (in_array[21 + inpos] >> 3) & 134217727;
        out_array[26 + outpos] =
            (in_array[21 + inpos] >> 30) | ((in_array[22 + inpos] & 33554431) << (27 - 25));
        out_array[27 + outpos] =
            (in_array[22 + inpos] >> 25) | ((in_array[23 + inpos] & 1048575) << (27 - 20));
        out_array[28 + outpos] =
            (in_array[23 + inpos] >> 20) | ((in_array[24 + inpos] & 32767) << (27 - 15));
        out_array[29 + outpos] =
            (in_array[24 + inpos] >> 15) | ((in_array[25 + inpos] & 1023) << (27 - 10));
        out_array[30 + outpos] =
            (in_array[25 + inpos] >> 10) | ((in_array[26 + inpos] & 31) << (27 - 5));
        out_array[31 + outpos] = in_array[26 + inpos] >> 5;
    }

    pub fn fastunpack28(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 268435455;
        out_array[1 + outpos] =
            (in_array[inpos] >> 28) | ((in_array[1 + inpos] & 16777215) << (28 - 24));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 24) | ((in_array[2 + inpos] & 1048575) << (28 - 20));
        out_array[3 + outpos] =
            (in_array[2 + inpos] >> 20) | ((in_array[3 + inpos] & 65535) << (28 - 16));
        out_array[4 + outpos] =
            (in_array[3 + inpos] >> 16) | ((in_array[4 + inpos] & 4095) << (28 - 12));
        out_array[5 + outpos] = (in_array[4 + inpos] >> 12) | ((in_array[5 + inpos] & 255) << (28 - 8));
        out_array[6 + outpos] = (in_array[5 + inpos] >> 8) | ((in_array[6 + inpos] & 15) << (28 - 4));
        out_array[7 + outpos] = in_array[6 + inpos] >> 4;
        out_array[8 + outpos] = (in_array[7 + inpos] >> 0) & 268435455;
        out_array[9 + outpos] =
            (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 16777215) << (28 - 24));
        out_array[10 + outpos] =
            (in_array[8 + inpos] >> 24) | ((in_array[9 + inpos] & 1048575) << (28 - 20));
        out_array[11 + outpos] =
            (in_array[9 + inpos] >> 20) | ((in_array[10 + inpos] & 65535) << (28 - 16));
        out_array[12 + outpos] =
            (in_array[10 + inpos] >> 16) | ((in_array[11 + inpos] & 4095) << (28 - 12));
        out_array[13 + outpos] =
            (in_array[11 + inpos] >> 12) | ((in_array[12 + inpos] & 255) << (28 - 8));
        out_array[14 + outpos] =
            (in_array[12 + inpos] >> 8) | ((in_array[13 + inpos] & 15) << (28 - 4));
        out_array[15 + outpos] = in_array[13 + inpos] >> 4;
        out_array[16 + outpos] = (in_array[14 + inpos] >> 0) & 268435455;
        out_array[17 + outpos] =
            (in_array[14 + inpos] >> 28) | ((in_array[15 + inpos] & 16777215) << (28 - 24));
        out_array[18 + outpos] =
            (in_array[15 + inpos] >> 24) | ((in_array[16 + inpos] & 1048575) << (28 - 20));
        out_array[19 + outpos] =
            (in_array[16 + inpos] >> 20) | ((in_array[17 + inpos] & 65535) << (28 - 16));
        out_array[20 + outpos] =
            (in_array[17 + inpos] >> 16) | ((in_array[18 + inpos] & 4095) << (28 - 12));
        out_array[21 + outpos] =
            (in_array[18 + inpos] >> 12) | ((in_array[19 + inpos] & 255) << (28 - 8));
        out_array[22 + outpos] =
            (in_array[19 + inpos] >> 8) | ((in_array[20 + inpos] & 15) << (28 - 4));
        out_array[23 + outpos] = in_array[20 + inpos] >> 4;
        out_array[24 + outpos] = (in_array[21 + inpos] >> 0) & 268435455;
        out_array[25 + outpos] =
            (in_array[21 + inpos] >> 28) | ((in_array[22 + inpos] & 16777215) << (28 - 24));
        out_array[26 + outpos] =
            (in_array[22 + inpos] >> 24) | ((in_array[23 + inpos] & 1048575) << (28 - 20));
        out_array[27 + outpos] =
            (in_array[23 + inpos] >> 20) | ((in_array[24 + inpos] & 65535) << (28 - 16));
        out_array[28 + outpos] =
            (in_array[24 + inpos] >> 16) | ((in_array[25 + inpos] & 4095) << (28 - 12));
        out_array[29 + outpos] =
            (in_array[25 + inpos] >> 12) | ((in_array[26 + inpos] & 255) << (28 - 8));
        out_array[30 + outpos] =
            (in_array[26 + inpos] >> 8) | ((in_array[27 + inpos] & 15) << (28 - 4));
        out_array[31 + outpos] = in_array[27 + inpos] >> 4;
    }

    pub fn fastunpack29(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 536870911;
        out_array[1 + outpos] =
            (in_array[inpos] >> 29) | ((in_array[1 + inpos] & 67108863) << (29 - 26));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 26) | ((in_array[2 + inpos] & 8388607) << (29 - 23));
        out_array[3 + outpos] =
            (in_array[2 + inpos] >> 23) | ((in_array[3 + inpos] & 1048575) << (29 - 20));
        out_array[4 + outpos] =
            (in_array[3 + inpos] >> 20) | ((in_array[4 + inpos] & 131071) << (29 - 17));
        out_array[5 + outpos] =
            (in_array[4 + inpos] >> 17) | ((in_array[5 + inpos] & 16383) << (29 - 14));
        out_array[6 + outpos] =
            (in_array[5 + inpos] >> 14) | ((in_array[6 + inpos] & 2047) << (29 - 11));
        out_array[7 + outpos] = (in_array[6 + inpos] >> 11) | ((in_array[7 + inpos] & 255) << (29 - 8));
        out_array[8 + outpos] = (in_array[7 + inpos] >> 8) | ((in_array[8 + inpos] & 31) << (29 - 5));
        out_array[9 + outpos] = (in_array[8 + inpos] >> 5) | ((in_array[9 + inpos] & 3) << (29 - 2));
        out_array[10 + outpos] = (in_array[9 + inpos] >> 2) & 536870911;
        out_array[11 + outpos] =
            (in_array[9 + inpos] >> 31) | ((in_array[10 + inpos] & 268435455) << (29 - 28));
        out_array[12 + outpos] =
            (in_array[10 + inpos] >> 28) | ((in_array[11 + inpos] & 33554431) << (29 - 25));
        out_array[13 + outpos] =
            (in_array[11 + inpos] >> 25) | ((in_array[12 + inpos] & 4194303) << (29 - 22));
        out_array[14 + outpos] =
            (in_array[12 + inpos] >> 22) | ((in_array[13 + inpos] & 524287) << (29 - 19));
        out_array[15 + outpos] =
            (in_array[13 + inpos] >> 19) | ((in_array[14 + inpos] & 65535) << (29 - 16));
        out_array[16 + outpos] =
            (in_array[14 + inpos] >> 16) | ((in_array[15 + inpos] & 8191) << (29 - 13));
        out_array[17 + outpos] =
            (in_array[15 + inpos] >> 13) | ((in_array[16 + inpos] & 1023) << (29 - 10));
        out_array[18 + outpos] =
            (in_array[16 + inpos] >> 10) | ((in_array[17 + inpos] & 127) << (29 - 7));
        out_array[19 + outpos] =
            (in_array[17 + inpos] >> 7) | ((in_array[18 + inpos] & 15) << (29 - 4));
        out_array[20 + outpos] = (in_array[18 + inpos] >> 4) | ((in_array[19 + inpos] & 1) << (29 - 1));
        out_array[21 + outpos] = (in_array[19 + inpos] >> 1) & 536870911;
        out_array[22 + outpos] =
            (in_array[19 + inpos] >> 30) | ((in_array[20 + inpos] & 134217727) << (29 - 27));
        out_array[23 + outpos] =
            (in_array[20 + inpos] >> 27) | ((in_array[21 + inpos] & 16777215) << (29 - 24));
        out_array[24 + outpos] =
            (in_array[21 + inpos] >> 24) | ((in_array[22 + inpos] & 2097151) << (29 - 21));
        out_array[25 + outpos] =
            (in_array[22 + inpos] >> 21) | ((in_array[23 + inpos] & 262143) << (29 - 18));
        out_array[26 + outpos] =
            (in_array[23 + inpos] >> 18) | ((in_array[24 + inpos] & 32767) << (29 - 15));
        out_array[27 + outpos] =
            (in_array[24 + inpos] >> 15) | ((in_array[25 + inpos] & 4095) << (29 - 12));
        out_array[28 + outpos] =
            (in_array[25 + inpos] >> 12) | ((in_array[26 + inpos] & 511) << (29 - 9));
        out_array[29 + outpos] =
            (in_array[26 + inpos] >> 9) | ((in_array[27 + inpos] & 63) << (29 - 6));
        out_array[30 + outpos] = (in_array[27 + inpos] >> 6) | ((in_array[28 + inpos] & 7) << (29 - 3));
        out_array[31 + outpos] = in_array[28 + inpos] >> 3;
    }

    pub fn fastunpack3(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 7;
        out_array[1 + outpos] = (in_array[inpos] >> 3) & 7;
        out_array[2 + outpos] = (in_array[inpos] >> 6) & 7;
        out_array[3 + outpos] = (in_array[inpos] >> 9) & 7;
        out_array[4 + outpos] = (in_array[inpos] >> 12) & 7;
        out_array[5 + outpos] = (in_array[inpos] >> 15) & 7;
        out_array[6 + outpos] = (in_array[inpos] >> 18) & 7;
        out_array[7 + outpos] = (in_array[inpos] >> 21) & 7;
        out_array[8 + outpos] = (in_array[inpos] >> 24) & 7;
        out_array[9 + outpos] = (in_array[inpos] >> 27) & 7;
        out_array[10 + outpos] = (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 1) << (3 - 1));
        out_array[11 + outpos] = (in_array[1 + inpos] >> 1) & 7;
        out_array[12 + outpos] = (in_array[1 + inpos] >> 4) & 7;
        out_array[13 + outpos] = (in_array[1 + inpos] >> 7) & 7;
        out_array[14 + outpos] = (in_array[1 + inpos] >> 10) & 7;
        out_array[15 + outpos] = (in_array[1 + inpos] >> 13) & 7;
        out_array[16 + outpos] = (in_array[1 + inpos] >> 16) & 7;
        out_array[17 + outpos] = (in_array[1 + inpos] >> 19) & 7;
        out_array[18 + outpos] = (in_array[1 + inpos] >> 22) & 7;
        out_array[19 + outpos] = (in_array[1 + inpos] >> 25) & 7;
        out_array[20 + outpos] = (in_array[1 + inpos] >> 28) & 7;
        out_array[21 + outpos] = (in_array[1 + inpos] >> 31) | ((in_array[2 + inpos] & 3) << (3 - 2));
        out_array[22 + outpos] = (in_array[2 + inpos] >> 2) & 7;
        out_array[23 + outpos] = (in_array[2 + inpos] >> 5) & 7;
        out_array[24 + outpos] = (in_array[2 + inpos] >> 8) & 7;
        out_array[25 + outpos] = (in_array[2 + inpos] >> 11) & 7;
        out_array[26 + outpos] = (in_array[2 + inpos] >> 14) & 7;
        out_array[27 + outpos] = (in_array[2 + inpos] >> 17) & 7;
        out_array[28 + outpos] = (in_array[2 + inpos] >> 20) & 7;
        out_array[29 + outpos] = (in_array[2 + inpos] >> 23) & 7;
        out_array[30 + outpos] = (in_array[2 + inpos] >> 26) & 7;
        out_array[31 + outpos] = in_array[2 + inpos] >> 29;
    }

    pub fn fastunpack30(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 1073741823;
        out_array[1 + outpos] =
            (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 268435455) << (30 - 28));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 67108863) << (30 - 26));
        out_array[3 + outpos] =
            (in_array[2 + inpos] >> 26) | ((in_array[3 + inpos] & 16777215) << (30 - 24));
        out_array[4 + outpos] =
            (in_array[3 + inpos] >> 24) | ((in_array[4 + inpos] & 4194303) << (30 - 22));
        out_array[5 + outpos] =
            (in_array[4 + inpos] >> 22) | ((in_array[5 + inpos] & 1048575) << (30 - 20));
        out_array[6 + outpos] =
            (in_array[5 + inpos] >> 20) | ((in_array[6 + inpos] & 262143) << (30 - 18));
        out_array[7 + outpos] =
            (in_array[6 + inpos] >> 18) | ((in_array[7 + inpos] & 65535) << (30 - 16));
        out_array[8 + outpos] =
            (in_array[7 + inpos] >> 16) | ((in_array[8 + inpos] & 16383) << (30 - 14));
        out_array[9 + outpos] =
            (in_array[8 + inpos] >> 14) | ((in_array[9 + inpos] & 4095) << (30 - 12));
        out_array[10 + outpos] =
            (in_array[9 + inpos] >> 12) | ((in_array[10 + inpos] & 1023) << (30 - 10));
        out_array[11 + outpos] =
            (in_array[10 + inpos] >> 10) | ((in_array[11 + inpos] & 255) << (30 - 8));
        out_array[12 + outpos] =
            (in_array[11 + inpos] >> 8) | ((in_array[12 + inpos] & 63) << (30 - 6));
        out_array[13 + outpos] =
            (in_array[12 + inpos] >> 6) | ((in_array[13 + inpos] & 15) << (30 - 4));
        out_array[14 + outpos] = (in_array[13 + inpos] >> 4) | ((in_array[14 + inpos] & 3) << (30 - 2));
        out_array[15 + outpos] = in_array[14 + inpos] >> 2;
        out_array[16 + outpos] = (in_array[15 + inpos] >> 0) & 1073741823;
        out_array[17 + outpos] =
            (in_array[15 + inpos] >> 30) | ((in_array[16 + inpos] & 268435455) << (30 - 28));
        out_array[18 + outpos] =
            (in_array[16 + inpos] >> 28) | ((in_array[17 + inpos] & 67108863) << (30 - 26));
        out_array[19 + outpos] =
            (in_array[17 + inpos] >> 26) | ((in_array[18 + inpos] & 16777215) << (30 - 24));
        out_array[20 + outpos] =
            (in_array[18 + inpos] >> 24) | ((in_array[19 + inpos] & 4194303) << (30 - 22));
        out_array[21 + outpos] =
            (in_array[19 + inpos] >> 22) | ((in_array[20 + inpos] & 1048575) << (30 - 20));
        out_array[22 + outpos] =
            (in_array[20 + inpos] >> 20) | ((in_array[21 + inpos] & 262143) << (30 - 18));
        out_array[23 + outpos] =
            (in_array[21 + inpos] >> 18) | ((in_array[22 + inpos] & 65535) << (30 - 16));
        out_array[24 + outpos] =
            (in_array[22 + inpos] >> 16) | ((in_array[23 + inpos] & 16383) << (30 - 14));
        out_array[25 + outpos] =
            (in_array[23 + inpos] >> 14) | ((in_array[24 + inpos] & 4095) << (30 - 12));
        out_array[26 + outpos] =
            (in_array[24 + inpos] >> 12) | ((in_array[25 + inpos] & 1023) << (30 - 10));
        out_array[27 + outpos] =
            (in_array[25 + inpos] >> 10) | ((in_array[26 + inpos] & 255) << (30 - 8));
        out_array[28 + outpos] =
            (in_array[26 + inpos] >> 8) | ((in_array[27 + inpos] & 63) << (30 - 6));
        out_array[29 + outpos] =
            (in_array[27 + inpos] >> 6) | ((in_array[28 + inpos] & 15) << (30 - 4));
        out_array[30 + outpos] = (in_array[28 + inpos] >> 4) | ((in_array[29 + inpos] & 3) << (30 - 2));
        out_array[31 + outpos] = in_array[29 + inpos] >> 2;
    }

    pub fn fastunpack31(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 2147483647;
        out_array[1 + outpos] =
            (in_array[inpos] >> 31) | ((in_array[1 + inpos] & 1073741823) << (31 - 30));
        out_array[2 + outpos] =
            (in_array[1 + inpos] >> 30) | ((in_array[2 + inpos] & 536870911) << (31 - 29));
        out_array[3 + outpos] =
            (in_array[2 + inpos] >> 29) | ((in_array[3 + inpos] & 268435455) << (31 - 28));
        out_array[4 + outpos] =
            (in_array[3 + inpos] >> 28) | ((in_array[4 + inpos] & 134217727) << (31 - 27));
        out_array[5 + outpos] =
            (in_array[4 + inpos] >> 27) | ((in_array[5 + inpos] & 67108863) << (31 - 26));
        out_array[6 + outpos] =
            (in_array[5 + inpos] >> 26) | ((in_array[6 + inpos] & 33554431) << (31 - 25));
        out_array[7 + outpos] =
            (in_array[6 + inpos] >> 25) | ((in_array[7 + inpos] & 16777215) << (31 - 24));
        out_array[8 + outpos] =
            (in_array[7 + inpos] >> 24) | ((in_array[8 + inpos] & 8388607) << (31 - 23));
        out_array[9 + outpos] =
            (in_array[8 + inpos] >> 23) | ((in_array[9 + inpos] & 4194303) << (31 - 22));
        out_array[10 + outpos] =
            (in_array[9 + inpos] >> 22) | ((in_array[10 + inpos] & 2097151) << (31 - 21));
        out_array[11 + outpos] =
            (in_array[10 + inpos] >> 21) | ((in_array[11 + inpos] & 1048575) << (31 - 20));
        out_array[12 + outpos] =
            (in_array[11 + inpos] >> 20) | ((in_array[12 + inpos] & 524287) << (31 - 19));
        out_array[13 + outpos] =
            (in_array[12 + inpos] >> 19) | ((in_array[13 + inpos] & 262143) << (31 - 18));
        out_array[14 + outpos] =
            (in_array[13 + inpos] >> 18) | ((in_array[14 + inpos] & 131071) << (31 - 17));
        out_array[15 + outpos] =
            (in_array[14 + inpos] >> 17) | ((in_array[15 + inpos] & 65535) << (31 - 16));
        out_array[16 + outpos] =
            (in_array[15 + inpos] >> 16) | ((in_array[16 + inpos] & 32767) << (31 - 15));
        out_array[17 + outpos] =
            (in_array[16 + inpos] >> 15) | ((in_array[17 + inpos] & 16383) << (31 - 14));
        out_array[18 + outpos] =
            (in_array[17 + inpos] >> 14) | ((in_array[18 + inpos] & 8191) << (31 - 13));
        out_array[19 + outpos] =
            (in_array[18 + inpos] >> 13) | ((in_array[19 + inpos] & 4095) << (31 - 12));
        out_array[20 + outpos] =
            (in_array[19 + inpos] >> 12) | ((in_array[20 + inpos] & 2047) << (31 - 11));
        out_array[21 + outpos] =
            (in_array[20 + inpos] >> 11) | ((in_array[21 + inpos] & 1023) << (31 - 10));
        out_array[22 + outpos] =
            (in_array[21 + inpos] >> 10) | ((in_array[22 + inpos] & 511) << (31 - 9));
        out_array[23 + outpos] =
            (in_array[22 + inpos] >> 9) | ((in_array[23 + inpos] & 255) << (31 - 8));
        out_array[24 + outpos] =
            (in_array[23 + inpos] >> 8) | ((in_array[24 + inpos] & 127) << (31 - 7));
        out_array[25 + outpos] =
            (in_array[24 + inpos] >> 7) | ((in_array[25 + inpos] & 63) << (31 - 6));
        out_array[26 + outpos] =
            (in_array[25 + inpos] >> 6) | ((in_array[26 + inpos] & 31) << (31 - 5));
        out_array[27 + outpos] =
            (in_array[26 + inpos] >> 5) | ((in_array[27 + inpos] & 15) << (31 - 4));
        out_array[28 + outpos] = (in_array[27 + inpos] >> 4) | ((in_array[28 + inpos] & 7) << (31 - 3));
        out_array[29 + outpos] = (in_array[28 + inpos] >> 3) | ((in_array[29 + inpos] & 3) << (31 - 2));
        out_array[30 + outpos] = (in_array[29 + inpos] >> 2) | ((in_array[30 + inpos] & 1) << (31 - 1));
        out_array[31 + outpos] = in_array[30 + inpos] >> 1;
    }

    pub fn fastunpack32(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        for i in 0..32 {
            out_array[outpos + i] = in_array[inpos + i];
        }
    }

    pub fn fastunpack4(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 15;
        out_array[1 + outpos] = (in_array[inpos] >> 4) & 15;
        out_array[2 + outpos] = (in_array[inpos] >> 8) & 15;
        out_array[3 + outpos] = (in_array[inpos] >> 12) & 15;
        out_array[4 + outpos] = (in_array[inpos] >> 16) & 15;
        out_array[5 + outpos] = (in_array[inpos] >> 20) & 15;
        out_array[6 + outpos] = (in_array[inpos] >> 24) & 15;
        out_array[7 + outpos] = in_array[inpos] >> 28;
        out_array[8 + outpos] = (in_array[1 + inpos] >> 0) & 15;
        out_array[9 + outpos] = (in_array[1 + inpos] >> 4) & 15;
        out_array[10 + outpos] = (in_array[1 + inpos] >> 8) & 15;
        out_array[11 + outpos] = (in_array[1 + inpos] >> 12) & 15;
        out_array[12 + outpos] = (in_array[1 + inpos] >> 16) & 15;
        out_array[13 + outpos] = (in_array[1 + inpos] >> 20) & 15;
        out_array[14 + outpos] = (in_array[1 + inpos] >> 24) & 15;
        out_array[15 + outpos] = in_array[1 + inpos] >> 28;
        out_array[16 + outpos] = (in_array[2 + inpos] >> 0) & 15;
        out_array[17 + outpos] = (in_array[2 + inpos] >> 4) & 15;
        out_array[18 + outpos] = (in_array[2 + inpos] >> 8) & 15;
        out_array[19 + outpos] = (in_array[2 + inpos] >> 12) & 15;
        out_array[20 + outpos] = (in_array[2 + inpos] >> 16) & 15;
        out_array[21 + outpos] = (in_array[2 + inpos] >> 20) & 15;
        out_array[22 + outpos] = (in_array[2 + inpos] >> 24) & 15;
        out_array[23 + outpos] = in_array[2 + inpos] >> 28;
        out_array[24 + outpos] = (in_array[3 + inpos] >> 0) & 15;
        out_array[25 + outpos] = (in_array[3 + inpos] >> 4) & 15;
        out_array[26 + outpos] = (in_array[3 + inpos] >> 8) & 15;
        out_array[27 + outpos] = (in_array[3 + inpos] >> 12) & 15;
        out_array[28 + outpos] = (in_array[3 + inpos] >> 16) & 15;
        out_array[29 + outpos] = (in_array[3 + inpos] >> 20) & 15;
        out_array[30 + outpos] = (in_array[3 + inpos] >> 24) & 15;
        out_array[31 + outpos] = in_array[3 + inpos] >> 28;
    }

    pub fn fastunpack5(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 31;
        out_array[1 + outpos] = (in_array[inpos] >> 5) & 31;
        out_array[2 + outpos] = (in_array[inpos] >> 10) & 31;
        out_array[3 + outpos] = (in_array[inpos] >> 15) & 31;
        out_array[4 + outpos] = (in_array[inpos] >> 20) & 31;
        out_array[5 + outpos] = (in_array[inpos] >> 25) & 31;
        out_array[6 + outpos] = (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 7) << (5 - 3));
        out_array[7 + outpos] = (in_array[1 + inpos] >> 3) & 31;
        out_array[8 + outpos] = (in_array[1 + inpos] >> 8) & 31;
        out_array[9 + outpos] = (in_array[1 + inpos] >> 13) & 31;
        out_array[10 + outpos] = (in_array[1 + inpos] >> 18) & 31;
        out_array[11 + outpos] = (in_array[1 + inpos] >> 23) & 31;
        out_array[12 + outpos] = (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 1) << (5 - 1));
        out_array[13 + outpos] = (in_array[2 + inpos] >> 1) & 31;
        out_array[14 + outpos] = (in_array[2 + inpos] >> 6) & 31;
        out_array[15 + outpos] = (in_array[2 + inpos] >> 11) & 31;
        out_array[16 + outpos] = (in_array[2 + inpos] >> 16) & 31;
        out_array[17 + outpos] = (in_array[2 + inpos] >> 21) & 31;
        out_array[18 + outpos] = (in_array[2 + inpos] >> 26) & 31;
        out_array[19 + outpos] = (in_array[2 + inpos] >> 31) | ((in_array[3 + inpos] & 15) << (5 - 4));
        out_array[20 + outpos] = (in_array[3 + inpos] >> 4) & 31;
        out_array[21 + outpos] = (in_array[3 + inpos] >> 9) & 31;
        out_array[22 + outpos] = (in_array[3 + inpos] >> 14) & 31;
        out_array[23 + outpos] = (in_array[3 + inpos] >> 19) & 31;
        out_array[24 + outpos] = (in_array[3 + inpos] >> 24) & 31;
        out_array[25 + outpos] = (in_array[3 + inpos] >> 29) | ((in_array[4 + inpos] & 3) << (5 - 2));
        out_array[26 + outpos] = (in_array[4 + inpos] >> 2) & 31;
        out_array[27 + outpos] = (in_array[4 + inpos] >> 7) & 31;
        out_array[28 + outpos] = (in_array[4 + inpos] >> 12) & 31;
        out_array[29 + outpos] = (in_array[4 + inpos] >> 17) & 31;
        out_array[30 + outpos] = (in_array[4 + inpos] >> 22) & 31;
        out_array[31 + outpos] = in_array[4 + inpos] >> 27;
    }

    pub fn fastunpack6(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 63;
        out_array[1 + outpos] = (in_array[inpos] >> 6) & 63;
        out_array[2 + outpos] = (in_array[inpos] >> 12) & 63;
        out_array[3 + outpos] = (in_array[inpos] >> 18) & 63;
        out_array[4 + outpos] = (in_array[inpos] >> 24) & 63;
        out_array[5 + outpos] = (in_array[inpos] >> 30) | ((in_array[1 + inpos] & 15) << (6 - 4));
        out_array[6 + outpos] = (in_array[1 + inpos] >> 4) & 63;
        out_array[7 + outpos] = (in_array[1 + inpos] >> 10) & 63;
        out_array[8 + outpos] = (in_array[1 + inpos] >> 16) & 63;
        out_array[9 + outpos] = (in_array[1 + inpos] >> 22) & 63;
        out_array[10 + outpos] = (in_array[1 + inpos] >> 28) | ((in_array[2 + inpos] & 3) << (6 - 2));
        out_array[11 + outpos] = (in_array[2 + inpos] >> 2) & 63;
        out_array[12 + outpos] = (in_array[2 + inpos] >> 8) & 63;
        out_array[13 + outpos] = (in_array[2 + inpos] >> 14) & 63;
        out_array[14 + outpos] = (in_array[2 + inpos] >> 20) & 63;
        out_array[15 + outpos] = in_array[2 + inpos] >> 26;
        out_array[16 + outpos] = (in_array[3 + inpos] >> 0) & 63;
        out_array[17 + outpos] = (in_array[3 + inpos] >> 6) & 63;
        out_array[18 + outpos] = (in_array[3 + inpos] >> 12) & 63;
        out_array[19 + outpos] = (in_array[3 + inpos] >> 18) & 63;
        out_array[20 + outpos] = (in_array[3 + inpos] >> 24) & 63;
        out_array[21 + outpos] = (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 15) << (6 - 4));
        out_array[22 + outpos] = (in_array[4 + inpos] >> 4) & 63;
        out_array[23 + outpos] = (in_array[4 + inpos] >> 10) & 63;
        out_array[24 + outpos] = (in_array[4 + inpos] >> 16) & 63;
        out_array[25 + outpos] = (in_array[4 + inpos] >> 22) & 63;
        out_array[26 + outpos] = (in_array[4 + inpos] >> 28) | ((in_array[5 + inpos] & 3) << (6 - 2));
        out_array[27 + outpos] = (in_array[5 + inpos] >> 2) & 63;
        out_array[28 + outpos] = (in_array[5 + inpos] >> 8) & 63;
        out_array[29 + outpos] = (in_array[5 + inpos] >> 14) & 63;
        out_array[30 + outpos] = (in_array[5 + inpos] >> 20) & 63;
        out_array[31 + outpos] = in_array[5 + inpos] >> 26;
    }

    pub fn fastunpack7(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 127;
        out_array[1 + outpos] = (in_array[inpos] >> 7) & 127;
        out_array[2 + outpos] = (in_array[inpos] >> 14) & 127;
        out_array[3 + outpos] = (in_array[inpos] >> 21) & 127;
        out_array[4 + outpos] = (in_array[inpos] >> 28) | ((in_array[1 + inpos] & 7) << (7 - 3));
        out_array[5 + outpos] = (in_array[1 + inpos] >> 3) & 127;
        out_array[6 + outpos] = (in_array[1 + inpos] >> 10) & 127;
        out_array[7 + outpos] = (in_array[1 + inpos] >> 17) & 127;
        out_array[8 + outpos] = (in_array[1 + inpos] >> 24) & 127;
        out_array[9 + outpos] = (in_array[1 + inpos] >> 31) | ((in_array[2 + inpos] & 63) << (7 - 6));
        out_array[10 + outpos] = (in_array[2 + inpos] >> 6) & 127;
        out_array[11 + outpos] = (in_array[2 + inpos] >> 13) & 127;
        out_array[12 + outpos] = (in_array[2 + inpos] >> 20) & 127;
        out_array[13 + outpos] = (in_array[2 + inpos] >> 27) | ((in_array[3 + inpos] & 3) << (7 - 2));
        out_array[14 + outpos] = (in_array[3 + inpos] >> 2) & 127;
        out_array[15 + outpos] = (in_array[3 + inpos] >> 9) & 127;
        out_array[16 + outpos] = (in_array[3 + inpos] >> 16) & 127;
        out_array[17 + outpos] = (in_array[3 + inpos] >> 23) & 127;
        out_array[18 + outpos] = (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 31) << (7 - 5));
        out_array[19 + outpos] = (in_array[4 + inpos] >> 5) & 127;
        out_array[20 + outpos] = (in_array[4 + inpos] >> 12) & 127;
        out_array[21 + outpos] = (in_array[4 + inpos] >> 19) & 127;
        out_array[22 + outpos] = (in_array[4 + inpos] >> 26) | ((in_array[5 + inpos] & 1) << (7 - 1));
        out_array[23 + outpos] = (in_array[5 + inpos] >> 1) & 127;
        out_array[24 + outpos] = (in_array[5 + inpos] >> 8) & 127;
        out_array[25 + outpos] = (in_array[5 + inpos] >> 15) & 127;
        out_array[26 + outpos] = (in_array[5 + inpos] >> 22) & 127;
        out_array[27 + outpos] = (in_array[5 + inpos] >> 29) | ((in_array[6 + inpos] & 15) << (7 - 4));
        out_array[28 + outpos] = (in_array[6 + inpos] >> 4) & 127;
        out_array[29 + outpos] = (in_array[6 + inpos] >> 11) & 127;
        out_array[30 + outpos] = (in_array[6 + inpos] >> 18) & 127;
        out_array[31 + outpos] = in_array[6 + inpos] >> 25;
    }

    pub fn fastunpack8(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 255;
        out_array[1 + outpos] = (in_array[inpos] >> 8) & 255;
        out_array[2 + outpos] = (in_array[inpos] >> 16) & 255;
        out_array[3 + outpos] = in_array[inpos] >> 24;
        out_array[4 + outpos] = (in_array[1 + inpos] >> 0) & 255;
        out_array[5 + outpos] = (in_array[1 + inpos] >> 8) & 255;
        out_array[6 + outpos] = (in_array[1 + inpos] >> 16) & 255;
        out_array[7 + outpos] = in_array[1 + inpos] >> 24;
        out_array[8 + outpos] = (in_array[2 + inpos] >> 0) & 255;
        out_array[9 + outpos] = (in_array[2 + inpos] >> 8) & 255;
        out_array[10 + outpos] = (in_array[2 + inpos] >> 16) & 255;
        out_array[11 + outpos] = in_array[2 + inpos] >> 24;
        out_array[12 + outpos] = (in_array[3 + inpos] >> 0) & 255;
        out_array[13 + outpos] = (in_array[3 + inpos] >> 8) & 255;
        out_array[14 + outpos] = (in_array[3 + inpos] >> 16) & 255;
        out_array[15 + outpos] = in_array[3 + inpos] >> 24;
        out_array[16 + outpos] = (in_array[4 + inpos] >> 0) & 255;
        out_array[17 + outpos] = (in_array[4 + inpos] >> 8) & 255;
        out_array[18 + outpos] = (in_array[4 + inpos] >> 16) & 255;
        out_array[19 + outpos] = in_array[4 + inpos] >> 24;
        out_array[20 + outpos] = (in_array[5 + inpos] >> 0) & 255;
        out_array[21 + outpos] = (in_array[5 + inpos] >> 8) & 255;
        out_array[22 + outpos] = (in_array[5 + inpos] >> 16) & 255;
        out_array[23 + outpos] = in_array[5 + inpos] >> 24;
        out_array[24 + outpos] = (in_array[6 + inpos] >> 0) & 255;
        out_array[25 + outpos] = (in_array[6 + inpos] >> 8) & 255;
        out_array[26 + outpos] = (in_array[6 + inpos] >> 16) & 255;
        out_array[27 + outpos] = in_array[6 + inpos] >> 24;
        out_array[28 + outpos] = (in_array[7 + inpos] >> 0) & 255;
        out_array[29 + outpos] = (in_array[7 + inpos] >> 8) & 255;
        out_array[30 + outpos] = (in_array[7 + inpos] >> 16) & 255;
        out_array[31 + outpos] = in_array[7 + inpos] >> 24;
    }

    pub fn fastunpack9(in_array: &[u32], inpos: usize, out_array: &mut [u32], outpos: usize) {
        out_array[outpos] = (in_array[inpos] >> 0) & 511;
        out_array[1 + outpos] = (in_array[inpos] >> 9) & 511;
        out_array[2 + outpos] = (in_array[inpos] >> 18) & 511;
        out_array[3 + outpos] = (in_array[inpos] >> 27) | ((in_array[1 + inpos] & 15) << (9 - 4));
        out_array[4 + outpos] = (in_array[1 + inpos] >> 4) & 511;
        out_array[5 + outpos] = (in_array[1 + inpos] >> 13) & 511;
        out_array[6 + outpos] = (in_array[1 + inpos] >> 22) & 511;
        out_array[7 + outpos] = (in_array[1 + inpos] >> 31) | ((in_array[2 + inpos] & 255) << (9 - 8));
        out_array[8 + outpos] = (in_array[2 + inpos] >> 8) & 511;
        out_array[9 + outpos] = (in_array[2 + inpos] >> 17) & 511;
        out_array[10 + outpos] = (in_array[2 + inpos] >> 26) | ((in_array[3 + inpos] & 7) << (9 - 3));
        out_array[11 + outpos] = (in_array[3 + inpos] >> 3) & 511;
        out_array[12 + outpos] = (in_array[3 + inpos] >> 12) & 511;
        out_array[13 + outpos] = (in_array[3 + inpos] >> 21) & 511;
        out_array[14 + outpos] = (in_array[3 + inpos] >> 30) | ((in_array[4 + inpos] & 127) << (9 - 7));
        out_array[15 + outpos] = (in_array[4 + inpos] >> 7) & 511;
        out_array[16 + outpos] = (in_array[4 + inpos] >> 16) & 511;
        out_array[17 + outpos] = (in_array[4 + inpos] >> 25) | ((in_array[5 + inpos] & 3) << (9 - 2));
        out_array[18 + outpos] = (in_array[5 + inpos] >> 2) & 511;
        out_array[19 + outpos] = (in_array[5 + inpos] >> 11) & 511;
        out_array[20 + outpos] = (in_array[5 + inpos] >> 20) & 511;
        out_array[21 + outpos] = (in_array[5 + inpos] >> 29) | ((in_array[6 + inpos] & 63) << (9 - 6));
        out_array[22 + outpos] = (in_array[6 + inpos] >> 6) & 511;
        out_array[23 + outpos] = (in_array[6 + inpos] >> 15) & 511;
        out_array[24 + outpos] = (in_array[6 + inpos] >> 24) | ((in_array[7 + inpos] & 1) << (9 - 1));
        out_array[25 + outpos] = (in_array[7 + inpos] >> 1) & 511;
        out_array[26 + outpos] = (in_array[7 + inpos] >> 10) & 511;
        out_array[27 + outpos] = (in_array[7 + inpos] >> 19) & 511;
        out_array[28 + outpos] = (in_array[7 + inpos] >> 28) | ((in_array[8 + inpos] & 31) << (9 - 5));
        out_array[29 + outpos] = (in_array[8 + inpos] >> 5) & 511;
        out_array[30 + outpos] = (in_array[8 + inpos] >> 14) & 511;
        out_array[31 + outpos] = in_array[8 + inpos] >> 23;
    }
}
