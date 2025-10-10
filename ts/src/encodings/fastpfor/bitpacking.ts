import { arraycopy } from "./util";

/**
 * Pack 32 numberegers
 *
 * @param input
 *                source array
 * @param inpos
 *                position in source array
 * @param output
 *                output array
 * @param outpos
 *                position in output array
 * @param bit
 *                number of bits to use per numbereger
 */
export function fastpack(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number, bit: number) {
    switch (bit) {
        case 0:
            fastpack0(input, inpos, output, outpos);
            break;
        case 1:
            fastpack1(input, inpos, output, outpos);
            break;
        case 2:
            fastpack2(input, inpos, output, outpos);
            break;
        case 3:
            fastpack3(input, inpos, output, outpos);
            break;
        case 4:
            fastpack4(input, inpos, output, outpos);
            break;
        case 5:
            fastpack5(input, inpos, output, outpos);
            break;
        case 6:
            fastpack6(input, inpos, output, outpos);
            break;
        case 7:
            fastpack7(input, inpos, output, outpos);
            break;
        case 8:
            fastpack8(input, inpos, output, outpos);
            break;
        case 9:
            fastpack9(input, inpos, output, outpos);
            break;
        case 10:
            fastpack10(input, inpos, output, outpos);
            break;
        case 11:
            fastpack11(input, inpos, output, outpos);
            break;
        case 12:
            fastpack12(input, inpos, output, outpos);
            break;
        case 13:
            fastpack13(input, inpos, output, outpos);
            break;
        case 14:
            fastpack14(input, inpos, output, outpos);
            break;
        case 15:
            fastpack15(input, inpos, output, outpos);
            break;
        case 16:
            fastpack16(input, inpos, output, outpos);
            break;
        case 17:
            fastpack17(input, inpos, output, outpos);
            break;
        case 18:
            fastpack18(input, inpos, output, outpos);
            break;
        case 19:
            fastpack19(input, inpos, output, outpos);
            break;
        case 20:
            fastpack20(input, inpos, output, outpos);
            break;
        case 21:
            fastpack21(input, inpos, output, outpos);
            break;
        case 22:
            fastpack22(input, inpos, output, outpos);
            break;
        case 23:
            fastpack23(input, inpos, output, outpos);
            break;
        case 24:
            fastpack24(input, inpos, output, outpos);
            break;
        case 25:
            fastpack25(input, inpos, output, outpos);
            break;
        case 26:
            fastpack26(input, inpos, output, outpos);
            break;
        case 27:
            fastpack27(input, inpos, output, outpos);
            break;
        case 28:
            fastpack28(input, inpos, output, outpos);
            break;
        case 29:
            fastpack29(input, inpos, output, outpos);
            break;
        case 30:
            fastpack30(input, inpos, output, outpos);
            break;
        case 31:
            fastpack31(input, inpos, output, outpos);
            break;
        case 32:
            fastpack32(input, inpos, output, outpos);
            break;
        default:
            throw new Error("Unsupported bit width.");
    }
}

function fastpack0(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    // nothing
}

function fastpack1(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 1) |
        ((input[1 + inpos] & 1) << 1) |
        ((input[2 + inpos] & 1) << 2) |
        ((input[3 + inpos] & 1) << 3) |
        ((input[4 + inpos] & 1) << 4) |
        ((input[5 + inpos] & 1) << 5) |
        ((input[6 + inpos] & 1) << 6) |
        ((input[7 + inpos] & 1) << 7) |
        ((input[8 + inpos] & 1) << 8) |
        ((input[9 + inpos] & 1) << 9) |
        ((input[10 + inpos] & 1) << 10) |
        ((input[11 + inpos] & 1) << 11) |
        ((input[12 + inpos] & 1) << 12) |
        ((input[13 + inpos] & 1) << 13) |
        ((input[14 + inpos] & 1) << 14) |
        ((input[15 + inpos] & 1) << 15) |
        ((input[16 + inpos] & 1) << 16) |
        ((input[17 + inpos] & 1) << 17) |
        ((input[18 + inpos] & 1) << 18) |
        ((input[19 + inpos] & 1) << 19) |
        ((input[20 + inpos] & 1) << 20) |
        ((input[21 + inpos] & 1) << 21) |
        ((input[22 + inpos] & 1) << 22) |
        ((input[23 + inpos] & 1) << 23) |
        ((input[24 + inpos] & 1) << 24) |
        ((input[25 + inpos] & 1) << 25) |
        ((input[26 + inpos] & 1) << 26) |
        ((input[27 + inpos] & 1) << 27) |
        ((input[28 + inpos] & 1) << 28) |
        ((input[29 + inpos] & 1) << 29) |
        ((input[30 + inpos] & 1) << 30) |
        (input[31 + inpos] << 31);
}

function fastpack10(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 1023) |
        ((input[1 + inpos] & 1023) << 10) |
        ((input[2 + inpos] & 1023) << 20) |
        (input[3 + inpos] << 30);
    output[1 + outpos] =
        ((input[3 + inpos] & 1023) >>> (10 - 8)) |
        ((input[4 + inpos] & 1023) << 8) |
        ((input[5 + inpos] & 1023) << 18) |
        (input[6 + inpos] << 28);
    output[2 + outpos] =
        ((input[6 + inpos] & 1023) >>> (10 - 6)) |
        ((input[7 + inpos] & 1023) << 6) |
        ((input[8 + inpos] & 1023) << 16) |
        (input[9 + inpos] << 26);
    output[3 + outpos] =
        ((input[9 + inpos] & 1023) >>> (10 - 4)) |
        ((input[10 + inpos] & 1023) << 4) |
        ((input[11 + inpos] & 1023) << 14) |
        (input[12 + inpos] << 24);
    output[4 + outpos] =
        ((input[12 + inpos] & 1023) >>> (10 - 2)) |
        ((input[13 + inpos] & 1023) << 2) |
        ((input[14 + inpos] & 1023) << 12) |
        (input[15 + inpos] << 22);
    output[5 + outpos] =
        (input[16 + inpos] & 1023) |
        ((input[17 + inpos] & 1023) << 10) |
        ((input[18 + inpos] & 1023) << 20) |
        (input[19 + inpos] << 30);
    output[6 + outpos] =
        ((input[19 + inpos] & 1023) >>> (10 - 8)) |
        ((input[20 + inpos] & 1023) << 8) |
        ((input[21 + inpos] & 1023) << 18) |
        (input[22 + inpos] << 28);
    output[7 + outpos] =
        ((input[22 + inpos] & 1023) >>> (10 - 6)) |
        ((input[23 + inpos] & 1023) << 6) |
        ((input[24 + inpos] & 1023) << 16) |
        (input[25 + inpos] << 26);
    output[8 + outpos] =
        ((input[25 + inpos] & 1023) >>> (10 - 4)) |
        ((input[26 + inpos] & 1023) << 4) |
        ((input[27 + inpos] & 1023) << 14) |
        (input[28 + inpos] << 24);
    output[9 + outpos] =
        ((input[28 + inpos] & 1023) >>> (10 - 2)) |
        ((input[29 + inpos] & 1023) << 2) |
        ((input[30 + inpos] & 1023) << 12) |
        (input[31 + inpos] << 22);
}

function fastpack11(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 2047) | ((input[1 + inpos] & 2047) << 11) | (input[2 + inpos] << 22);
    output[1 + outpos] =
        ((input[2 + inpos] & 2047) >>> (11 - 1)) |
        ((input[3 + inpos] & 2047) << 1) |
        ((input[4 + inpos] & 2047) << 12) |
        (input[5 + inpos] << 23);
    output[2 + outpos] =
        ((input[5 + inpos] & 2047) >>> (11 - 2)) |
        ((input[6 + inpos] & 2047) << 2) |
        ((input[7 + inpos] & 2047) << 13) |
        (input[8 + inpos] << 24);
    output[3 + outpos] =
        ((input[8 + inpos] & 2047) >>> (11 - 3)) |
        ((input[9 + inpos] & 2047) << 3) |
        ((input[10 + inpos] & 2047) << 14) |
        (input[11 + inpos] << 25);
    output[4 + outpos] =
        ((input[11 + inpos] & 2047) >>> (11 - 4)) |
        ((input[12 + inpos] & 2047) << 4) |
        ((input[13 + inpos] & 2047) << 15) |
        (input[14 + inpos] << 26);
    output[5 + outpos] =
        ((input[14 + inpos] & 2047) >>> (11 - 5)) |
        ((input[15 + inpos] & 2047) << 5) |
        ((input[16 + inpos] & 2047) << 16) |
        (input[17 + inpos] << 27);
    output[6 + outpos] =
        ((input[17 + inpos] & 2047) >>> (11 - 6)) |
        ((input[18 + inpos] & 2047) << 6) |
        ((input[19 + inpos] & 2047) << 17) |
        (input[20 + inpos] << 28);
    output[7 + outpos] =
        ((input[20 + inpos] & 2047) >>> (11 - 7)) |
        ((input[21 + inpos] & 2047) << 7) |
        ((input[22 + inpos] & 2047) << 18) |
        (input[23 + inpos] << 29);
    output[8 + outpos] =
        ((input[23 + inpos] & 2047) >>> (11 - 8)) |
        ((input[24 + inpos] & 2047) << 8) |
        ((input[25 + inpos] & 2047) << 19) |
        (input[26 + inpos] << 30);
    output[9 + outpos] =
        ((input[26 + inpos] & 2047) >>> (11 - 9)) |
        ((input[27 + inpos] & 2047) << 9) |
        ((input[28 + inpos] & 2047) << 20) |
        (input[29 + inpos] << 31);
    output[10 + outpos] =
        ((input[29 + inpos] & 2047) >>> (11 - 10)) | ((input[30 + inpos] & 2047) << 10) | (input[31 + inpos] << 21);
}

function fastpack12(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 4095) | ((input[1 + inpos] & 4095) << 12) | (input[2 + inpos] << 24);
    output[1 + outpos] =
        ((input[2 + inpos] & 4095) >>> (12 - 4)) |
        ((input[3 + inpos] & 4095) << 4) |
        ((input[4 + inpos] & 4095) << 16) |
        (input[5 + inpos] << 28);
    output[2 + outpos] =
        ((input[5 + inpos] & 4095) >>> (12 - 8)) | ((input[6 + inpos] & 4095) << 8) | (input[7 + inpos] << 20);
    output[3 + outpos] = (input[8 + inpos] & 4095) | ((input[9 + inpos] & 4095) << 12) | (input[10 + inpos] << 24);
    output[4 + outpos] =
        ((input[10 + inpos] & 4095) >>> (12 - 4)) |
        ((input[11 + inpos] & 4095) << 4) |
        ((input[12 + inpos] & 4095) << 16) |
        (input[13 + inpos] << 28);
    output[5 + outpos] =
        ((input[13 + inpos] & 4095) >>> (12 - 8)) | ((input[14 + inpos] & 4095) << 8) | (input[15 + inpos] << 20);
    output[6 + outpos] = (input[16 + inpos] & 4095) | ((input[17 + inpos] & 4095) << 12) | (input[18 + inpos] << 24);
    output[7 + outpos] =
        ((input[18 + inpos] & 4095) >>> (12 - 4)) |
        ((input[19 + inpos] & 4095) << 4) |
        ((input[20 + inpos] & 4095) << 16) |
        (input[21 + inpos] << 28);
    output[8 + outpos] =
        ((input[21 + inpos] & 4095) >>> (12 - 8)) | ((input[22 + inpos] & 4095) << 8) | (input[23 + inpos] << 20);
    output[9 + outpos] = (input[24 + inpos] & 4095) | ((input[25 + inpos] & 4095) << 12) | (input[26 + inpos] << 24);
    output[10 + outpos] =
        ((input[26 + inpos] & 4095) >>> (12 - 4)) |
        ((input[27 + inpos] & 4095) << 4) |
        ((input[28 + inpos] & 4095) << 16) |
        (input[29 + inpos] << 28);
    output[11 + outpos] =
        ((input[29 + inpos] & 4095) >>> (12 - 8)) | ((input[30 + inpos] & 4095) << 8) | (input[31 + inpos] << 20);
}

function fastpack13(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 8191) | ((input[1 + inpos] & 8191) << 13) | (input[2 + inpos] << 26);
    output[1 + outpos] =
        ((input[2 + inpos] & 8191) >>> (13 - 7)) | ((input[3 + inpos] & 8191) << 7) | (input[4 + inpos] << 20);
    output[2 + outpos] =
        ((input[4 + inpos] & 8191) >>> (13 - 1)) |
        ((input[5 + inpos] & 8191) << 1) |
        ((input[6 + inpos] & 8191) << 14) |
        (input[7 + inpos] << 27);
    output[3 + outpos] =
        ((input[7 + inpos] & 8191) >>> (13 - 8)) | ((input[8 + inpos] & 8191) << 8) | (input[9 + inpos] << 21);
    output[4 + outpos] =
        ((input[9 + inpos] & 8191) >>> (13 - 2)) |
        ((input[10 + inpos] & 8191) << 2) |
        ((input[11 + inpos] & 8191) << 15) |
        (input[12 + inpos] << 28);
    output[5 + outpos] =
        ((input[12 + inpos] & 8191) >>> (13 - 9)) | ((input[13 + inpos] & 8191) << 9) | (input[14 + inpos] << 22);
    output[6 + outpos] =
        ((input[14 + inpos] & 8191) >>> (13 - 3)) |
        ((input[15 + inpos] & 8191) << 3) |
        ((input[16 + inpos] & 8191) << 16) |
        (input[17 + inpos] << 29);
    output[7 + outpos] =
        ((input[17 + inpos] & 8191) >>> (13 - 10)) | ((input[18 + inpos] & 8191) << 10) | (input[19 + inpos] << 23);
    output[8 + outpos] =
        ((input[19 + inpos] & 8191) >>> (13 - 4)) |
        ((input[20 + inpos] & 8191) << 4) |
        ((input[21 + inpos] & 8191) << 17) |
        (input[22 + inpos] << 30);
    output[9 + outpos] =
        ((input[22 + inpos] & 8191) >>> (13 - 11)) | ((input[23 + inpos] & 8191) << 11) | (input[24 + inpos] << 24);
    output[10 + outpos] =
        ((input[24 + inpos] & 8191) >>> (13 - 5)) |
        ((input[25 + inpos] & 8191) << 5) |
        ((input[26 + inpos] & 8191) << 18) |
        (input[27 + inpos] << 31);
    output[11 + outpos] =
        ((input[27 + inpos] & 8191) >>> (13 - 12)) | ((input[28 + inpos] & 8191) << 12) | (input[29 + inpos] << 25);
    output[12 + outpos] =
        ((input[29 + inpos] & 8191) >>> (13 - 6)) | ((input[30 + inpos] & 8191) << 6) | (input[31 + inpos] << 19);
}

function fastpack14(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 16383) | ((input[1 + inpos] & 16383) << 14) | (input[2 + inpos] << 28);
    output[1 + outpos] =
        ((input[2 + inpos] & 16383) >>> (14 - 10)) | ((input[3 + inpos] & 16383) << 10) | (input[4 + inpos] << 24);
    output[2 + outpos] =
        ((input[4 + inpos] & 16383) >>> (14 - 6)) | ((input[5 + inpos] & 16383) << 6) | (input[6 + inpos] << 20);
    output[3 + outpos] =
        ((input[6 + inpos] & 16383) >>> (14 - 2)) |
        ((input[7 + inpos] & 16383) << 2) |
        ((input[8 + inpos] & 16383) << 16) |
        (input[9 + inpos] << 30);
    output[4 + outpos] =
        ((input[9 + inpos] & 16383) >>> (14 - 12)) | ((input[10 + inpos] & 16383) << 12) | (input[11 + inpos] << 26);
    output[5 + outpos] =
        ((input[11 + inpos] & 16383) >>> (14 - 8)) | ((input[12 + inpos] & 16383) << 8) | (input[13 + inpos] << 22);
    output[6 + outpos] =
        ((input[13 + inpos] & 16383) >>> (14 - 4)) | ((input[14 + inpos] & 16383) << 4) | (input[15 + inpos] << 18);
    output[7 + outpos] = (input[16 + inpos] & 16383) | ((input[17 + inpos] & 16383) << 14) | (input[18 + inpos] << 28);
    output[8 + outpos] =
        ((input[18 + inpos] & 16383) >>> (14 - 10)) | ((input[19 + inpos] & 16383) << 10) | (input[20 + inpos] << 24);
    output[9 + outpos] =
        ((input[20 + inpos] & 16383) >>> (14 - 6)) | ((input[21 + inpos] & 16383) << 6) | (input[22 + inpos] << 20);
    output[10 + outpos] =
        ((input[22 + inpos] & 16383) >>> (14 - 2)) |
        ((input[23 + inpos] & 16383) << 2) |
        ((input[24 + inpos] & 16383) << 16) |
        (input[25 + inpos] << 30);
    output[11 + outpos] =
        ((input[25 + inpos] & 16383) >>> (14 - 12)) | ((input[26 + inpos] & 16383) << 12) | (input[27 + inpos] << 26);
    output[12 + outpos] =
        ((input[27 + inpos] & 16383) >>> (14 - 8)) | ((input[28 + inpos] & 16383) << 8) | (input[29 + inpos] << 22);
    output[13 + outpos] =
        ((input[29 + inpos] & 16383) >>> (14 - 4)) | ((input[30 + inpos] & 16383) << 4) | (input[31 + inpos] << 18);
}

function fastpack15(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 32767) | ((input[1 + inpos] & 32767) << 15) | (input[2 + inpos] << 30);
    output[1 + outpos] =
        ((input[2 + inpos] & 32767) >>> (15 - 13)) | ((input[3 + inpos] & 32767) << 13) | (input[4 + inpos] << 28);
    output[2 + outpos] =
        ((input[4 + inpos] & 32767) >>> (15 - 11)) | ((input[5 + inpos] & 32767) << 11) | (input[6 + inpos] << 26);
    output[3 + outpos] =
        ((input[6 + inpos] & 32767) >>> (15 - 9)) | ((input[7 + inpos] & 32767) << 9) | (input[8 + inpos] << 24);
    output[4 + outpos] =
        ((input[8 + inpos] & 32767) >>> (15 - 7)) | ((input[9 + inpos] & 32767) << 7) | (input[10 + inpos] << 22);
    output[5 + outpos] =
        ((input[10 + inpos] & 32767) >>> (15 - 5)) | ((input[11 + inpos] & 32767) << 5) | (input[12 + inpos] << 20);
    output[6 + outpos] =
        ((input[12 + inpos] & 32767) >>> (15 - 3)) | ((input[13 + inpos] & 32767) << 3) | (input[14 + inpos] << 18);
    output[7 + outpos] =
        ((input[14 + inpos] & 32767) >>> (15 - 1)) |
        ((input[15 + inpos] & 32767) << 1) |
        ((input[16 + inpos] & 32767) << 16) |
        (input[17 + inpos] << 31);
    output[8 + outpos] =
        ((input[17 + inpos] & 32767) >>> (15 - 14)) | ((input[18 + inpos] & 32767) << 14) | (input[19 + inpos] << 29);
    output[9 + outpos] =
        ((input[19 + inpos] & 32767) >>> (15 - 12)) | ((input[20 + inpos] & 32767) << 12) | (input[21 + inpos] << 27);
    output[10 + outpos] =
        ((input[21 + inpos] & 32767) >>> (15 - 10)) | ((input[22 + inpos] & 32767) << 10) | (input[23 + inpos] << 25);
    output[11 + outpos] =
        ((input[23 + inpos] & 32767) >>> (15 - 8)) | ((input[24 + inpos] & 32767) << 8) | (input[25 + inpos] << 23);
    output[12 + outpos] =
        ((input[25 + inpos] & 32767) >>> (15 - 6)) | ((input[26 + inpos] & 32767) << 6) | (input[27 + inpos] << 21);
    output[13 + outpos] =
        ((input[27 + inpos] & 32767) >>> (15 - 4)) | ((input[28 + inpos] & 32767) << 4) | (input[29 + inpos] << 19);
    output[14 + outpos] =
        ((input[29 + inpos] & 32767) >>> (15 - 2)) | ((input[30 + inpos] & 32767) << 2) | (input[31 + inpos] << 17);
}

function fastpack16(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 65535) | (input[1 + inpos] << 16);
    output[1 + outpos] = (input[2 + inpos] & 65535) | (input[3 + inpos] << 16);
    output[2 + outpos] = (input[4 + inpos] & 65535) | (input[5 + inpos] << 16);
    output[3 + outpos] = (input[6 + inpos] & 65535) | (input[7 + inpos] << 16);
    output[4 + outpos] = (input[8 + inpos] & 65535) | (input[9 + inpos] << 16);
    output[5 + outpos] = (input[10 + inpos] & 65535) | (input[11 + inpos] << 16);
    output[6 + outpos] = (input[12 + inpos] & 65535) | (input[13 + inpos] << 16);
    output[7 + outpos] = (input[14 + inpos] & 65535) | (input[15 + inpos] << 16);
    output[8 + outpos] = (input[16 + inpos] & 65535) | (input[17 + inpos] << 16);
    output[9 + outpos] = (input[18 + inpos] & 65535) | (input[19 + inpos] << 16);
    output[10 + outpos] = (input[20 + inpos] & 65535) | (input[21 + inpos] << 16);
    output[11 + outpos] = (input[22 + inpos] & 65535) | (input[23 + inpos] << 16);
    output[12 + outpos] = (input[24 + inpos] & 65535) | (input[25 + inpos] << 16);
    output[13 + outpos] = (input[26 + inpos] & 65535) | (input[27 + inpos] << 16);
    output[14 + outpos] = (input[28 + inpos] & 65535) | (input[29 + inpos] << 16);
    output[15 + outpos] = (input[30 + inpos] & 65535) | (input[31 + inpos] << 16);
}

function fastpack17(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 131071) | (input[1 + inpos] << 17);
    output[1 + outpos] =
        ((input[1 + inpos] & 131071) >>> (17 - 2)) | ((input[2 + inpos] & 131071) << 2) | (input[3 + inpos] << 19);
    output[2 + outpos] =
        ((input[3 + inpos] & 131071) >>> (17 - 4)) | ((input[4 + inpos] & 131071) << 4) | (input[5 + inpos] << 21);
    output[3 + outpos] =
        ((input[5 + inpos] & 131071) >>> (17 - 6)) | ((input[6 + inpos] & 131071) << 6) | (input[7 + inpos] << 23);
    output[4 + outpos] =
        ((input[7 + inpos] & 131071) >>> (17 - 8)) | ((input[8 + inpos] & 131071) << 8) | (input[9 + inpos] << 25);
    output[5 + outpos] =
        ((input[9 + inpos] & 131071) >>> (17 - 10)) | ((input[10 + inpos] & 131071) << 10) | (input[11 + inpos] << 27);
    output[6 + outpos] =
        ((input[11 + inpos] & 131071) >>> (17 - 12)) | ((input[12 + inpos] & 131071) << 12) | (input[13 + inpos] << 29);
    output[7 + outpos] =
        ((input[13 + inpos] & 131071) >>> (17 - 14)) | ((input[14 + inpos] & 131071) << 14) | (input[15 + inpos] << 31);
    output[8 + outpos] = ((input[15 + inpos] & 131071) >>> (17 - 16)) | (input[16 + inpos] << 16);
    output[9 + outpos] =
        ((input[16 + inpos] & 131071) >>> (17 - 1)) | ((input[17 + inpos] & 131071) << 1) | (input[18 + inpos] << 18);
    output[10 + outpos] =
        ((input[18 + inpos] & 131071) >>> (17 - 3)) | ((input[19 + inpos] & 131071) << 3) | (input[20 + inpos] << 20);
    output[11 + outpos] =
        ((input[20 + inpos] & 131071) >>> (17 - 5)) | ((input[21 + inpos] & 131071) << 5) | (input[22 + inpos] << 22);
    output[12 + outpos] =
        ((input[22 + inpos] & 131071) >>> (17 - 7)) | ((input[23 + inpos] & 131071) << 7) | (input[24 + inpos] << 24);
    output[13 + outpos] =
        ((input[24 + inpos] & 131071) >>> (17 - 9)) | ((input[25 + inpos] & 131071) << 9) | (input[26 + inpos] << 26);
    output[14 + outpos] =
        ((input[26 + inpos] & 131071) >>> (17 - 11)) | ((input[27 + inpos] & 131071) << 11) | (input[28 + inpos] << 28);
    output[15 + outpos] =
        ((input[28 + inpos] & 131071) >>> (17 - 13)) | ((input[29 + inpos] & 131071) << 13) | (input[30 + inpos] << 30);
    output[16 + outpos] = ((input[30 + inpos] & 131071) >>> (17 - 15)) | (input[31 + inpos] << 15);
}

function fastpack18(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 262143) | (input[1 + inpos] << 18);
    output[1 + outpos] =
        ((input[1 + inpos] & 262143) >>> (18 - 4)) | ((input[2 + inpos] & 262143) << 4) | (input[3 + inpos] << 22);
    output[2 + outpos] =
        ((input[3 + inpos] & 262143) >>> (18 - 8)) | ((input[4 + inpos] & 262143) << 8) | (input[5 + inpos] << 26);
    output[3 + outpos] =
        ((input[5 + inpos] & 262143) >>> (18 - 12)) | ((input[6 + inpos] & 262143) << 12) | (input[7 + inpos] << 30);
    output[4 + outpos] = ((input[7 + inpos] & 262143) >>> (18 - 16)) | (input[8 + inpos] << 16);
    output[5 + outpos] =
        ((input[8 + inpos] & 262143) >>> (18 - 2)) | ((input[9 + inpos] & 262143) << 2) | (input[10 + inpos] << 20);
    output[6 + outpos] =
        ((input[10 + inpos] & 262143) >>> (18 - 6)) | ((input[11 + inpos] & 262143) << 6) | (input[12 + inpos] << 24);
    output[7 + outpos] =
        ((input[12 + inpos] & 262143) >>> (18 - 10)) | ((input[13 + inpos] & 262143) << 10) | (input[14 + inpos] << 28);
    output[8 + outpos] = ((input[14 + inpos] & 262143) >>> (18 - 14)) | (input[15 + inpos] << 14);
    output[9 + outpos] = (input[16 + inpos] & 262143) | (input[17 + inpos] << 18);
    output[10 + outpos] =
        ((input[17 + inpos] & 262143) >>> (18 - 4)) | ((input[18 + inpos] & 262143) << 4) | (input[19 + inpos] << 22);
    output[11 + outpos] =
        ((input[19 + inpos] & 262143) >>> (18 - 8)) | ((input[20 + inpos] & 262143) << 8) | (input[21 + inpos] << 26);
    output[12 + outpos] =
        ((input[21 + inpos] & 262143) >>> (18 - 12)) | ((input[22 + inpos] & 262143) << 12) | (input[23 + inpos] << 30);
    output[13 + outpos] = ((input[23 + inpos] & 262143) >>> (18 - 16)) | (input[24 + inpos] << 16);
    output[14 + outpos] =
        ((input[24 + inpos] & 262143) >>> (18 - 2)) | ((input[25 + inpos] & 262143) << 2) | (input[26 + inpos] << 20);
    output[15 + outpos] =
        ((input[26 + inpos] & 262143) >>> (18 - 6)) | ((input[27 + inpos] & 262143) << 6) | (input[28 + inpos] << 24);
    output[16 + outpos] =
        ((input[28 + inpos] & 262143) >>> (18 - 10)) | ((input[29 + inpos] & 262143) << 10) | (input[30 + inpos] << 28);
    output[17 + outpos] = ((input[30 + inpos] & 262143) >>> (18 - 14)) | (input[31 + inpos] << 14);
}

function fastpack19(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 524287) | (input[1 + inpos] << 19);
    output[1 + outpos] =
        ((input[1 + inpos] & 524287) >>> (19 - 6)) | ((input[2 + inpos] & 524287) << 6) | (input[3 + inpos] << 25);
    output[2 + outpos] =
        ((input[3 + inpos] & 524287) >>> (19 - 12)) | ((input[4 + inpos] & 524287) << 12) | (input[5 + inpos] << 31);
    output[3 + outpos] = ((input[5 + inpos] & 524287) >>> (19 - 18)) | (input[6 + inpos] << 18);
    output[4 + outpos] =
        ((input[6 + inpos] & 524287) >>> (19 - 5)) | ((input[7 + inpos] & 524287) << 5) | (input[8 + inpos] << 24);
    output[5 + outpos] =
        ((input[8 + inpos] & 524287) >>> (19 - 11)) | ((input[9 + inpos] & 524287) << 11) | (input[10 + inpos] << 30);
    output[6 + outpos] = ((input[10 + inpos] & 524287) >>> (19 - 17)) | (input[11 + inpos] << 17);
    output[7 + outpos] =
        ((input[11 + inpos] & 524287) >>> (19 - 4)) | ((input[12 + inpos] & 524287) << 4) | (input[13 + inpos] << 23);
    output[8 + outpos] =
        ((input[13 + inpos] & 524287) >>> (19 - 10)) | ((input[14 + inpos] & 524287) << 10) | (input[15 + inpos] << 29);
    output[9 + outpos] = ((input[15 + inpos] & 524287) >>> (19 - 16)) | (input[16 + inpos] << 16);
    output[10 + outpos] =
        ((input[16 + inpos] & 524287) >>> (19 - 3)) | ((input[17 + inpos] & 524287) << 3) | (input[18 + inpos] << 22);
    output[11 + outpos] =
        ((input[18 + inpos] & 524287) >>> (19 - 9)) | ((input[19 + inpos] & 524287) << 9) | (input[20 + inpos] << 28);
    output[12 + outpos] = ((input[20 + inpos] & 524287) >>> (19 - 15)) | (input[21 + inpos] << 15);
    output[13 + outpos] =
        ((input[21 + inpos] & 524287) >>> (19 - 2)) | ((input[22 + inpos] & 524287) << 2) | (input[23 + inpos] << 21);
    output[14 + outpos] =
        ((input[23 + inpos] & 524287) >>> (19 - 8)) | ((input[24 + inpos] & 524287) << 8) | (input[25 + inpos] << 27);
    output[15 + outpos] = ((input[25 + inpos] & 524287) >>> (19 - 14)) | (input[26 + inpos] << 14);
    output[16 + outpos] =
        ((input[26 + inpos] & 524287) >>> (19 - 1)) | ((input[27 + inpos] & 524287) << 1) | (input[28 + inpos] << 20);
    output[17 + outpos] =
        ((input[28 + inpos] & 524287) >>> (19 - 7)) | ((input[29 + inpos] & 524287) << 7) | (input[30 + inpos] << 26);
    output[18 + outpos] = ((input[30 + inpos] & 524287) >>> (19 - 13)) | (input[31 + inpos] << 13);
}

function fastpack2(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 3) |
        ((input[1 + inpos] & 3) << 2) |
        ((input[2 + inpos] & 3) << 4) |
        ((input[3 + inpos] & 3) << 6) |
        ((input[4 + inpos] & 3) << 8) |
        ((input[5 + inpos] & 3) << 10) |
        ((input[6 + inpos] & 3) << 12) |
        ((input[7 + inpos] & 3) << 14) |
        ((input[8 + inpos] & 3) << 16) |
        ((input[9 + inpos] & 3) << 18) |
        ((input[10 + inpos] & 3) << 20) |
        ((input[11 + inpos] & 3) << 22) |
        ((input[12 + inpos] & 3) << 24) |
        ((input[13 + inpos] & 3) << 26) |
        ((input[14 + inpos] & 3) << 28) |
        (input[15 + inpos] << 30);
    output[1 + outpos] =
        (input[16 + inpos] & 3) |
        ((input[17 + inpos] & 3) << 2) |
        ((input[18 + inpos] & 3) << 4) |
        ((input[19 + inpos] & 3) << 6) |
        ((input[20 + inpos] & 3) << 8) |
        ((input[21 + inpos] & 3) << 10) |
        ((input[22 + inpos] & 3) << 12) |
        ((input[23 + inpos] & 3) << 14) |
        ((input[24 + inpos] & 3) << 16) |
        ((input[25 + inpos] & 3) << 18) |
        ((input[26 + inpos] & 3) << 20) |
        ((input[27 + inpos] & 3) << 22) |
        ((input[28 + inpos] & 3) << 24) |
        ((input[29 + inpos] & 3) << 26) |
        ((input[30 + inpos] & 3) << 28) |
        (input[31 + inpos] << 30);
}

function fastpack20(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 1048575) | (input[1 + inpos] << 20);
    output[1 + outpos] =
        ((input[1 + inpos] & 1048575) >>> (20 - 8)) | ((input[2 + inpos] & 1048575) << 8) | (input[3 + inpos] << 28);
    output[2 + outpos] = ((input[3 + inpos] & 1048575) >>> (20 - 16)) | (input[4 + inpos] << 16);
    output[3 + outpos] =
        ((input[4 + inpos] & 1048575) >>> (20 - 4)) | ((input[5 + inpos] & 1048575) << 4) | (input[6 + inpos] << 24);
    output[4 + outpos] = ((input[6 + inpos] & 1048575) >>> (20 - 12)) | (input[7 + inpos] << 12);
    output[5 + outpos] = (input[8 + inpos] & 1048575) | (input[9 + inpos] << 20);
    output[6 + outpos] =
        ((input[9 + inpos] & 1048575) >>> (20 - 8)) | ((input[10 + inpos] & 1048575) << 8) | (input[11 + inpos] << 28);
    output[7 + outpos] = ((input[11 + inpos] & 1048575) >>> (20 - 16)) | (input[12 + inpos] << 16);
    output[8 + outpos] =
        ((input[12 + inpos] & 1048575) >>> (20 - 4)) | ((input[13 + inpos] & 1048575) << 4) | (input[14 + inpos] << 24);
    output[9 + outpos] = ((input[14 + inpos] & 1048575) >>> (20 - 12)) | (input[15 + inpos] << 12);
    output[10 + outpos] = (input[16 + inpos] & 1048575) | (input[17 + inpos] << 20);
    output[11 + outpos] =
        ((input[17 + inpos] & 1048575) >>> (20 - 8)) | ((input[18 + inpos] & 1048575) << 8) | (input[19 + inpos] << 28);
    output[12 + outpos] = ((input[19 + inpos] & 1048575) >>> (20 - 16)) | (input[20 + inpos] << 16);
    output[13 + outpos] =
        ((input[20 + inpos] & 1048575) >>> (20 - 4)) | ((input[21 + inpos] & 1048575) << 4) | (input[22 + inpos] << 24);
    output[14 + outpos] = ((input[22 + inpos] & 1048575) >>> (20 - 12)) | (input[23 + inpos] << 12);
    output[15 + outpos] = (input[24 + inpos] & 1048575) | (input[25 + inpos] << 20);
    output[16 + outpos] =
        ((input[25 + inpos] & 1048575) >>> (20 - 8)) | ((input[26 + inpos] & 1048575) << 8) | (input[27 + inpos] << 28);
    output[17 + outpos] = ((input[27 + inpos] & 1048575) >>> (20 - 16)) | (input[28 + inpos] << 16);
    output[18 + outpos] =
        ((input[28 + inpos] & 1048575) >>> (20 - 4)) | ((input[29 + inpos] & 1048575) << 4) | (input[30 + inpos] << 24);
    output[19 + outpos] = ((input[30 + inpos] & 1048575) >>> (20 - 12)) | (input[31 + inpos] << 12);
}

function fastpack21(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 2097151) | (input[1 + inpos] << 21);
    output[1 + outpos] =
        ((input[1 + inpos] & 2097151) >>> (21 - 10)) | ((input[2 + inpos] & 2097151) << 10) | (input[3 + inpos] << 31);
    output[2 + outpos] = ((input[3 + inpos] & 2097151) >>> (21 - 20)) | (input[4 + inpos] << 20);
    output[3 + outpos] =
        ((input[4 + inpos] & 2097151) >>> (21 - 9)) | ((input[5 + inpos] & 2097151) << 9) | (input[6 + inpos] << 30);
    output[4 + outpos] = ((input[6 + inpos] & 2097151) >>> (21 - 19)) | (input[7 + inpos] << 19);
    output[5 + outpos] =
        ((input[7 + inpos] & 2097151) >>> (21 - 8)) | ((input[8 + inpos] & 2097151) << 8) | (input[9 + inpos] << 29);
    output[6 + outpos] = ((input[9 + inpos] & 2097151) >>> (21 - 18)) | (input[10 + inpos] << 18);
    output[7 + outpos] =
        ((input[10 + inpos] & 2097151) >>> (21 - 7)) | ((input[11 + inpos] & 2097151) << 7) | (input[12 + inpos] << 28);
    output[8 + outpos] = ((input[12 + inpos] & 2097151) >>> (21 - 17)) | (input[13 + inpos] << 17);
    output[9 + outpos] =
        ((input[13 + inpos] & 2097151) >>> (21 - 6)) | ((input[14 + inpos] & 2097151) << 6) | (input[15 + inpos] << 27);
    output[10 + outpos] = ((input[15 + inpos] & 2097151) >>> (21 - 16)) | (input[16 + inpos] << 16);
    output[11 + outpos] =
        ((input[16 + inpos] & 2097151) >>> (21 - 5)) | ((input[17 + inpos] & 2097151) << 5) | (input[18 + inpos] << 26);
    output[12 + outpos] = ((input[18 + inpos] & 2097151) >>> (21 - 15)) | (input[19 + inpos] << 15);
    output[13 + outpos] =
        ((input[19 + inpos] & 2097151) >>> (21 - 4)) | ((input[20 + inpos] & 2097151) << 4) | (input[21 + inpos] << 25);
    output[14 + outpos] = ((input[21 + inpos] & 2097151) >>> (21 - 14)) | (input[22 + inpos] << 14);
    output[15 + outpos] =
        ((input[22 + inpos] & 2097151) >>> (21 - 3)) | ((input[23 + inpos] & 2097151) << 3) | (input[24 + inpos] << 24);
    output[16 + outpos] = ((input[24 + inpos] & 2097151) >>> (21 - 13)) | (input[25 + inpos] << 13);
    output[17 + outpos] =
        ((input[25 + inpos] & 2097151) >>> (21 - 2)) | ((input[26 + inpos] & 2097151) << 2) | (input[27 + inpos] << 23);
    output[18 + outpos] = ((input[27 + inpos] & 2097151) >>> (21 - 12)) | (input[28 + inpos] << 12);
    output[19 + outpos] =
        ((input[28 + inpos] & 2097151) >>> (21 - 1)) | ((input[29 + inpos] & 2097151) << 1) | (input[30 + inpos] << 22);
    output[20 + outpos] = ((input[30 + inpos] & 2097151) >>> (21 - 11)) | (input[31 + inpos] << 11);
}

function fastpack22(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 4194303) | (input[1 + inpos] << 22);
    output[1 + outpos] = ((input[1 + inpos] & 4194303) >>> (22 - 12)) | (input[2 + inpos] << 12);
    output[2 + outpos] =
        ((input[2 + inpos] & 4194303) >>> (22 - 2)) | ((input[3 + inpos] & 4194303) << 2) | (input[4 + inpos] << 24);
    output[3 + outpos] = ((input[4 + inpos] & 4194303) >>> (22 - 14)) | (input[5 + inpos] << 14);
    output[4 + outpos] =
        ((input[5 + inpos] & 4194303) >>> (22 - 4)) | ((input[6 + inpos] & 4194303) << 4) | (input[7 + inpos] << 26);
    output[5 + outpos] = ((input[7 + inpos] & 4194303) >>> (22 - 16)) | (input[8 + inpos] << 16);
    output[6 + outpos] =
        ((input[8 + inpos] & 4194303) >>> (22 - 6)) | ((input[9 + inpos] & 4194303) << 6) | (input[10 + inpos] << 28);
    output[7 + outpos] = ((input[10 + inpos] & 4194303) >>> (22 - 18)) | (input[11 + inpos] << 18);
    output[8 + outpos] =
        ((input[11 + inpos] & 4194303) >>> (22 - 8)) | ((input[12 + inpos] & 4194303) << 8) | (input[13 + inpos] << 30);
    output[9 + outpos] = ((input[13 + inpos] & 4194303) >>> (22 - 20)) | (input[14 + inpos] << 20);
    output[10 + outpos] = ((input[14 + inpos] & 4194303) >>> (22 - 10)) | (input[15 + inpos] << 10);
    output[11 + outpos] = (input[16 + inpos] & 4194303) | (input[17 + inpos] << 22);
    output[12 + outpos] = ((input[17 + inpos] & 4194303) >>> (22 - 12)) | (input[18 + inpos] << 12);
    output[13 + outpos] =
        ((input[18 + inpos] & 4194303) >>> (22 - 2)) | ((input[19 + inpos] & 4194303) << 2) | (input[20 + inpos] << 24);
    output[14 + outpos] = ((input[20 + inpos] & 4194303) >>> (22 - 14)) | (input[21 + inpos] << 14);
    output[15 + outpos] =
        ((input[21 + inpos] & 4194303) >>> (22 - 4)) | ((input[22 + inpos] & 4194303) << 4) | (input[23 + inpos] << 26);
    output[16 + outpos] = ((input[23 + inpos] & 4194303) >>> (22 - 16)) | (input[24 + inpos] << 16);
    output[17 + outpos] =
        ((input[24 + inpos] & 4194303) >>> (22 - 6)) | ((input[25 + inpos] & 4194303) << 6) | (input[26 + inpos] << 28);
    output[18 + outpos] = ((input[26 + inpos] & 4194303) >>> (22 - 18)) | (input[27 + inpos] << 18);
    output[19 + outpos] =
        ((input[27 + inpos] & 4194303) >>> (22 - 8)) | ((input[28 + inpos] & 4194303) << 8) | (input[29 + inpos] << 30);
    output[20 + outpos] = ((input[29 + inpos] & 4194303) >>> (22 - 20)) | (input[30 + inpos] << 20);
    output[21 + outpos] = ((input[30 + inpos] & 4194303) >>> (22 - 10)) | (input[31 + inpos] << 10);
}

function fastpack23(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 8388607) | (input[1 + inpos] << 23);
    output[1 + outpos] = ((input[1 + inpos] & 8388607) >>> (23 - 14)) | (input[2 + inpos] << 14);
    output[2 + outpos] =
        ((input[2 + inpos] & 8388607) >>> (23 - 5)) | ((input[3 + inpos] & 8388607) << 5) | (input[4 + inpos] << 28);
    output[3 + outpos] = ((input[4 + inpos] & 8388607) >>> (23 - 19)) | (input[5 + inpos] << 19);
    output[4 + outpos] = ((input[5 + inpos] & 8388607) >>> (23 - 10)) | (input[6 + inpos] << 10);
    output[5 + outpos] =
        ((input[6 + inpos] & 8388607) >>> (23 - 1)) | ((input[7 + inpos] & 8388607) << 1) | (input[8 + inpos] << 24);
    output[6 + outpos] = ((input[8 + inpos] & 8388607) >>> (23 - 15)) | (input[9 + inpos] << 15);
    output[7 + outpos] =
        ((input[9 + inpos] & 8388607) >>> (23 - 6)) | ((input[10 + inpos] & 8388607) << 6) | (input[11 + inpos] << 29);
    output[8 + outpos] = ((input[11 + inpos] & 8388607) >>> (23 - 20)) | (input[12 + inpos] << 20);
    output[9 + outpos] = ((input[12 + inpos] & 8388607) >>> (23 - 11)) | (input[13 + inpos] << 11);
    output[10 + outpos] =
        ((input[13 + inpos] & 8388607) >>> (23 - 2)) | ((input[14 + inpos] & 8388607) << 2) | (input[15 + inpos] << 25);
    output[11 + outpos] = ((input[15 + inpos] & 8388607) >>> (23 - 16)) | (input[16 + inpos] << 16);
    output[12 + outpos] =
        ((input[16 + inpos] & 8388607) >>> (23 - 7)) | ((input[17 + inpos] & 8388607) << 7) | (input[18 + inpos] << 30);
    output[13 + outpos] = ((input[18 + inpos] & 8388607) >>> (23 - 21)) | (input[19 + inpos] << 21);
    output[14 + outpos] = ((input[19 + inpos] & 8388607) >>> (23 - 12)) | (input[20 + inpos] << 12);
    output[15 + outpos] =
        ((input[20 + inpos] & 8388607) >>> (23 - 3)) | ((input[21 + inpos] & 8388607) << 3) | (input[22 + inpos] << 26);
    output[16 + outpos] = ((input[22 + inpos] & 8388607) >>> (23 - 17)) | (input[23 + inpos] << 17);
    output[17 + outpos] =
        ((input[23 + inpos] & 8388607) >>> (23 - 8)) | ((input[24 + inpos] & 8388607) << 8) | (input[25 + inpos] << 31);
    output[18 + outpos] = ((input[25 + inpos] & 8388607) >>> (23 - 22)) | (input[26 + inpos] << 22);
    output[19 + outpos] = ((input[26 + inpos] & 8388607) >>> (23 - 13)) | (input[27 + inpos] << 13);
    output[20 + outpos] =
        ((input[27 + inpos] & 8388607) >>> (23 - 4)) | ((input[28 + inpos] & 8388607) << 4) | (input[29 + inpos] << 27);
    output[21 + outpos] = ((input[29 + inpos] & 8388607) >>> (23 - 18)) | (input[30 + inpos] << 18);
    output[22 + outpos] = ((input[30 + inpos] & 8388607) >>> (23 - 9)) | (input[31 + inpos] << 9);
}

function fastpack24(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 16777215) | (input[1 + inpos] << 24);
    output[1 + outpos] = ((input[1 + inpos] & 16777215) >>> (24 - 16)) | (input[2 + inpos] << 16);
    output[2 + outpos] = ((input[2 + inpos] & 16777215) >>> (24 - 8)) | (input[3 + inpos] << 8);
    output[3 + outpos] = (input[4 + inpos] & 16777215) | (input[5 + inpos] << 24);
    output[4 + outpos] = ((input[5 + inpos] & 16777215) >>> (24 - 16)) | (input[6 + inpos] << 16);
    output[5 + outpos] = ((input[6 + inpos] & 16777215) >>> (24 - 8)) | (input[7 + inpos] << 8);
    output[6 + outpos] = (input[8 + inpos] & 16777215) | (input[9 + inpos] << 24);
    output[7 + outpos] = ((input[9 + inpos] & 16777215) >>> (24 - 16)) | (input[10 + inpos] << 16);
    output[8 + outpos] = ((input[10 + inpos] & 16777215) >>> (24 - 8)) | (input[11 + inpos] << 8);
    output[9 + outpos] = (input[12 + inpos] & 16777215) | (input[13 + inpos] << 24);
    output[10 + outpos] = ((input[13 + inpos] & 16777215) >>> (24 - 16)) | (input[14 + inpos] << 16);
    output[11 + outpos] = ((input[14 + inpos] & 16777215) >>> (24 - 8)) | (input[15 + inpos] << 8);
    output[12 + outpos] = (input[16 + inpos] & 16777215) | (input[17 + inpos] << 24);
    output[13 + outpos] = ((input[17 + inpos] & 16777215) >>> (24 - 16)) | (input[18 + inpos] << 16);
    output[14 + outpos] = ((input[18 + inpos] & 16777215) >>> (24 - 8)) | (input[19 + inpos] << 8);
    output[15 + outpos] = (input[20 + inpos] & 16777215) | (input[21 + inpos] << 24);
    output[16 + outpos] = ((input[21 + inpos] & 16777215) >>> (24 - 16)) | (input[22 + inpos] << 16);
    output[17 + outpos] = ((input[22 + inpos] & 16777215) >>> (24 - 8)) | (input[23 + inpos] << 8);
    output[18 + outpos] = (input[24 + inpos] & 16777215) | (input[25 + inpos] << 24);
    output[19 + outpos] = ((input[25 + inpos] & 16777215) >>> (24 - 16)) | (input[26 + inpos] << 16);
    output[20 + outpos] = ((input[26 + inpos] & 16777215) >>> (24 - 8)) | (input[27 + inpos] << 8);
    output[21 + outpos] = (input[28 + inpos] & 16777215) | (input[29 + inpos] << 24);
    output[22 + outpos] = ((input[29 + inpos] & 16777215) >>> (24 - 16)) | (input[30 + inpos] << 16);
    output[23 + outpos] = ((input[30 + inpos] & 16777215) >>> (24 - 8)) | (input[31 + inpos] << 8);
}

function fastpack25(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 33554431) | (input[1 + inpos] << 25);
    output[1 + outpos] = ((input[1 + inpos] & 33554431) >>> (25 - 18)) | (input[2 + inpos] << 18);
    output[2 + outpos] = ((input[2 + inpos] & 33554431) >>> (25 - 11)) | (input[3 + inpos] << 11);
    output[3 + outpos] =
        ((input[3 + inpos] & 33554431) >>> (25 - 4)) | ((input[4 + inpos] & 33554431) << 4) | (input[5 + inpos] << 29);
    output[4 + outpos] = ((input[5 + inpos] & 33554431) >>> (25 - 22)) | (input[6 + inpos] << 22);
    output[5 + outpos] = ((input[6 + inpos] & 33554431) >>> (25 - 15)) | (input[7 + inpos] << 15);
    output[6 + outpos] = ((input[7 + inpos] & 33554431) >>> (25 - 8)) | (input[8 + inpos] << 8);
    output[7 + outpos] =
        ((input[8 + inpos] & 33554431) >>> (25 - 1)) | ((input[9 + inpos] & 33554431) << 1) | (input[10 + inpos] << 26);
    output[8 + outpos] = ((input[10 + inpos] & 33554431) >>> (25 - 19)) | (input[11 + inpos] << 19);
    output[9 + outpos] = ((input[11 + inpos] & 33554431) >>> (25 - 12)) | (input[12 + inpos] << 12);
    output[10 + outpos] =
        ((input[12 + inpos] & 33554431) >>> (25 - 5)) |
        ((input[13 + inpos] & 33554431) << 5) |
        (input[14 + inpos] << 30);
    output[11 + outpos] = ((input[14 + inpos] & 33554431) >>> (25 - 23)) | (input[15 + inpos] << 23);
    output[12 + outpos] = ((input[15 + inpos] & 33554431) >>> (25 - 16)) | (input[16 + inpos] << 16);
    output[13 + outpos] = ((input[16 + inpos] & 33554431) >>> (25 - 9)) | (input[17 + inpos] << 9);
    output[14 + outpos] =
        ((input[17 + inpos] & 33554431) >>> (25 - 2)) |
        ((input[18 + inpos] & 33554431) << 2) |
        (input[19 + inpos] << 27);
    output[15 + outpos] = ((input[19 + inpos] & 33554431) >>> (25 - 20)) | (input[20 + inpos] << 20);
    output[16 + outpos] = ((input[20 + inpos] & 33554431) >>> (25 - 13)) | (input[21 + inpos] << 13);
    output[17 + outpos] =
        ((input[21 + inpos] & 33554431) >>> (25 - 6)) |
        ((input[22 + inpos] & 33554431) << 6) |
        (input[23 + inpos] << 31);
    output[18 + outpos] = ((input[23 + inpos] & 33554431) >>> (25 - 24)) | (input[24 + inpos] << 24);
    output[19 + outpos] = ((input[24 + inpos] & 33554431) >>> (25 - 17)) | (input[25 + inpos] << 17);
    output[20 + outpos] = ((input[25 + inpos] & 33554431) >>> (25 - 10)) | (input[26 + inpos] << 10);
    output[21 + outpos] =
        ((input[26 + inpos] & 33554431) >>> (25 - 3)) |
        ((input[27 + inpos] & 33554431) << 3) |
        (input[28 + inpos] << 28);
    output[22 + outpos] = ((input[28 + inpos] & 33554431) >>> (25 - 21)) | (input[29 + inpos] << 21);
    output[23 + outpos] = ((input[29 + inpos] & 33554431) >>> (25 - 14)) | (input[30 + inpos] << 14);
    output[24 + outpos] = ((input[30 + inpos] & 33554431) >>> (25 - 7)) | (input[31 + inpos] << 7);
}

function fastpack26(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 67108863) | (input[1 + inpos] << 26);
    output[1 + outpos] = ((input[1 + inpos] & 67108863) >>> (26 - 20)) | (input[2 + inpos] << 20);
    output[2 + outpos] = ((input[2 + inpos] & 67108863) >>> (26 - 14)) | (input[3 + inpos] << 14);
    output[3 + outpos] = ((input[3 + inpos] & 67108863) >>> (26 - 8)) | (input[4 + inpos] << 8);
    output[4 + outpos] =
        ((input[4 + inpos] & 67108863) >>> (26 - 2)) | ((input[5 + inpos] & 67108863) << 2) | (input[6 + inpos] << 28);
    output[5 + outpos] = ((input[6 + inpos] & 67108863) >>> (26 - 22)) | (input[7 + inpos] << 22);
    output[6 + outpos] = ((input[7 + inpos] & 67108863) >>> (26 - 16)) | (input[8 + inpos] << 16);
    output[7 + outpos] = ((input[8 + inpos] & 67108863) >>> (26 - 10)) | (input[9 + inpos] << 10);
    output[8 + outpos] =
        ((input[9 + inpos] & 67108863) >>> (26 - 4)) |
        ((input[10 + inpos] & 67108863) << 4) |
        (input[11 + inpos] << 30);
    output[9 + outpos] = ((input[11 + inpos] & 67108863) >>> (26 - 24)) | (input[12 + inpos] << 24);
    output[10 + outpos] = ((input[12 + inpos] & 67108863) >>> (26 - 18)) | (input[13 + inpos] << 18);
    output[11 + outpos] = ((input[13 + inpos] & 67108863) >>> (26 - 12)) | (input[14 + inpos] << 12);
    output[12 + outpos] = ((input[14 + inpos] & 67108863) >>> (26 - 6)) | (input[15 + inpos] << 6);
    output[13 + outpos] = (input[16 + inpos] & 67108863) | (input[17 + inpos] << 26);
    output[14 + outpos] = ((input[17 + inpos] & 67108863) >>> (26 - 20)) | (input[18 + inpos] << 20);
    output[15 + outpos] = ((input[18 + inpos] & 67108863) >>> (26 - 14)) | (input[19 + inpos] << 14);
    output[16 + outpos] = ((input[19 + inpos] & 67108863) >>> (26 - 8)) | (input[20 + inpos] << 8);
    output[17 + outpos] =
        ((input[20 + inpos] & 67108863) >>> (26 - 2)) |
        ((input[21 + inpos] & 67108863) << 2) |
        (input[22 + inpos] << 28);
    output[18 + outpos] = ((input[22 + inpos] & 67108863) >>> (26 - 22)) | (input[23 + inpos] << 22);
    output[19 + outpos] = ((input[23 + inpos] & 67108863) >>> (26 - 16)) | (input[24 + inpos] << 16);
    output[20 + outpos] = ((input[24 + inpos] & 67108863) >>> (26 - 10)) | (input[25 + inpos] << 10);
    output[21 + outpos] =
        ((input[25 + inpos] & 67108863) >>> (26 - 4)) |
        ((input[26 + inpos] & 67108863) << 4) |
        (input[27 + inpos] << 30);
    output[22 + outpos] = ((input[27 + inpos] & 67108863) >>> (26 - 24)) | (input[28 + inpos] << 24);
    output[23 + outpos] = ((input[28 + inpos] & 67108863) >>> (26 - 18)) | (input[29 + inpos] << 18);
    output[24 + outpos] = ((input[29 + inpos] & 67108863) >>> (26 - 12)) | (input[30 + inpos] << 12);
    output[25 + outpos] = ((input[30 + inpos] & 67108863) >>> (26 - 6)) | (input[31 + inpos] << 6);
}

function fastpack27(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 134217727) | (input[1 + inpos] << 27);
    output[1 + outpos] = ((input[1 + inpos] & 134217727) >>> (27 - 22)) | (input[2 + inpos] << 22);
    output[2 + outpos] = ((input[2 + inpos] & 134217727) >>> (27 - 17)) | (input[3 + inpos] << 17);
    output[3 + outpos] = ((input[3 + inpos] & 134217727) >>> (27 - 12)) | (input[4 + inpos] << 12);
    output[4 + outpos] = ((input[4 + inpos] & 134217727) >>> (27 - 7)) | (input[5 + inpos] << 7);
    output[5 + outpos] =
        ((input[5 + inpos] & 134217727) >>> (27 - 2)) |
        ((input[6 + inpos] & 134217727) << 2) |
        (input[7 + inpos] << 29);
    output[6 + outpos] = ((input[7 + inpos] & 134217727) >>> (27 - 24)) | (input[8 + inpos] << 24);
    output[7 + outpos] = ((input[8 + inpos] & 134217727) >>> (27 - 19)) | (input[9 + inpos] << 19);
    output[8 + outpos] = ((input[9 + inpos] & 134217727) >>> (27 - 14)) | (input[10 + inpos] << 14);
    output[9 + outpos] = ((input[10 + inpos] & 134217727) >>> (27 - 9)) | (input[11 + inpos] << 9);
    output[10 + outpos] =
        ((input[11 + inpos] & 134217727) >>> (27 - 4)) |
        ((input[12 + inpos] & 134217727) << 4) |
        (input[13 + inpos] << 31);
    output[11 + outpos] = ((input[13 + inpos] & 134217727) >>> (27 - 26)) | (input[14 + inpos] << 26);
    output[12 + outpos] = ((input[14 + inpos] & 134217727) >>> (27 - 21)) | (input[15 + inpos] << 21);
    output[13 + outpos] = ((input[15 + inpos] & 134217727) >>> (27 - 16)) | (input[16 + inpos] << 16);
    output[14 + outpos] = ((input[16 + inpos] & 134217727) >>> (27 - 11)) | (input[17 + inpos] << 11);
    output[15 + outpos] = ((input[17 + inpos] & 134217727) >>> (27 - 6)) | (input[18 + inpos] << 6);
    output[16 + outpos] =
        ((input[18 + inpos] & 134217727) >>> (27 - 1)) |
        ((input[19 + inpos] & 134217727) << 1) |
        (input[20 + inpos] << 28);
    output[17 + outpos] = ((input[20 + inpos] & 134217727) >>> (27 - 23)) | (input[21 + inpos] << 23);
    output[18 + outpos] = ((input[21 + inpos] & 134217727) >>> (27 - 18)) | (input[22 + inpos] << 18);
    output[19 + outpos] = ((input[22 + inpos] & 134217727) >>> (27 - 13)) | (input[23 + inpos] << 13);
    output[20 + outpos] = ((input[23 + inpos] & 134217727) >>> (27 - 8)) | (input[24 + inpos] << 8);
    output[21 + outpos] =
        ((input[24 + inpos] & 134217727) >>> (27 - 3)) |
        ((input[25 + inpos] & 134217727) << 3) |
        (input[26 + inpos] << 30);
    output[22 + outpos] = ((input[26 + inpos] & 134217727) >>> (27 - 25)) | (input[27 + inpos] << 25);
    output[23 + outpos] = ((input[27 + inpos] & 134217727) >>> (27 - 20)) | (input[28 + inpos] << 20);
    output[24 + outpos] = ((input[28 + inpos] & 134217727) >>> (27 - 15)) | (input[29 + inpos] << 15);
    output[25 + outpos] = ((input[29 + inpos] & 134217727) >>> (27 - 10)) | (input[30 + inpos] << 10);
    output[26 + outpos] = ((input[30 + inpos] & 134217727) >>> (27 - 5)) | (input[31 + inpos] << 5);
}

function fastpack28(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 268435455) | (input[1 + inpos] << 28);
    output[1 + outpos] = ((input[1 + inpos] & 268435455) >>> (28 - 24)) | (input[2 + inpos] << 24);
    output[2 + outpos] = ((input[2 + inpos] & 268435455) >>> (28 - 20)) | (input[3 + inpos] << 20);
    output[3 + outpos] = ((input[3 + inpos] & 268435455) >>> (28 - 16)) | (input[4 + inpos] << 16);
    output[4 + outpos] = ((input[4 + inpos] & 268435455) >>> (28 - 12)) | (input[5 + inpos] << 12);
    output[5 + outpos] = ((input[5 + inpos] & 268435455) >>> (28 - 8)) | (input[6 + inpos] << 8);
    output[6 + outpos] = ((input[6 + inpos] & 268435455) >>> (28 - 4)) | (input[7 + inpos] << 4);
    output[7 + outpos] = (input[8 + inpos] & 268435455) | (input[9 + inpos] << 28);
    output[8 + outpos] = ((input[9 + inpos] & 268435455) >>> (28 - 24)) | (input[10 + inpos] << 24);
    output[9 + outpos] = ((input[10 + inpos] & 268435455) >>> (28 - 20)) | (input[11 + inpos] << 20);
    output[10 + outpos] = ((input[11 + inpos] & 268435455) >>> (28 - 16)) | (input[12 + inpos] << 16);
    output[11 + outpos] = ((input[12 + inpos] & 268435455) >>> (28 - 12)) | (input[13 + inpos] << 12);
    output[12 + outpos] = ((input[13 + inpos] & 268435455) >>> (28 - 8)) | (input[14 + inpos] << 8);
    output[13 + outpos] = ((input[14 + inpos] & 268435455) >>> (28 - 4)) | (input[15 + inpos] << 4);
    output[14 + outpos] = (input[16 + inpos] & 268435455) | (input[17 + inpos] << 28);
    output[15 + outpos] = ((input[17 + inpos] & 268435455) >>> (28 - 24)) | (input[18 + inpos] << 24);
    output[16 + outpos] = ((input[18 + inpos] & 268435455) >>> (28 - 20)) | (input[19 + inpos] << 20);
    output[17 + outpos] = ((input[19 + inpos] & 268435455) >>> (28 - 16)) | (input[20 + inpos] << 16);
    output[18 + outpos] = ((input[20 + inpos] & 268435455) >>> (28 - 12)) | (input[21 + inpos] << 12);
    output[19 + outpos] = ((input[21 + inpos] & 268435455) >>> (28 - 8)) | (input[22 + inpos] << 8);
    output[20 + outpos] = ((input[22 + inpos] & 268435455) >>> (28 - 4)) | (input[23 + inpos] << 4);
    output[21 + outpos] = (input[24 + inpos] & 268435455) | (input[25 + inpos] << 28);
    output[22 + outpos] = ((input[25 + inpos] & 268435455) >>> (28 - 24)) | (input[26 + inpos] << 24);
    output[23 + outpos] = ((input[26 + inpos] & 268435455) >>> (28 - 20)) | (input[27 + inpos] << 20);
    output[24 + outpos] = ((input[27 + inpos] & 268435455) >>> (28 - 16)) | (input[28 + inpos] << 16);
    output[25 + outpos] = ((input[28 + inpos] & 268435455) >>> (28 - 12)) | (input[29 + inpos] << 12);
    output[26 + outpos] = ((input[29 + inpos] & 268435455) >>> (28 - 8)) | (input[30 + inpos] << 8);
    output[27 + outpos] = ((input[30 + inpos] & 268435455) >>> (28 - 4)) | (input[31 + inpos] << 4);
}

function fastpack29(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 536870911) | (input[1 + inpos] << 29);
    output[1 + outpos] = ((input[1 + inpos] & 536870911) >>> (29 - 26)) | (input[2 + inpos] << 26);
    output[2 + outpos] = ((input[2 + inpos] & 536870911) >>> (29 - 23)) | (input[3 + inpos] << 23);
    output[3 + outpos] = ((input[3 + inpos] & 536870911) >>> (29 - 20)) | (input[4 + inpos] << 20);
    output[4 + outpos] = ((input[4 + inpos] & 536870911) >>> (29 - 17)) | (input[5 + inpos] << 17);
    output[5 + outpos] = ((input[5 + inpos] & 536870911) >>> (29 - 14)) | (input[6 + inpos] << 14);
    output[6 + outpos] = ((input[6 + inpos] & 536870911) >>> (29 - 11)) | (input[7 + inpos] << 11);
    output[7 + outpos] = ((input[7 + inpos] & 536870911) >>> (29 - 8)) | (input[8 + inpos] << 8);
    output[8 + outpos] = ((input[8 + inpos] & 536870911) >>> (29 - 5)) | (input[9 + inpos] << 5);
    output[9 + outpos] =
        ((input[9 + inpos] & 536870911) >>> (29 - 2)) |
        ((input[10 + inpos] & 536870911) << 2) |
        (input[11 + inpos] << 31);
    output[10 + outpos] = ((input[11 + inpos] & 536870911) >>> (29 - 28)) | (input[12 + inpos] << 28);
    output[11 + outpos] = ((input[12 + inpos] & 536870911) >>> (29 - 25)) | (input[13 + inpos] << 25);
    output[12 + outpos] = ((input[13 + inpos] & 536870911) >>> (29 - 22)) | (input[14 + inpos] << 22);
    output[13 + outpos] = ((input[14 + inpos] & 536870911) >>> (29 - 19)) | (input[15 + inpos] << 19);
    output[14 + outpos] = ((input[15 + inpos] & 536870911) >>> (29 - 16)) | (input[16 + inpos] << 16);
    output[15 + outpos] = ((input[16 + inpos] & 536870911) >>> (29 - 13)) | (input[17 + inpos] << 13);
    output[16 + outpos] = ((input[17 + inpos] & 536870911) >>> (29 - 10)) | (input[18 + inpos] << 10);
    output[17 + outpos] = ((input[18 + inpos] & 536870911) >>> (29 - 7)) | (input[19 + inpos] << 7);
    output[18 + outpos] = ((input[19 + inpos] & 536870911) >>> (29 - 4)) | (input[20 + inpos] << 4);
    output[19 + outpos] =
        ((input[20 + inpos] & 536870911) >>> (29 - 1)) |
        ((input[21 + inpos] & 536870911) << 1) |
        (input[22 + inpos] << 30);
    output[20 + outpos] = ((input[22 + inpos] & 536870911) >>> (29 - 27)) | (input[23 + inpos] << 27);
    output[21 + outpos] = ((input[23 + inpos] & 536870911) >>> (29 - 24)) | (input[24 + inpos] << 24);
    output[22 + outpos] = ((input[24 + inpos] & 536870911) >>> (29 - 21)) | (input[25 + inpos] << 21);
    output[23 + outpos] = ((input[25 + inpos] & 536870911) >>> (29 - 18)) | (input[26 + inpos] << 18);
    output[24 + outpos] = ((input[26 + inpos] & 536870911) >>> (29 - 15)) | (input[27 + inpos] << 15);
    output[25 + outpos] = ((input[27 + inpos] & 536870911) >>> (29 - 12)) | (input[28 + inpos] << 12);
    output[26 + outpos] = ((input[28 + inpos] & 536870911) >>> (29 - 9)) | (input[29 + inpos] << 9);
    output[27 + outpos] = ((input[29 + inpos] & 536870911) >>> (29 - 6)) | (input[30 + inpos] << 6);
    output[28 + outpos] = ((input[30 + inpos] & 536870911) >>> (29 - 3)) | (input[31 + inpos] << 3);
}

function fastpack3(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 7) |
        ((input[1 + inpos] & 7) << 3) |
        ((input[2 + inpos] & 7) << 6) |
        ((input[3 + inpos] & 7) << 9) |
        ((input[4 + inpos] & 7) << 12) |
        ((input[5 + inpos] & 7) << 15) |
        ((input[6 + inpos] & 7) << 18) |
        ((input[7 + inpos] & 7) << 21) |
        ((input[8 + inpos] & 7) << 24) |
        ((input[9 + inpos] & 7) << 27) |
        (input[10 + inpos] << 30);
    output[1 + outpos] =
        ((input[10 + inpos] & 7) >>> (3 - 1)) |
        ((input[11 + inpos] & 7) << 1) |
        ((input[12 + inpos] & 7) << 4) |
        ((input[13 + inpos] & 7) << 7) |
        ((input[14 + inpos] & 7) << 10) |
        ((input[15 + inpos] & 7) << 13) |
        ((input[16 + inpos] & 7) << 16) |
        ((input[17 + inpos] & 7) << 19) |
        ((input[18 + inpos] & 7) << 22) |
        ((input[19 + inpos] & 7) << 25) |
        ((input[20 + inpos] & 7) << 28) |
        (input[21 + inpos] << 31);
    output[2 + outpos] =
        ((input[21 + inpos] & 7) >>> (3 - 2)) |
        ((input[22 + inpos] & 7) << 2) |
        ((input[23 + inpos] & 7) << 5) |
        ((input[24 + inpos] & 7) << 8) |
        ((input[25 + inpos] & 7) << 11) |
        ((input[26 + inpos] & 7) << 14) |
        ((input[27 + inpos] & 7) << 17) |
        ((input[28 + inpos] & 7) << 20) |
        ((input[29 + inpos] & 7) << 23) |
        ((input[30 + inpos] & 7) << 26) |
        (input[31 + inpos] << 29);
}

function fastpack30(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 1073741823) | (input[1 + inpos] << 30);
    output[1 + outpos] = ((input[1 + inpos] & 1073741823) >>> (30 - 28)) | (input[2 + inpos] << 28);
    output[2 + outpos] = ((input[2 + inpos] & 1073741823) >>> (30 - 26)) | (input[3 + inpos] << 26);
    output[3 + outpos] = ((input[3 + inpos] & 1073741823) >>> (30 - 24)) | (input[4 + inpos] << 24);
    output[4 + outpos] = ((input[4 + inpos] & 1073741823) >>> (30 - 22)) | (input[5 + inpos] << 22);
    output[5 + outpos] = ((input[5 + inpos] & 1073741823) >>> (30 - 20)) | (input[6 + inpos] << 20);
    output[6 + outpos] = ((input[6 + inpos] & 1073741823) >>> (30 - 18)) | (input[7 + inpos] << 18);
    output[7 + outpos] = ((input[7 + inpos] & 1073741823) >>> (30 - 16)) | (input[8 + inpos] << 16);
    output[8 + outpos] = ((input[8 + inpos] & 1073741823) >>> (30 - 14)) | (input[9 + inpos] << 14);
    output[9 + outpos] = ((input[9 + inpos] & 1073741823) >>> (30 - 12)) | (input[10 + inpos] << 12);
    output[10 + outpos] = ((input[10 + inpos] & 1073741823) >>> (30 - 10)) | (input[11 + inpos] << 10);
    output[11 + outpos] = ((input[11 + inpos] & 1073741823) >>> (30 - 8)) | (input[12 + inpos] << 8);
    output[12 + outpos] = ((input[12 + inpos] & 1073741823) >>> (30 - 6)) | (input[13 + inpos] << 6);
    output[13 + outpos] = ((input[13 + inpos] & 1073741823) >>> (30 - 4)) | (input[14 + inpos] << 4);
    output[14 + outpos] = ((input[14 + inpos] & 1073741823) >>> (30 - 2)) | (input[15 + inpos] << 2);
    output[15 + outpos] = (input[16 + inpos] & 1073741823) | (input[17 + inpos] << 30);
    output[16 + outpos] = ((input[17 + inpos] & 1073741823) >>> (30 - 28)) | (input[18 + inpos] << 28);
    output[17 + outpos] = ((input[18 + inpos] & 1073741823) >>> (30 - 26)) | (input[19 + inpos] << 26);
    output[18 + outpos] = ((input[19 + inpos] & 1073741823) >>> (30 - 24)) | (input[20 + inpos] << 24);
    output[19 + outpos] = ((input[20 + inpos] & 1073741823) >>> (30 - 22)) | (input[21 + inpos] << 22);
    output[20 + outpos] = ((input[21 + inpos] & 1073741823) >>> (30 - 20)) | (input[22 + inpos] << 20);
    output[21 + outpos] = ((input[22 + inpos] & 1073741823) >>> (30 - 18)) | (input[23 + inpos] << 18);
    output[22 + outpos] = ((input[23 + inpos] & 1073741823) >>> (30 - 16)) | (input[24 + inpos] << 16);
    output[23 + outpos] = ((input[24 + inpos] & 1073741823) >>> (30 - 14)) | (input[25 + inpos] << 14);
    output[24 + outpos] = ((input[25 + inpos] & 1073741823) >>> (30 - 12)) | (input[26 + inpos] << 12);
    output[25 + outpos] = ((input[26 + inpos] & 1073741823) >>> (30 - 10)) | (input[27 + inpos] << 10);
    output[26 + outpos] = ((input[27 + inpos] & 1073741823) >>> (30 - 8)) | (input[28 + inpos] << 8);
    output[27 + outpos] = ((input[28 + inpos] & 1073741823) >>> (30 - 6)) | (input[29 + inpos] << 6);
    output[28 + outpos] = ((input[29 + inpos] & 1073741823) >>> (30 - 4)) | (input[30 + inpos] << 4);
    output[29 + outpos] = ((input[30 + inpos] & 1073741823) >>> (30 - 2)) | (input[31 + inpos] << 2);
}

function fastpack31(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = (input[inpos] & 2147483647) | (input[1 + inpos] << 31);
    output[1 + outpos] = ((input[1 + inpos] & 2147483647) >>> (31 - 30)) | (input[2 + inpos] << 30);
    output[2 + outpos] = ((input[2 + inpos] & 2147483647) >>> (31 - 29)) | (input[3 + inpos] << 29);
    output[3 + outpos] = ((input[3 + inpos] & 2147483647) >>> (31 - 28)) | (input[4 + inpos] << 28);
    output[4 + outpos] = ((input[4 + inpos] & 2147483647) >>> (31 - 27)) | (input[5 + inpos] << 27);
    output[5 + outpos] = ((input[5 + inpos] & 2147483647) >>> (31 - 26)) | (input[6 + inpos] << 26);
    output[6 + outpos] = ((input[6 + inpos] & 2147483647) >>> (31 - 25)) | (input[7 + inpos] << 25);
    output[7 + outpos] = ((input[7 + inpos] & 2147483647) >>> (31 - 24)) | (input[8 + inpos] << 24);
    output[8 + outpos] = ((input[8 + inpos] & 2147483647) >>> (31 - 23)) | (input[9 + inpos] << 23);
    output[9 + outpos] = ((input[9 + inpos] & 2147483647) >>> (31 - 22)) | (input[10 + inpos] << 22);
    output[10 + outpos] = ((input[10 + inpos] & 2147483647) >>> (31 - 21)) | (input[11 + inpos] << 21);
    output[11 + outpos] = ((input[11 + inpos] & 2147483647) >>> (31 - 20)) | (input[12 + inpos] << 20);
    output[12 + outpos] = ((input[12 + inpos] & 2147483647) >>> (31 - 19)) | (input[13 + inpos] << 19);
    output[13 + outpos] = ((input[13 + inpos] & 2147483647) >>> (31 - 18)) | (input[14 + inpos] << 18);
    output[14 + outpos] = ((input[14 + inpos] & 2147483647) >>> (31 - 17)) | (input[15 + inpos] << 17);
    output[15 + outpos] = ((input[15 + inpos] & 2147483647) >>> (31 - 16)) | (input[16 + inpos] << 16);
    output[16 + outpos] = ((input[16 + inpos] & 2147483647) >>> (31 - 15)) | (input[17 + inpos] << 15);
    output[17 + outpos] = ((input[17 + inpos] & 2147483647) >>> (31 - 14)) | (input[18 + inpos] << 14);
    output[18 + outpos] = ((input[18 + inpos] & 2147483647) >>> (31 - 13)) | (input[19 + inpos] << 13);
    output[19 + outpos] = ((input[19 + inpos] & 2147483647) >>> (31 - 12)) | (input[20 + inpos] << 12);
    output[20 + outpos] = ((input[20 + inpos] & 2147483647) >>> (31 - 11)) | (input[21 + inpos] << 11);
    output[21 + outpos] = ((input[21 + inpos] & 2147483647) >>> (31 - 10)) | (input[22 + inpos] << 10);
    output[22 + outpos] = ((input[22 + inpos] & 2147483647) >>> (31 - 9)) | (input[23 + inpos] << 9);
    output[23 + outpos] = ((input[23 + inpos] & 2147483647) >>> (31 - 8)) | (input[24 + inpos] << 8);
    output[24 + outpos] = ((input[24 + inpos] & 2147483647) >>> (31 - 7)) | (input[25 + inpos] << 7);
    output[25 + outpos] = ((input[25 + inpos] & 2147483647) >>> (31 - 6)) | (input[26 + inpos] << 6);
    output[26 + outpos] = ((input[26 + inpos] & 2147483647) >>> (31 - 5)) | (input[27 + inpos] << 5);
    output[27 + outpos] = ((input[27 + inpos] & 2147483647) >>> (31 - 4)) | (input[28 + inpos] << 4);
    output[28 + outpos] = ((input[28 + inpos] & 2147483647) >>> (31 - 3)) | (input[29 + inpos] << 3);
    output[29 + outpos] = ((input[29 + inpos] & 2147483647) >>> (31 - 2)) | (input[30 + inpos] << 2);
    output[30 + outpos] = ((input[30 + inpos] & 2147483647) >>> (31 - 1)) | (input[31 + inpos] << 1);
}

function fastpack32(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    arraycopy(input, inpos, output, outpos, 32);
}

function fastpack4(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 15) |
        ((input[1 + inpos] & 15) << 4) |
        ((input[2 + inpos] & 15) << 8) |
        ((input[3 + inpos] & 15) << 12) |
        ((input[4 + inpos] & 15) << 16) |
        ((input[5 + inpos] & 15) << 20) |
        ((input[6 + inpos] & 15) << 24) |
        (input[7 + inpos] << 28);
    output[1 + outpos] =
        (input[8 + inpos] & 15) |
        ((input[9 + inpos] & 15) << 4) |
        ((input[10 + inpos] & 15) << 8) |
        ((input[11 + inpos] & 15) << 12) |
        ((input[12 + inpos] & 15) << 16) |
        ((input[13 + inpos] & 15) << 20) |
        ((input[14 + inpos] & 15) << 24) |
        (input[15 + inpos] << 28);
    output[2 + outpos] =
        (input[16 + inpos] & 15) |
        ((input[17 + inpos] & 15) << 4) |
        ((input[18 + inpos] & 15) << 8) |
        ((input[19 + inpos] & 15) << 12) |
        ((input[20 + inpos] & 15) << 16) |
        ((input[21 + inpos] & 15) << 20) |
        ((input[22 + inpos] & 15) << 24) |
        (input[23 + inpos] << 28);
    output[3 + outpos] =
        (input[24 + inpos] & 15) |
        ((input[25 + inpos] & 15) << 4) |
        ((input[26 + inpos] & 15) << 8) |
        ((input[27 + inpos] & 15) << 12) |
        ((input[28 + inpos] & 15) << 16) |
        ((input[29 + inpos] & 15) << 20) |
        ((input[30 + inpos] & 15) << 24) |
        (input[31 + inpos] << 28);
}

function fastpack5(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 31) |
        ((input[1 + inpos] & 31) << 5) |
        ((input[2 + inpos] & 31) << 10) |
        ((input[3 + inpos] & 31) << 15) |
        ((input[4 + inpos] & 31) << 20) |
        ((input[5 + inpos] & 31) << 25) |
        (input[6 + inpos] << 30);
    output[1 + outpos] =
        ((input[6 + inpos] & 31) >>> (5 - 3)) |
        ((input[7 + inpos] & 31) << 3) |
        ((input[8 + inpos] & 31) << 8) |
        ((input[9 + inpos] & 31) << 13) |
        ((input[10 + inpos] & 31) << 18) |
        ((input[11 + inpos] & 31) << 23) |
        (input[12 + inpos] << 28);
    output[2 + outpos] =
        ((input[12 + inpos] & 31) >>> (5 - 1)) |
        ((input[13 + inpos] & 31) << 1) |
        ((input[14 + inpos] & 31) << 6) |
        ((input[15 + inpos] & 31) << 11) |
        ((input[16 + inpos] & 31) << 16) |
        ((input[17 + inpos] & 31) << 21) |
        ((input[18 + inpos] & 31) << 26) |
        (input[19 + inpos] << 31);
    output[3 + outpos] =
        ((input[19 + inpos] & 31) >>> (5 - 4)) |
        ((input[20 + inpos] & 31) << 4) |
        ((input[21 + inpos] & 31) << 9) |
        ((input[22 + inpos] & 31) << 14) |
        ((input[23 + inpos] & 31) << 19) |
        ((input[24 + inpos] & 31) << 24) |
        (input[25 + inpos] << 29);
    output[4 + outpos] =
        ((input[25 + inpos] & 31) >>> (5 - 2)) |
        ((input[26 + inpos] & 31) << 2) |
        ((input[27 + inpos] & 31) << 7) |
        ((input[28 + inpos] & 31) << 12) |
        ((input[29 + inpos] & 31) << 17) |
        ((input[30 + inpos] & 31) << 22) |
        (input[31 + inpos] << 27);
}

function fastpack6(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 63) |
        ((input[1 + inpos] & 63) << 6) |
        ((input[2 + inpos] & 63) << 12) |
        ((input[3 + inpos] & 63) << 18) |
        ((input[4 + inpos] & 63) << 24) |
        (input[5 + inpos] << 30);
    output[1 + outpos] =
        ((input[5 + inpos] & 63) >>> (6 - 4)) |
        ((input[6 + inpos] & 63) << 4) |
        ((input[7 + inpos] & 63) << 10) |
        ((input[8 + inpos] & 63) << 16) |
        ((input[9 + inpos] & 63) << 22) |
        (input[10 + inpos] << 28);
    output[2 + outpos] =
        ((input[10 + inpos] & 63) >>> (6 - 2)) |
        ((input[11 + inpos] & 63) << 2) |
        ((input[12 + inpos] & 63) << 8) |
        ((input[13 + inpos] & 63) << 14) |
        ((input[14 + inpos] & 63) << 20) |
        (input[15 + inpos] << 26);
    output[3 + outpos] =
        (input[16 + inpos] & 63) |
        ((input[17 + inpos] & 63) << 6) |
        ((input[18 + inpos] & 63) << 12) |
        ((input[19 + inpos] & 63) << 18) |
        ((input[20 + inpos] & 63) << 24) |
        (input[21 + inpos] << 30);
    output[4 + outpos] =
        ((input[21 + inpos] & 63) >>> (6 - 4)) |
        ((input[22 + inpos] & 63) << 4) |
        ((input[23 + inpos] & 63) << 10) |
        ((input[24 + inpos] & 63) << 16) |
        ((input[25 + inpos] & 63) << 22) |
        (input[26 + inpos] << 28);
    output[5 + outpos] =
        ((input[26 + inpos] & 63) >>> (6 - 2)) |
        ((input[27 + inpos] & 63) << 2) |
        ((input[28 + inpos] & 63) << 8) |
        ((input[29 + inpos] & 63) << 14) |
        ((input[30 + inpos] & 63) << 20) |
        (input[31 + inpos] << 26);
}

function fastpack7(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 127) |
        ((input[1 + inpos] & 127) << 7) |
        ((input[2 + inpos] & 127) << 14) |
        ((input[3 + inpos] & 127) << 21) |
        (input[4 + inpos] << 28);
    output[1 + outpos] =
        ((input[4 + inpos] & 127) >>> (7 - 3)) |
        ((input[5 + inpos] & 127) << 3) |
        ((input[6 + inpos] & 127) << 10) |
        ((input[7 + inpos] & 127) << 17) |
        ((input[8 + inpos] & 127) << 24) |
        (input[9 + inpos] << 31);
    output[2 + outpos] =
        ((input[9 + inpos] & 127) >>> (7 - 6)) |
        ((input[10 + inpos] & 127) << 6) |
        ((input[11 + inpos] & 127) << 13) |
        ((input[12 + inpos] & 127) << 20) |
        (input[13 + inpos] << 27);
    output[3 + outpos] =
        ((input[13 + inpos] & 127) >>> (7 - 2)) |
        ((input[14 + inpos] & 127) << 2) |
        ((input[15 + inpos] & 127) << 9) |
        ((input[16 + inpos] & 127) << 16) |
        ((input[17 + inpos] & 127) << 23) |
        (input[18 + inpos] << 30);
    output[4 + outpos] =
        ((input[18 + inpos] & 127) >>> (7 - 5)) |
        ((input[19 + inpos] & 127) << 5) |
        ((input[20 + inpos] & 127) << 12) |
        ((input[21 + inpos] & 127) << 19) |
        (input[22 + inpos] << 26);
    output[5 + outpos] =
        ((input[22 + inpos] & 127) >>> (7 - 1)) |
        ((input[23 + inpos] & 127) << 1) |
        ((input[24 + inpos] & 127) << 8) |
        ((input[25 + inpos] & 127) << 15) |
        ((input[26 + inpos] & 127) << 22) |
        (input[27 + inpos] << 29);
    output[6 + outpos] =
        ((input[27 + inpos] & 127) >>> (7 - 4)) |
        ((input[28 + inpos] & 127) << 4) |
        ((input[29 + inpos] & 127) << 11) |
        ((input[30 + inpos] & 127) << 18) |
        (input[31 + inpos] << 25);
}

function fastpack8(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 255) |
        ((input[1 + inpos] & 255) << 8) |
        ((input[2 + inpos] & 255) << 16) |
        (input[3 + inpos] << 24);
    output[1 + outpos] =
        (input[4 + inpos] & 255) |
        ((input[5 + inpos] & 255) << 8) |
        ((input[6 + inpos] & 255) << 16) |
        (input[7 + inpos] << 24);
    output[2 + outpos] =
        (input[8 + inpos] & 255) |
        ((input[9 + inpos] & 255) << 8) |
        ((input[10 + inpos] & 255) << 16) |
        (input[11 + inpos] << 24);
    output[3 + outpos] =
        (input[12 + inpos] & 255) |
        ((input[13 + inpos] & 255) << 8) |
        ((input[14 + inpos] & 255) << 16) |
        (input[15 + inpos] << 24);
    output[4 + outpos] =
        (input[16 + inpos] & 255) |
        ((input[17 + inpos] & 255) << 8) |
        ((input[18 + inpos] & 255) << 16) |
        (input[19 + inpos] << 24);
    output[5 + outpos] =
        (input[20 + inpos] & 255) |
        ((input[21 + inpos] & 255) << 8) |
        ((input[22 + inpos] & 255) << 16) |
        (input[23 + inpos] << 24);
    output[6 + outpos] =
        (input[24 + inpos] & 255) |
        ((input[25 + inpos] & 255) << 8) |
        ((input[26 + inpos] & 255) << 16) |
        (input[27 + inpos] << 24);
    output[7 + outpos] =
        (input[28 + inpos] & 255) |
        ((input[29 + inpos] & 255) << 8) |
        ((input[30 + inpos] & 255) << 16) |
        (input[31 + inpos] << 24);
}

function fastpack9(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        (input[inpos] & 511) |
        ((input[1 + inpos] & 511) << 9) |
        ((input[2 + inpos] & 511) << 18) |
        (input[3 + inpos] << 27);
    output[1 + outpos] =
        ((input[3 + inpos] & 511) >>> (9 - 4)) |
        ((input[4 + inpos] & 511) << 4) |
        ((input[5 + inpos] & 511) << 13) |
        ((input[6 + inpos] & 511) << 22) |
        (input[7 + inpos] << 31);
    output[2 + outpos] =
        ((input[7 + inpos] & 511) >>> (9 - 8)) |
        ((input[8 + inpos] & 511) << 8) |
        ((input[9 + inpos] & 511) << 17) |
        (input[10 + inpos] << 26);
    output[3 + outpos] =
        ((input[10 + inpos] & 511) >>> (9 - 3)) |
        ((input[11 + inpos] & 511) << 3) |
        ((input[12 + inpos] & 511) << 12) |
        ((input[13 + inpos] & 511) << 21) |
        (input[14 + inpos] << 30);
    output[4 + outpos] =
        ((input[14 + inpos] & 511) >>> (9 - 7)) |
        ((input[15 + inpos] & 511) << 7) |
        ((input[16 + inpos] & 511) << 16) |
        (input[17 + inpos] << 25);
    output[5 + outpos] =
        ((input[17 + inpos] & 511) >>> (9 - 2)) |
        ((input[18 + inpos] & 511) << 2) |
        ((input[19 + inpos] & 511) << 11) |
        ((input[20 + inpos] & 511) << 20) |
        (input[21 + inpos] << 29);
    output[6 + outpos] =
        ((input[21 + inpos] & 511) >>> (9 - 6)) |
        ((input[22 + inpos] & 511) << 6) |
        ((input[23 + inpos] & 511) << 15) |
        (input[24 + inpos] << 24);
    output[7 + outpos] =
        ((input[24 + inpos] & 511) >>> (9 - 1)) |
        ((input[25 + inpos] & 511) << 1) |
        ((input[26 + inpos] & 511) << 10) |
        ((input[27 + inpos] & 511) << 19) |
        (input[28 + inpos] << 28);
    output[8 + outpos] =
        ((input[28 + inpos] & 511) >>> (9 - 5)) |
        ((input[29 + inpos] & 511) << 5) |
        ((input[30 + inpos] & 511) << 14) |
        (input[31 + inpos] << 23);
}

/**
 * Unpack 32 numberegers
 *
 * @param input
 *                source array
 * @param inpos
 *                position in source array
 * @param output
 *                output array
 * @param outpos
 *                position in output array
 * @param bit
 *                number of bits to use per numbereger
 */
export function fastpackwithoutmask(
    input: Uint32Array,
    inpos: number,
    output: Uint32Array,
    outpos: number,
    bit: number,
) {
    switch (bit) {
        case 0:
            fastpackwithoutmask0(input, inpos, output, outpos);
            break;
        case 1:
            fastpackwithoutmask1(input, inpos, output, outpos);
            break;
        case 2:
            fastpackwithoutmask2(input, inpos, output, outpos);
            break;
        case 3:
            fastpackwithoutmask3(input, inpos, output, outpos);
            break;
        case 4:
            fastpackwithoutmask4(input, inpos, output, outpos);
            break;
        case 5:
            fastpackwithoutmask5(input, inpos, output, outpos);
            break;
        case 6:
            fastpackwithoutmask6(input, inpos, output, outpos);
            break;
        case 7:
            fastpackwithoutmask7(input, inpos, output, outpos);
            break;
        case 8:
            fastpackwithoutmask8(input, inpos, output, outpos);
            break;
        case 9:
            fastpackwithoutmask9(input, inpos, output, outpos);
            break;
        case 10:
            fastpackwithoutmask10(input, inpos, output, outpos);
            break;
        case 11:
            fastpackwithoutmask11(input, inpos, output, outpos);
            break;
        case 12:
            fastpackwithoutmask12(input, inpos, output, outpos);
            break;
        case 13:
            fastpackwithoutmask13(input, inpos, output, outpos);
            break;
        case 14:
            fastpackwithoutmask14(input, inpos, output, outpos);
            break;
        case 15:
            fastpackwithoutmask15(input, inpos, output, outpos);
            break;
        case 16:
            fastpackwithoutmask16(input, inpos, output, outpos);
            break;
        case 17:
            fastpackwithoutmask17(input, inpos, output, outpos);
            break;
        case 18:
            fastpackwithoutmask18(input, inpos, output, outpos);
            break;
        case 19:
            fastpackwithoutmask19(input, inpos, output, outpos);
            break;
        case 20:
            fastpackwithoutmask20(input, inpos, output, outpos);
            break;
        case 21:
            fastpackwithoutmask21(input, inpos, output, outpos);
            break;
        case 22:
            fastpackwithoutmask22(input, inpos, output, outpos);
            break;
        case 23:
            fastpackwithoutmask23(input, inpos, output, outpos);
            break;
        case 24:
            fastpackwithoutmask24(input, inpos, output, outpos);
            break;
        case 25:
            fastpackwithoutmask25(input, inpos, output, outpos);
            break;
        case 26:
            fastpackwithoutmask26(input, inpos, output, outpos);
            break;
        case 27:
            fastpackwithoutmask27(input, inpos, output, outpos);
            break;
        case 28:
            fastpackwithoutmask28(input, inpos, output, outpos);
            break;
        case 29:
            fastpackwithoutmask29(input, inpos, output, outpos);
            break;
        case 30:
            fastpackwithoutmask30(input, inpos, output, outpos);
            break;
        case 31:
            fastpackwithoutmask31(input, inpos, output, outpos);
            break;
        case 32:
            fastpackwithoutmask32(input, inpos, output, outpos);
            break;
        default:
            throw new Error("Unsupported bit width.");
    }
}

function fastpackwithoutmask0(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    // nothing
}

function fastpackwithoutmask1(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 1) |
        (input[2 + inpos] << 2) |
        (input[3 + inpos] << 3) |
        (input[4 + inpos] << 4) |
        (input[5 + inpos] << 5) |
        (input[6 + inpos] << 6) |
        (input[7 + inpos] << 7) |
        (input[8 + inpos] << 8) |
        (input[9 + inpos] << 9) |
        (input[10 + inpos] << 10) |
        (input[11 + inpos] << 11) |
        (input[12 + inpos] << 12) |
        (input[13 + inpos] << 13) |
        (input[14 + inpos] << 14) |
        (input[15 + inpos] << 15) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 17) |
        (input[18 + inpos] << 18) |
        (input[19 + inpos] << 19) |
        (input[20 + inpos] << 20) |
        (input[21 + inpos] << 21) |
        (input[22 + inpos] << 22) |
        (input[23 + inpos] << 23) |
        (input[24 + inpos] << 24) |
        (input[25 + inpos] << 25) |
        (input[26 + inpos] << 26) |
        (input[27 + inpos] << 27) |
        (input[28 + inpos] << 28) |
        (input[29 + inpos] << 29) |
        (input[30 + inpos] << 30) |
        (input[31 + inpos] << 31);
}

function fastpackwithoutmask10(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 10) | (input[2 + inpos] << 20) | (input[3 + inpos] << 30);
    output[1 + outpos] =
        (input[3 + inpos] >>> (10 - 8)) | (input[4 + inpos] << 8) | (input[5 + inpos] << 18) | (input[6 + inpos] << 28);
    output[2 + outpos] =
        (input[6 + inpos] >>> (10 - 6)) | (input[7 + inpos] << 6) | (input[8 + inpos] << 16) | (input[9 + inpos] << 26);
    output[3 + outpos] =
        (input[9 + inpos] >>> (10 - 4)) |
        (input[10 + inpos] << 4) |
        (input[11 + inpos] << 14) |
        (input[12 + inpos] << 24);
    output[4 + outpos] =
        (input[12 + inpos] >>> (10 - 2)) |
        (input[13 + inpos] << 2) |
        (input[14 + inpos] << 12) |
        (input[15 + inpos] << 22);
    output[5 + outpos] =
        input[16 + inpos] | (input[17 + inpos] << 10) | (input[18 + inpos] << 20) | (input[19 + inpos] << 30);
    output[6 + outpos] =
        (input[19 + inpos] >>> (10 - 8)) |
        (input[20 + inpos] << 8) |
        (input[21 + inpos] << 18) |
        (input[22 + inpos] << 28);
    output[7 + outpos] =
        (input[22 + inpos] >>> (10 - 6)) |
        (input[23 + inpos] << 6) |
        (input[24 + inpos] << 16) |
        (input[25 + inpos] << 26);
    output[8 + outpos] =
        (input[25 + inpos] >>> (10 - 4)) |
        (input[26 + inpos] << 4) |
        (input[27 + inpos] << 14) |
        (input[28 + inpos] << 24);
    output[9 + outpos] =
        (input[28 + inpos] >>> (10 - 2)) |
        (input[29 + inpos] << 2) |
        (input[30 + inpos] << 12) |
        (input[31 + inpos] << 22);
}

function fastpackwithoutmask11(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 11) | (input[2 + inpos] << 22);
    output[1 + outpos] =
        (input[2 + inpos] >>> (11 - 1)) | (input[3 + inpos] << 1) | (input[4 + inpos] << 12) | (input[5 + inpos] << 23);
    output[2 + outpos] =
        (input[5 + inpos] >>> (11 - 2)) | (input[6 + inpos] << 2) | (input[7 + inpos] << 13) | (input[8 + inpos] << 24);
    output[3 + outpos] =
        (input[8 + inpos] >>> (11 - 3)) |
        (input[9 + inpos] << 3) |
        (input[10 + inpos] << 14) |
        (input[11 + inpos] << 25);
    output[4 + outpos] =
        (input[11 + inpos] >>> (11 - 4)) |
        (input[12 + inpos] << 4) |
        (input[13 + inpos] << 15) |
        (input[14 + inpos] << 26);
    output[5 + outpos] =
        (input[14 + inpos] >>> (11 - 5)) |
        (input[15 + inpos] << 5) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 27);
    output[6 + outpos] =
        (input[17 + inpos] >>> (11 - 6)) |
        (input[18 + inpos] << 6) |
        (input[19 + inpos] << 17) |
        (input[20 + inpos] << 28);
    output[7 + outpos] =
        (input[20 + inpos] >>> (11 - 7)) |
        (input[21 + inpos] << 7) |
        (input[22 + inpos] << 18) |
        (input[23 + inpos] << 29);
    output[8 + outpos] =
        (input[23 + inpos] >>> (11 - 8)) |
        (input[24 + inpos] << 8) |
        (input[25 + inpos] << 19) |
        (input[26 + inpos] << 30);
    output[9 + outpos] =
        (input[26 + inpos] >>> (11 - 9)) |
        (input[27 + inpos] << 9) |
        (input[28 + inpos] << 20) |
        (input[29 + inpos] << 31);
    output[10 + outpos] = (input[29 + inpos] >>> (11 - 10)) | (input[30 + inpos] << 10) | (input[31 + inpos] << 21);
}

function fastpackwithoutmask12(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 12) | (input[2 + inpos] << 24);
    output[1 + outpos] =
        (input[2 + inpos] >>> (12 - 4)) | (input[3 + inpos] << 4) | (input[4 + inpos] << 16) | (input[5 + inpos] << 28);
    output[2 + outpos] = (input[5 + inpos] >>> (12 - 8)) | (input[6 + inpos] << 8) | (input[7 + inpos] << 20);
    output[3 + outpos] = input[8 + inpos] | (input[9 + inpos] << 12) | (input[10 + inpos] << 24);
    output[4 + outpos] =
        (input[10 + inpos] >>> (12 - 4)) |
        (input[11 + inpos] << 4) |
        (input[12 + inpos] << 16) |
        (input[13 + inpos] << 28);
    output[5 + outpos] = (input[13 + inpos] >>> (12 - 8)) | (input[14 + inpos] << 8) | (input[15 + inpos] << 20);
    output[6 + outpos] = input[16 + inpos] | (input[17 + inpos] << 12) | (input[18 + inpos] << 24);
    output[7 + outpos] =
        (input[18 + inpos] >>> (12 - 4)) |
        (input[19 + inpos] << 4) |
        (input[20 + inpos] << 16) |
        (input[21 + inpos] << 28);
    output[8 + outpos] = (input[21 + inpos] >>> (12 - 8)) | (input[22 + inpos] << 8) | (input[23 + inpos] << 20);
    output[9 + outpos] = input[24 + inpos] | (input[25 + inpos] << 12) | (input[26 + inpos] << 24);
    output[10 + outpos] =
        (input[26 + inpos] >>> (12 - 4)) |
        (input[27 + inpos] << 4) |
        (input[28 + inpos] << 16) |
        (input[29 + inpos] << 28);
    output[11 + outpos] = (input[29 + inpos] >>> (12 - 8)) | (input[30 + inpos] << 8) | (input[31 + inpos] << 20);
}

function fastpackwithoutmask13(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 13) | (input[2 + inpos] << 26);
    output[1 + outpos] = (input[2 + inpos] >>> (13 - 7)) | (input[3 + inpos] << 7) | (input[4 + inpos] << 20);
    output[2 + outpos] =
        (input[4 + inpos] >>> (13 - 1)) | (input[5 + inpos] << 1) | (input[6 + inpos] << 14) | (input[7 + inpos] << 27);
    output[3 + outpos] = (input[7 + inpos] >>> (13 - 8)) | (input[8 + inpos] << 8) | (input[9 + inpos] << 21);
    output[4 + outpos] =
        (input[9 + inpos] >>> (13 - 2)) |
        (input[10 + inpos] << 2) |
        (input[11 + inpos] << 15) |
        (input[12 + inpos] << 28);
    output[5 + outpos] = (input[12 + inpos] >>> (13 - 9)) | (input[13 + inpos] << 9) | (input[14 + inpos] << 22);
    output[6 + outpos] =
        (input[14 + inpos] >>> (13 - 3)) |
        (input[15 + inpos] << 3) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 29);
    output[7 + outpos] = (input[17 + inpos] >>> (13 - 10)) | (input[18 + inpos] << 10) | (input[19 + inpos] << 23);
    output[8 + outpos] =
        (input[19 + inpos] >>> (13 - 4)) |
        (input[20 + inpos] << 4) |
        (input[21 + inpos] << 17) |
        (input[22 + inpos] << 30);
    output[9 + outpos] = (input[22 + inpos] >>> (13 - 11)) | (input[23 + inpos] << 11) | (input[24 + inpos] << 24);
    output[10 + outpos] =
        (input[24 + inpos] >>> (13 - 5)) |
        (input[25 + inpos] << 5) |
        (input[26 + inpos] << 18) |
        (input[27 + inpos] << 31);
    output[11 + outpos] = (input[27 + inpos] >>> (13 - 12)) | (input[28 + inpos] << 12) | (input[29 + inpos] << 25);
    output[12 + outpos] = (input[29 + inpos] >>> (13 - 6)) | (input[30 + inpos] << 6) | (input[31 + inpos] << 19);
}

function fastpackwithoutmask14(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 14) | (input[2 + inpos] << 28);
    output[1 + outpos] = (input[2 + inpos] >>> (14 - 10)) | (input[3 + inpos] << 10) | (input[4 + inpos] << 24);
    output[2 + outpos] = (input[4 + inpos] >>> (14 - 6)) | (input[5 + inpos] << 6) | (input[6 + inpos] << 20);
    output[3 + outpos] =
        (input[6 + inpos] >>> (14 - 2)) | (input[7 + inpos] << 2) | (input[8 + inpos] << 16) | (input[9 + inpos] << 30);
    output[4 + outpos] = (input[9 + inpos] >>> (14 - 12)) | (input[10 + inpos] << 12) | (input[11 + inpos] << 26);
    output[5 + outpos] = (input[11 + inpos] >>> (14 - 8)) | (input[12 + inpos] << 8) | (input[13 + inpos] << 22);
    output[6 + outpos] = (input[13 + inpos] >>> (14 - 4)) | (input[14 + inpos] << 4) | (input[15 + inpos] << 18);
    output[7 + outpos] = input[16 + inpos] | (input[17 + inpos] << 14) | (input[18 + inpos] << 28);
    output[8 + outpos] = (input[18 + inpos] >>> (14 - 10)) | (input[19 + inpos] << 10) | (input[20 + inpos] << 24);
    output[9 + outpos] = (input[20 + inpos] >>> (14 - 6)) | (input[21 + inpos] << 6) | (input[22 + inpos] << 20);
    output[10 + outpos] =
        (input[22 + inpos] >>> (14 - 2)) |
        (input[23 + inpos] << 2) |
        (input[24 + inpos] << 16) |
        (input[25 + inpos] << 30);
    output[11 + outpos] = (input[25 + inpos] >>> (14 - 12)) | (input[26 + inpos] << 12) | (input[27 + inpos] << 26);
    output[12 + outpos] = (input[27 + inpos] >>> (14 - 8)) | (input[28 + inpos] << 8) | (input[29 + inpos] << 22);
    output[13 + outpos] = (input[29 + inpos] >>> (14 - 4)) | (input[30 + inpos] << 4) | (input[31 + inpos] << 18);
}

function fastpackwithoutmask15(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 15) | (input[2 + inpos] << 30);
    output[1 + outpos] = (input[2 + inpos] >>> (15 - 13)) | (input[3 + inpos] << 13) | (input[4 + inpos] << 28);
    output[2 + outpos] = (input[4 + inpos] >>> (15 - 11)) | (input[5 + inpos] << 11) | (input[6 + inpos] << 26);
    output[3 + outpos] = (input[6 + inpos] >>> (15 - 9)) | (input[7 + inpos] << 9) | (input[8 + inpos] << 24);
    output[4 + outpos] = (input[8 + inpos] >>> (15 - 7)) | (input[9 + inpos] << 7) | (input[10 + inpos] << 22);
    output[5 + outpos] = (input[10 + inpos] >>> (15 - 5)) | (input[11 + inpos] << 5) | (input[12 + inpos] << 20);
    output[6 + outpos] = (input[12 + inpos] >>> (15 - 3)) | (input[13 + inpos] << 3) | (input[14 + inpos] << 18);
    output[7 + outpos] =
        (input[14 + inpos] >>> (15 - 1)) |
        (input[15 + inpos] << 1) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 31);
    output[8 + outpos] = (input[17 + inpos] >>> (15 - 14)) | (input[18 + inpos] << 14) | (input[19 + inpos] << 29);
    output[9 + outpos] = (input[19 + inpos] >>> (15 - 12)) | (input[20 + inpos] << 12) | (input[21 + inpos] << 27);
    output[10 + outpos] = (input[21 + inpos] >>> (15 - 10)) | (input[22 + inpos] << 10) | (input[23 + inpos] << 25);
    output[11 + outpos] = (input[23 + inpos] >>> (15 - 8)) | (input[24 + inpos] << 8) | (input[25 + inpos] << 23);
    output[12 + outpos] = (input[25 + inpos] >>> (15 - 6)) | (input[26 + inpos] << 6) | (input[27 + inpos] << 21);
    output[13 + outpos] = (input[27 + inpos] >>> (15 - 4)) | (input[28 + inpos] << 4) | (input[29 + inpos] << 19);
    output[14 + outpos] = (input[29 + inpos] >>> (15 - 2)) | (input[30 + inpos] << 2) | (input[31 + inpos] << 17);
}

function fastpackwithoutmask16(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 16);
    output[1 + outpos] = input[2 + inpos] | (input[3 + inpos] << 16);
    output[2 + outpos] = input[4 + inpos] | (input[5 + inpos] << 16);
    output[3 + outpos] = input[6 + inpos] | (input[7 + inpos] << 16);
    output[4 + outpos] = input[8 + inpos] | (input[9 + inpos] << 16);
    output[5 + outpos] = input[10 + inpos] | (input[11 + inpos] << 16);
    output[6 + outpos] = input[12 + inpos] | (input[13 + inpos] << 16);
    output[7 + outpos] = input[14 + inpos] | (input[15 + inpos] << 16);
    output[8 + outpos] = input[16 + inpos] | (input[17 + inpos] << 16);
    output[9 + outpos] = input[18 + inpos] | (input[19 + inpos] << 16);
    output[10 + outpos] = input[20 + inpos] | (input[21 + inpos] << 16);
    output[11 + outpos] = input[22 + inpos] | (input[23 + inpos] << 16);
    output[12 + outpos] = input[24 + inpos] | (input[25 + inpos] << 16);
    output[13 + outpos] = input[26 + inpos] | (input[27 + inpos] << 16);
    output[14 + outpos] = input[28 + inpos] | (input[29 + inpos] << 16);
    output[15 + outpos] = input[30 + inpos] | (input[31 + inpos] << 16);
}

function fastpackwithoutmask17(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 17);
    output[1 + outpos] = (input[1 + inpos] >>> (17 - 2)) | (input[2 + inpos] << 2) | (input[3 + inpos] << 19);
    output[2 + outpos] = (input[3 + inpos] >>> (17 - 4)) | (input[4 + inpos] << 4) | (input[5 + inpos] << 21);
    output[3 + outpos] = (input[5 + inpos] >>> (17 - 6)) | (input[6 + inpos] << 6) | (input[7 + inpos] << 23);
    output[4 + outpos] = (input[7 + inpos] >>> (17 - 8)) | (input[8 + inpos] << 8) | (input[9 + inpos] << 25);
    output[5 + outpos] = (input[9 + inpos] >>> (17 - 10)) | (input[10 + inpos] << 10) | (input[11 + inpos] << 27);
    output[6 + outpos] = (input[11 + inpos] >>> (17 - 12)) | (input[12 + inpos] << 12) | (input[13 + inpos] << 29);
    output[7 + outpos] = (input[13 + inpos] >>> (17 - 14)) | (input[14 + inpos] << 14) | (input[15 + inpos] << 31);
    output[8 + outpos] = (input[15 + inpos] >>> (17 - 16)) | (input[16 + inpos] << 16);
    output[9 + outpos] = (input[16 + inpos] >>> (17 - 1)) | (input[17 + inpos] << 1) | (input[18 + inpos] << 18);
    output[10 + outpos] = (input[18 + inpos] >>> (17 - 3)) | (input[19 + inpos] << 3) | (input[20 + inpos] << 20);
    output[11 + outpos] = (input[20 + inpos] >>> (17 - 5)) | (input[21 + inpos] << 5) | (input[22 + inpos] << 22);
    output[12 + outpos] = (input[22 + inpos] >>> (17 - 7)) | (input[23 + inpos] << 7) | (input[24 + inpos] << 24);
    output[13 + outpos] = (input[24 + inpos] >>> (17 - 9)) | (input[25 + inpos] << 9) | (input[26 + inpos] << 26);
    output[14 + outpos] = (input[26 + inpos] >>> (17 - 11)) | (input[27 + inpos] << 11) | (input[28 + inpos] << 28);
    output[15 + outpos] = (input[28 + inpos] >>> (17 - 13)) | (input[29 + inpos] << 13) | (input[30 + inpos] << 30);
    output[16 + outpos] = (input[30 + inpos] >>> (17 - 15)) | (input[31 + inpos] << 15);
}

function fastpackwithoutmask18(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 18);
    output[1 + outpos] = (input[1 + inpos] >>> (18 - 4)) | (input[2 + inpos] << 4) | (input[3 + inpos] << 22);
    output[2 + outpos] = (input[3 + inpos] >>> (18 - 8)) | (input[4 + inpos] << 8) | (input[5 + inpos] << 26);
    output[3 + outpos] = (input[5 + inpos] >>> (18 - 12)) | (input[6 + inpos] << 12) | (input[7 + inpos] << 30);
    output[4 + outpos] = (input[7 + inpos] >>> (18 - 16)) | (input[8 + inpos] << 16);
    output[5 + outpos] = (input[8 + inpos] >>> (18 - 2)) | (input[9 + inpos] << 2) | (input[10 + inpos] << 20);
    output[6 + outpos] = (input[10 + inpos] >>> (18 - 6)) | (input[11 + inpos] << 6) | (input[12 + inpos] << 24);
    output[7 + outpos] = (input[12 + inpos] >>> (18 - 10)) | (input[13 + inpos] << 10) | (input[14 + inpos] << 28);
    output[8 + outpos] = (input[14 + inpos] >>> (18 - 14)) | (input[15 + inpos] << 14);
    output[9 + outpos] = input[16 + inpos] | (input[17 + inpos] << 18);
    output[10 + outpos] = (input[17 + inpos] >>> (18 - 4)) | (input[18 + inpos] << 4) | (input[19 + inpos] << 22);
    output[11 + outpos] = (input[19 + inpos] >>> (18 - 8)) | (input[20 + inpos] << 8) | (input[21 + inpos] << 26);
    output[12 + outpos] = (input[21 + inpos] >>> (18 - 12)) | (input[22 + inpos] << 12) | (input[23 + inpos] << 30);
    output[13 + outpos] = (input[23 + inpos] >>> (18 - 16)) | (input[24 + inpos] << 16);
    output[14 + outpos] = (input[24 + inpos] >>> (18 - 2)) | (input[25 + inpos] << 2) | (input[26 + inpos] << 20);
    output[15 + outpos] = (input[26 + inpos] >>> (18 - 6)) | (input[27 + inpos] << 6) | (input[28 + inpos] << 24);
    output[16 + outpos] = (input[28 + inpos] >>> (18 - 10)) | (input[29 + inpos] << 10) | (input[30 + inpos] << 28);
    output[17 + outpos] = (input[30 + inpos] >>> (18 - 14)) | (input[31 + inpos] << 14);
}

function fastpackwithoutmask19(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 19);
    output[1 + outpos] = (input[1 + inpos] >>> (19 - 6)) | (input[2 + inpos] << 6) | (input[3 + inpos] << 25);
    output[2 + outpos] = (input[3 + inpos] >>> (19 - 12)) | (input[4 + inpos] << 12) | (input[5 + inpos] << 31);
    output[3 + outpos] = (input[5 + inpos] >>> (19 - 18)) | (input[6 + inpos] << 18);
    output[4 + outpos] = (input[6 + inpos] >>> (19 - 5)) | (input[7 + inpos] << 5) | (input[8 + inpos] << 24);
    output[5 + outpos] = (input[8 + inpos] >>> (19 - 11)) | (input[9 + inpos] << 11) | (input[10 + inpos] << 30);
    output[6 + outpos] = (input[10 + inpos] >>> (19 - 17)) | (input[11 + inpos] << 17);
    output[7 + outpos] = (input[11 + inpos] >>> (19 - 4)) | (input[12 + inpos] << 4) | (input[13 + inpos] << 23);
    output[8 + outpos] = (input[13 + inpos] >>> (19 - 10)) | (input[14 + inpos] << 10) | (input[15 + inpos] << 29);
    output[9 + outpos] = (input[15 + inpos] >>> (19 - 16)) | (input[16 + inpos] << 16);
    output[10 + outpos] = (input[16 + inpos] >>> (19 - 3)) | (input[17 + inpos] << 3) | (input[18 + inpos] << 22);
    output[11 + outpos] = (input[18 + inpos] >>> (19 - 9)) | (input[19 + inpos] << 9) | (input[20 + inpos] << 28);
    output[12 + outpos] = (input[20 + inpos] >>> (19 - 15)) | (input[21 + inpos] << 15);
    output[13 + outpos] = (input[21 + inpos] >>> (19 - 2)) | (input[22 + inpos] << 2) | (input[23 + inpos] << 21);
    output[14 + outpos] = (input[23 + inpos] >>> (19 - 8)) | (input[24 + inpos] << 8) | (input[25 + inpos] << 27);
    output[15 + outpos] = (input[25 + inpos] >>> (19 - 14)) | (input[26 + inpos] << 14);
    output[16 + outpos] = (input[26 + inpos] >>> (19 - 1)) | (input[27 + inpos] << 1) | (input[28 + inpos] << 20);
    output[17 + outpos] = (input[28 + inpos] >>> (19 - 7)) | (input[29 + inpos] << 7) | (input[30 + inpos] << 26);
    output[18 + outpos] = (input[30 + inpos] >>> (19 - 13)) | (input[31 + inpos] << 13);
}

function fastpackwithoutmask2(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 2) |
        (input[2 + inpos] << 4) |
        (input[3 + inpos] << 6) |
        (input[4 + inpos] << 8) |
        (input[5 + inpos] << 10) |
        (input[6 + inpos] << 12) |
        (input[7 + inpos] << 14) |
        (input[8 + inpos] << 16) |
        (input[9 + inpos] << 18) |
        (input[10 + inpos] << 20) |
        (input[11 + inpos] << 22) |
        (input[12 + inpos] << 24) |
        (input[13 + inpos] << 26) |
        (input[14 + inpos] << 28) |
        (input[15 + inpos] << 30);
    output[1 + outpos] =
        input[16 + inpos] |
        (input[17 + inpos] << 2) |
        (input[18 + inpos] << 4) |
        (input[19 + inpos] << 6) |
        (input[20 + inpos] << 8) |
        (input[21 + inpos] << 10) |
        (input[22 + inpos] << 12) |
        (input[23 + inpos] << 14) |
        (input[24 + inpos] << 16) |
        (input[25 + inpos] << 18) |
        (input[26 + inpos] << 20) |
        (input[27 + inpos] << 22) |
        (input[28 + inpos] << 24) |
        (input[29 + inpos] << 26) |
        (input[30 + inpos] << 28) |
        (input[31 + inpos] << 30);
}

function fastpackwithoutmask20(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 20);
    output[1 + outpos] = (input[1 + inpos] >>> (20 - 8)) | (input[2 + inpos] << 8) | (input[3 + inpos] << 28);
    output[2 + outpos] = (input[3 + inpos] >>> (20 - 16)) | (input[4 + inpos] << 16);
    output[3 + outpos] = (input[4 + inpos] >>> (20 - 4)) | (input[5 + inpos] << 4) | (input[6 + inpos] << 24);
    output[4 + outpos] = (input[6 + inpos] >>> (20 - 12)) | (input[7 + inpos] << 12);
    output[5 + outpos] = input[8 + inpos] | (input[9 + inpos] << 20);
    output[6 + outpos] = (input[9 + inpos] >>> (20 - 8)) | (input[10 + inpos] << 8) | (input[11 + inpos] << 28);
    output[7 + outpos] = (input[11 + inpos] >>> (20 - 16)) | (input[12 + inpos] << 16);
    output[8 + outpos] = (input[12 + inpos] >>> (20 - 4)) | (input[13 + inpos] << 4) | (input[14 + inpos] << 24);
    output[9 + outpos] = (input[14 + inpos] >>> (20 - 12)) | (input[15 + inpos] << 12);
    output[10 + outpos] = input[16 + inpos] | (input[17 + inpos] << 20);
    output[11 + outpos] = (input[17 + inpos] >>> (20 - 8)) | (input[18 + inpos] << 8) | (input[19 + inpos] << 28);
    output[12 + outpos] = (input[19 + inpos] >>> (20 - 16)) | (input[20 + inpos] << 16);
    output[13 + outpos] = (input[20 + inpos] >>> (20 - 4)) | (input[21 + inpos] << 4) | (input[22 + inpos] << 24);
    output[14 + outpos] = (input[22 + inpos] >>> (20 - 12)) | (input[23 + inpos] << 12);
    output[15 + outpos] = input[24 + inpos] | (input[25 + inpos] << 20);
    output[16 + outpos] = (input[25 + inpos] >>> (20 - 8)) | (input[26 + inpos] << 8) | (input[27 + inpos] << 28);
    output[17 + outpos] = (input[27 + inpos] >>> (20 - 16)) | (input[28 + inpos] << 16);
    output[18 + outpos] = (input[28 + inpos] >>> (20 - 4)) | (input[29 + inpos] << 4) | (input[30 + inpos] << 24);
    output[19 + outpos] = (input[30 + inpos] >>> (20 - 12)) | (input[31 + inpos] << 12);
}

function fastpackwithoutmask21(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 21);
    output[1 + outpos] = (input[1 + inpos] >>> (21 - 10)) | (input[2 + inpos] << 10) | (input[3 + inpos] << 31);
    output[2 + outpos] = (input[3 + inpos] >>> (21 - 20)) | (input[4 + inpos] << 20);
    output[3 + outpos] = (input[4 + inpos] >>> (21 - 9)) | (input[5 + inpos] << 9) | (input[6 + inpos] << 30);
    output[4 + outpos] = (input[6 + inpos] >>> (21 - 19)) | (input[7 + inpos] << 19);
    output[5 + outpos] = (input[7 + inpos] >>> (21 - 8)) | (input[8 + inpos] << 8) | (input[9 + inpos] << 29);
    output[6 + outpos] = (input[9 + inpos] >>> (21 - 18)) | (input[10 + inpos] << 18);
    output[7 + outpos] = (input[10 + inpos] >>> (21 - 7)) | (input[11 + inpos] << 7) | (input[12 + inpos] << 28);
    output[8 + outpos] = (input[12 + inpos] >>> (21 - 17)) | (input[13 + inpos] << 17);
    output[9 + outpos] = (input[13 + inpos] >>> (21 - 6)) | (input[14 + inpos] << 6) | (input[15 + inpos] << 27);
    output[10 + outpos] = (input[15 + inpos] >>> (21 - 16)) | (input[16 + inpos] << 16);
    output[11 + outpos] = (input[16 + inpos] >>> (21 - 5)) | (input[17 + inpos] << 5) | (input[18 + inpos] << 26);
    output[12 + outpos] = (input[18 + inpos] >>> (21 - 15)) | (input[19 + inpos] << 15);
    output[13 + outpos] = (input[19 + inpos] >>> (21 - 4)) | (input[20 + inpos] << 4) | (input[21 + inpos] << 25);
    output[14 + outpos] = (input[21 + inpos] >>> (21 - 14)) | (input[22 + inpos] << 14);
    output[15 + outpos] = (input[22 + inpos] >>> (21 - 3)) | (input[23 + inpos] << 3) | (input[24 + inpos] << 24);
    output[16 + outpos] = (input[24 + inpos] >>> (21 - 13)) | (input[25 + inpos] << 13);
    output[17 + outpos] = (input[25 + inpos] >>> (21 - 2)) | (input[26 + inpos] << 2) | (input[27 + inpos] << 23);
    output[18 + outpos] = (input[27 + inpos] >>> (21 - 12)) | (input[28 + inpos] << 12);
    output[19 + outpos] = (input[28 + inpos] >>> (21 - 1)) | (input[29 + inpos] << 1) | (input[30 + inpos] << 22);
    output[20 + outpos] = (input[30 + inpos] >>> (21 - 11)) | (input[31 + inpos] << 11);
}

function fastpackwithoutmask22(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 22);
    output[1 + outpos] = (input[1 + inpos] >>> (22 - 12)) | (input[2 + inpos] << 12);
    output[2 + outpos] = (input[2 + inpos] >>> (22 - 2)) | (input[3 + inpos] << 2) | (input[4 + inpos] << 24);
    output[3 + outpos] = (input[4 + inpos] >>> (22 - 14)) | (input[5 + inpos] << 14);
    output[4 + outpos] = (input[5 + inpos] >>> (22 - 4)) | (input[6 + inpos] << 4) | (input[7 + inpos] << 26);
    output[5 + outpos] = (input[7 + inpos] >>> (22 - 16)) | (input[8 + inpos] << 16);
    output[6 + outpos] = (input[8 + inpos] >>> (22 - 6)) | (input[9 + inpos] << 6) | (input[10 + inpos] << 28);
    output[7 + outpos] = (input[10 + inpos] >>> (22 - 18)) | (input[11 + inpos] << 18);
    output[8 + outpos] = (input[11 + inpos] >>> (22 - 8)) | (input[12 + inpos] << 8) | (input[13 + inpos] << 30);
    output[9 + outpos] = (input[13 + inpos] >>> (22 - 20)) | (input[14 + inpos] << 20);
    output[10 + outpos] = (input[14 + inpos] >>> (22 - 10)) | (input[15 + inpos] << 10);
    output[11 + outpos] = input[16 + inpos] | (input[17 + inpos] << 22);
    output[12 + outpos] = (input[17 + inpos] >>> (22 - 12)) | (input[18 + inpos] << 12);
    output[13 + outpos] = (input[18 + inpos] >>> (22 - 2)) | (input[19 + inpos] << 2) | (input[20 + inpos] << 24);
    output[14 + outpos] = (input[20 + inpos] >>> (22 - 14)) | (input[21 + inpos] << 14);
    output[15 + outpos] = (input[21 + inpos] >>> (22 - 4)) | (input[22 + inpos] << 4) | (input[23 + inpos] << 26);
    output[16 + outpos] = (input[23 + inpos] >>> (22 - 16)) | (input[24 + inpos] << 16);
    output[17 + outpos] = (input[24 + inpos] >>> (22 - 6)) | (input[25 + inpos] << 6) | (input[26 + inpos] << 28);
    output[18 + outpos] = (input[26 + inpos] >>> (22 - 18)) | (input[27 + inpos] << 18);
    output[19 + outpos] = (input[27 + inpos] >>> (22 - 8)) | (input[28 + inpos] << 8) | (input[29 + inpos] << 30);
    output[20 + outpos] = (input[29 + inpos] >>> (22 - 20)) | (input[30 + inpos] << 20);
    output[21 + outpos] = (input[30 + inpos] >>> (22 - 10)) | (input[31 + inpos] << 10);
}

function fastpackwithoutmask23(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 23);
    output[1 + outpos] = (input[1 + inpos] >>> (23 - 14)) | (input[2 + inpos] << 14);
    output[2 + outpos] = (input[2 + inpos] >>> (23 - 5)) | (input[3 + inpos] << 5) | (input[4 + inpos] << 28);
    output[3 + outpos] = (input[4 + inpos] >>> (23 - 19)) | (input[5 + inpos] << 19);
    output[4 + outpos] = (input[5 + inpos] >>> (23 - 10)) | (input[6 + inpos] << 10);
    output[5 + outpos] = (input[6 + inpos] >>> (23 - 1)) | (input[7 + inpos] << 1) | (input[8 + inpos] << 24);
    output[6 + outpos] = (input[8 + inpos] >>> (23 - 15)) | (input[9 + inpos] << 15);
    output[7 + outpos] = (input[9 + inpos] >>> (23 - 6)) | (input[10 + inpos] << 6) | (input[11 + inpos] << 29);
    output[8 + outpos] = (input[11 + inpos] >>> (23 - 20)) | (input[12 + inpos] << 20);
    output[9 + outpos] = (input[12 + inpos] >>> (23 - 11)) | (input[13 + inpos] << 11);
    output[10 + outpos] = (input[13 + inpos] >>> (23 - 2)) | (input[14 + inpos] << 2) | (input[15 + inpos] << 25);
    output[11 + outpos] = (input[15 + inpos] >>> (23 - 16)) | (input[16 + inpos] << 16);
    output[12 + outpos] = (input[16 + inpos] >>> (23 - 7)) | (input[17 + inpos] << 7) | (input[18 + inpos] << 30);
    output[13 + outpos] = (input[18 + inpos] >>> (23 - 21)) | (input[19 + inpos] << 21);
    output[14 + outpos] = (input[19 + inpos] >>> (23 - 12)) | (input[20 + inpos] << 12);
    output[15 + outpos] = (input[20 + inpos] >>> (23 - 3)) | (input[21 + inpos] << 3) | (input[22 + inpos] << 26);
    output[16 + outpos] = (input[22 + inpos] >>> (23 - 17)) | (input[23 + inpos] << 17);
    output[17 + outpos] = (input[23 + inpos] >>> (23 - 8)) | (input[24 + inpos] << 8) | (input[25 + inpos] << 31);
    output[18 + outpos] = (input[25 + inpos] >>> (23 - 22)) | (input[26 + inpos] << 22);
    output[19 + outpos] = (input[26 + inpos] >>> (23 - 13)) | (input[27 + inpos] << 13);
    output[20 + outpos] = (input[27 + inpos] >>> (23 - 4)) | (input[28 + inpos] << 4) | (input[29 + inpos] << 27);
    output[21 + outpos] = (input[29 + inpos] >>> (23 - 18)) | (input[30 + inpos] << 18);
    output[22 + outpos] = (input[30 + inpos] >>> (23 - 9)) | (input[31 + inpos] << 9);
}

function fastpackwithoutmask24(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 24);
    output[1 + outpos] = (input[1 + inpos] >>> (24 - 16)) | (input[2 + inpos] << 16);
    output[2 + outpos] = (input[2 + inpos] >>> (24 - 8)) | (input[3 + inpos] << 8);
    output[3 + outpos] = input[4 + inpos] | (input[5 + inpos] << 24);
    output[4 + outpos] = (input[5 + inpos] >>> (24 - 16)) | (input[6 + inpos] << 16);
    output[5 + outpos] = (input[6 + inpos] >>> (24 - 8)) | (input[7 + inpos] << 8);
    output[6 + outpos] = input[8 + inpos] | (input[9 + inpos] << 24);
    output[7 + outpos] = (input[9 + inpos] >>> (24 - 16)) | (input[10 + inpos] << 16);
    output[8 + outpos] = (input[10 + inpos] >>> (24 - 8)) | (input[11 + inpos] << 8);
    output[9 + outpos] = input[12 + inpos] | (input[13 + inpos] << 24);
    output[10 + outpos] = (input[13 + inpos] >>> (24 - 16)) | (input[14 + inpos] << 16);
    output[11 + outpos] = (input[14 + inpos] >>> (24 - 8)) | (input[15 + inpos] << 8);
    output[12 + outpos] = input[16 + inpos] | (input[17 + inpos] << 24);
    output[13 + outpos] = (input[17 + inpos] >>> (24 - 16)) | (input[18 + inpos] << 16);
    output[14 + outpos] = (input[18 + inpos] >>> (24 - 8)) | (input[19 + inpos] << 8);
    output[15 + outpos] = input[20 + inpos] | (input[21 + inpos] << 24);
    output[16 + outpos] = (input[21 + inpos] >>> (24 - 16)) | (input[22 + inpos] << 16);
    output[17 + outpos] = (input[22 + inpos] >>> (24 - 8)) | (input[23 + inpos] << 8);
    output[18 + outpos] = input[24 + inpos] | (input[25 + inpos] << 24);
    output[19 + outpos] = (input[25 + inpos] >>> (24 - 16)) | (input[26 + inpos] << 16);
    output[20 + outpos] = (input[26 + inpos] >>> (24 - 8)) | (input[27 + inpos] << 8);
    output[21 + outpos] = input[28 + inpos] | (input[29 + inpos] << 24);
    output[22 + outpos] = (input[29 + inpos] >>> (24 - 16)) | (input[30 + inpos] << 16);
    output[23 + outpos] = (input[30 + inpos] >>> (24 - 8)) | (input[31 + inpos] << 8);
}

function fastpackwithoutmask25(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 25);
    output[1 + outpos] = (input[1 + inpos] >>> (25 - 18)) | (input[2 + inpos] << 18);
    output[2 + outpos] = (input[2 + inpos] >>> (25 - 11)) | (input[3 + inpos] << 11);
    output[3 + outpos] = (input[3 + inpos] >>> (25 - 4)) | (input[4 + inpos] << 4) | (input[5 + inpos] << 29);
    output[4 + outpos] = (input[5 + inpos] >>> (25 - 22)) | (input[6 + inpos] << 22);
    output[5 + outpos] = (input[6 + inpos] >>> (25 - 15)) | (input[7 + inpos] << 15);
    output[6 + outpos] = (input[7 + inpos] >>> (25 - 8)) | (input[8 + inpos] << 8);
    output[7 + outpos] = (input[8 + inpos] >>> (25 - 1)) | (input[9 + inpos] << 1) | (input[10 + inpos] << 26);
    output[8 + outpos] = (input[10 + inpos] >>> (25 - 19)) | (input[11 + inpos] << 19);
    output[9 + outpos] = (input[11 + inpos] >>> (25 - 12)) | (input[12 + inpos] << 12);
    output[10 + outpos] = (input[12 + inpos] >>> (25 - 5)) | (input[13 + inpos] << 5) | (input[14 + inpos] << 30);
    output[11 + outpos] = (input[14 + inpos] >>> (25 - 23)) | (input[15 + inpos] << 23);
    output[12 + outpos] = (input[15 + inpos] >>> (25 - 16)) | (input[16 + inpos] << 16);
    output[13 + outpos] = (input[16 + inpos] >>> (25 - 9)) | (input[17 + inpos] << 9);
    output[14 + outpos] = (input[17 + inpos] >>> (25 - 2)) | (input[18 + inpos] << 2) | (input[19 + inpos] << 27);
    output[15 + outpos] = (input[19 + inpos] >>> (25 - 20)) | (input[20 + inpos] << 20);
    output[16 + outpos] = (input[20 + inpos] >>> (25 - 13)) | (input[21 + inpos] << 13);
    output[17 + outpos] = (input[21 + inpos] >>> (25 - 6)) | (input[22 + inpos] << 6) | (input[23 + inpos] << 31);
    output[18 + outpos] = (input[23 + inpos] >>> (25 - 24)) | (input[24 + inpos] << 24);
    output[19 + outpos] = (input[24 + inpos] >>> (25 - 17)) | (input[25 + inpos] << 17);
    output[20 + outpos] = (input[25 + inpos] >>> (25 - 10)) | (input[26 + inpos] << 10);
    output[21 + outpos] = (input[26 + inpos] >>> (25 - 3)) | (input[27 + inpos] << 3) | (input[28 + inpos] << 28);
    output[22 + outpos] = (input[28 + inpos] >>> (25 - 21)) | (input[29 + inpos] << 21);
    output[23 + outpos] = (input[29 + inpos] >>> (25 - 14)) | (input[30 + inpos] << 14);
    output[24 + outpos] = (input[30 + inpos] >>> (25 - 7)) | (input[31 + inpos] << 7);
}

function fastpackwithoutmask26(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 26);
    output[1 + outpos] = (input[1 + inpos] >>> (26 - 20)) | (input[2 + inpos] << 20);
    output[2 + outpos] = (input[2 + inpos] >>> (26 - 14)) | (input[3 + inpos] << 14);
    output[3 + outpos] = (input[3 + inpos] >>> (26 - 8)) | (input[4 + inpos] << 8);
    output[4 + outpos] = (input[4 + inpos] >>> (26 - 2)) | (input[5 + inpos] << 2) | (input[6 + inpos] << 28);
    output[5 + outpos] = (input[6 + inpos] >>> (26 - 22)) | (input[7 + inpos] << 22);
    output[6 + outpos] = (input[7 + inpos] >>> (26 - 16)) | (input[8 + inpos] << 16);
    output[7 + outpos] = (input[8 + inpos] >>> (26 - 10)) | (input[9 + inpos] << 10);
    output[8 + outpos] = (input[9 + inpos] >>> (26 - 4)) | (input[10 + inpos] << 4) | (input[11 + inpos] << 30);
    output[9 + outpos] = (input[11 + inpos] >>> (26 - 24)) | (input[12 + inpos] << 24);
    output[10 + outpos] = (input[12 + inpos] >>> (26 - 18)) | (input[13 + inpos] << 18);
    output[11 + outpos] = (input[13 + inpos] >>> (26 - 12)) | (input[14 + inpos] << 12);
    output[12 + outpos] = (input[14 + inpos] >>> (26 - 6)) | (input[15 + inpos] << 6);
    output[13 + outpos] = input[16 + inpos] | (input[17 + inpos] << 26);
    output[14 + outpos] = (input[17 + inpos] >>> (26 - 20)) | (input[18 + inpos] << 20);
    output[15 + outpos] = (input[18 + inpos] >>> (26 - 14)) | (input[19 + inpos] << 14);
    output[16 + outpos] = (input[19 + inpos] >>> (26 - 8)) | (input[20 + inpos] << 8);
    output[17 + outpos] = (input[20 + inpos] >>> (26 - 2)) | (input[21 + inpos] << 2) | (input[22 + inpos] << 28);
    output[18 + outpos] = (input[22 + inpos] >>> (26 - 22)) | (input[23 + inpos] << 22);
    output[19 + outpos] = (input[23 + inpos] >>> (26 - 16)) | (input[24 + inpos] << 16);
    output[20 + outpos] = (input[24 + inpos] >>> (26 - 10)) | (input[25 + inpos] << 10);
    output[21 + outpos] = (input[25 + inpos] >>> (26 - 4)) | (input[26 + inpos] << 4) | (input[27 + inpos] << 30);
    output[22 + outpos] = (input[27 + inpos] >>> (26 - 24)) | (input[28 + inpos] << 24);
    output[23 + outpos] = (input[28 + inpos] >>> (26 - 18)) | (input[29 + inpos] << 18);
    output[24 + outpos] = (input[29 + inpos] >>> (26 - 12)) | (input[30 + inpos] << 12);
    output[25 + outpos] = (input[30 + inpos] >>> (26 - 6)) | (input[31 + inpos] << 6);
}

function fastpackwithoutmask27(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 27);
    output[1 + outpos] = (input[1 + inpos] >>> (27 - 22)) | (input[2 + inpos] << 22);
    output[2 + outpos] = (input[2 + inpos] >>> (27 - 17)) | (input[3 + inpos] << 17);
    output[3 + outpos] = (input[3 + inpos] >>> (27 - 12)) | (input[4 + inpos] << 12);
    output[4 + outpos] = (input[4 + inpos] >>> (27 - 7)) | (input[5 + inpos] << 7);
    output[5 + outpos] = (input[5 + inpos] >>> (27 - 2)) | (input[6 + inpos] << 2) | (input[7 + inpos] << 29);
    output[6 + outpos] = (input[7 + inpos] >>> (27 - 24)) | (input[8 + inpos] << 24);
    output[7 + outpos] = (input[8 + inpos] >>> (27 - 19)) | (input[9 + inpos] << 19);
    output[8 + outpos] = (input[9 + inpos] >>> (27 - 14)) | (input[10 + inpos] << 14);
    output[9 + outpos] = (input[10 + inpos] >>> (27 - 9)) | (input[11 + inpos] << 9);
    output[10 + outpos] = (input[11 + inpos] >>> (27 - 4)) | (input[12 + inpos] << 4) | (input[13 + inpos] << 31);
    output[11 + outpos] = (input[13 + inpos] >>> (27 - 26)) | (input[14 + inpos] << 26);
    output[12 + outpos] = (input[14 + inpos] >>> (27 - 21)) | (input[15 + inpos] << 21);
    output[13 + outpos] = (input[15 + inpos] >>> (27 - 16)) | (input[16 + inpos] << 16);
    output[14 + outpos] = (input[16 + inpos] >>> (27 - 11)) | (input[17 + inpos] << 11);
    output[15 + outpos] = (input[17 + inpos] >>> (27 - 6)) | (input[18 + inpos] << 6);
    output[16 + outpos] = (input[18 + inpos] >>> (27 - 1)) | (input[19 + inpos] << 1) | (input[20 + inpos] << 28);
    output[17 + outpos] = (input[20 + inpos] >>> (27 - 23)) | (input[21 + inpos] << 23);
    output[18 + outpos] = (input[21 + inpos] >>> (27 - 18)) | (input[22 + inpos] << 18);
    output[19 + outpos] = (input[22 + inpos] >>> (27 - 13)) | (input[23 + inpos] << 13);
    output[20 + outpos] = (input[23 + inpos] >>> (27 - 8)) | (input[24 + inpos] << 8);
    output[21 + outpos] = (input[24 + inpos] >>> (27 - 3)) | (input[25 + inpos] << 3) | (input[26 + inpos] << 30);
    output[22 + outpos] = (input[26 + inpos] >>> (27 - 25)) | (input[27 + inpos] << 25);
    output[23 + outpos] = (input[27 + inpos] >>> (27 - 20)) | (input[28 + inpos] << 20);
    output[24 + outpos] = (input[28 + inpos] >>> (27 - 15)) | (input[29 + inpos] << 15);
    output[25 + outpos] = (input[29 + inpos] >>> (27 - 10)) | (input[30 + inpos] << 10);
    output[26 + outpos] = (input[30 + inpos] >>> (27 - 5)) | (input[31 + inpos] << 5);
}

function fastpackwithoutmask28(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 28);
    output[1 + outpos] = (input[1 + inpos] >>> (28 - 24)) | (input[2 + inpos] << 24);
    output[2 + outpos] = (input[2 + inpos] >>> (28 - 20)) | (input[3 + inpos] << 20);
    output[3 + outpos] = (input[3 + inpos] >>> (28 - 16)) | (input[4 + inpos] << 16);
    output[4 + outpos] = (input[4 + inpos] >>> (28 - 12)) | (input[5 + inpos] << 12);
    output[5 + outpos] = (input[5 + inpos] >>> (28 - 8)) | (input[6 + inpos] << 8);
    output[6 + outpos] = (input[6 + inpos] >>> (28 - 4)) | (input[7 + inpos] << 4);
    output[7 + outpos] = input[8 + inpos] | (input[9 + inpos] << 28);
    output[8 + outpos] = (input[9 + inpos] >>> (28 - 24)) | (input[10 + inpos] << 24);
    output[9 + outpos] = (input[10 + inpos] >>> (28 - 20)) | (input[11 + inpos] << 20);
    output[10 + outpos] = (input[11 + inpos] >>> (28 - 16)) | (input[12 + inpos] << 16);
    output[11 + outpos] = (input[12 + inpos] >>> (28 - 12)) | (input[13 + inpos] << 12);
    output[12 + outpos] = (input[13 + inpos] >>> (28 - 8)) | (input[14 + inpos] << 8);
    output[13 + outpos] = (input[14 + inpos] >>> (28 - 4)) | (input[15 + inpos] << 4);
    output[14 + outpos] = input[16 + inpos] | (input[17 + inpos] << 28);
    output[15 + outpos] = (input[17 + inpos] >>> (28 - 24)) | (input[18 + inpos] << 24);
    output[16 + outpos] = (input[18 + inpos] >>> (28 - 20)) | (input[19 + inpos] << 20);
    output[17 + outpos] = (input[19 + inpos] >>> (28 - 16)) | (input[20 + inpos] << 16);
    output[18 + outpos] = (input[20 + inpos] >>> (28 - 12)) | (input[21 + inpos] << 12);
    output[19 + outpos] = (input[21 + inpos] >>> (28 - 8)) | (input[22 + inpos] << 8);
    output[20 + outpos] = (input[22 + inpos] >>> (28 - 4)) | (input[23 + inpos] << 4);
    output[21 + outpos] = input[24 + inpos] | (input[25 + inpos] << 28);
    output[22 + outpos] = (input[25 + inpos] >>> (28 - 24)) | (input[26 + inpos] << 24);
    output[23 + outpos] = (input[26 + inpos] >>> (28 - 20)) | (input[27 + inpos] << 20);
    output[24 + outpos] = (input[27 + inpos] >>> (28 - 16)) | (input[28 + inpos] << 16);
    output[25 + outpos] = (input[28 + inpos] >>> (28 - 12)) | (input[29 + inpos] << 12);
    output[26 + outpos] = (input[29 + inpos] >>> (28 - 8)) | (input[30 + inpos] << 8);
    output[27 + outpos] = (input[30 + inpos] >>> (28 - 4)) | (input[31 + inpos] << 4);
}

function fastpackwithoutmask29(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 29);
    output[1 + outpos] = (input[1 + inpos] >>> (29 - 26)) | (input[2 + inpos] << 26);
    output[2 + outpos] = (input[2 + inpos] >>> (29 - 23)) | (input[3 + inpos] << 23);
    output[3 + outpos] = (input[3 + inpos] >>> (29 - 20)) | (input[4 + inpos] << 20);
    output[4 + outpos] = (input[4 + inpos] >>> (29 - 17)) | (input[5 + inpos] << 17);
    output[5 + outpos] = (input[5 + inpos] >>> (29 - 14)) | (input[6 + inpos] << 14);
    output[6 + outpos] = (input[6 + inpos] >>> (29 - 11)) | (input[7 + inpos] << 11);
    output[7 + outpos] = (input[7 + inpos] >>> (29 - 8)) | (input[8 + inpos] << 8);
    output[8 + outpos] = (input[8 + inpos] >>> (29 - 5)) | (input[9 + inpos] << 5);
    output[9 + outpos] = (input[9 + inpos] >>> (29 - 2)) | (input[10 + inpos] << 2) | (input[11 + inpos] << 31);
    output[10 + outpos] = (input[11 + inpos] >>> (29 - 28)) | (input[12 + inpos] << 28);
    output[11 + outpos] = (input[12 + inpos] >>> (29 - 25)) | (input[13 + inpos] << 25);
    output[12 + outpos] = (input[13 + inpos] >>> (29 - 22)) | (input[14 + inpos] << 22);
    output[13 + outpos] = (input[14 + inpos] >>> (29 - 19)) | (input[15 + inpos] << 19);
    output[14 + outpos] = (input[15 + inpos] >>> (29 - 16)) | (input[16 + inpos] << 16);
    output[15 + outpos] = (input[16 + inpos] >>> (29 - 13)) | (input[17 + inpos] << 13);
    output[16 + outpos] = (input[17 + inpos] >>> (29 - 10)) | (input[18 + inpos] << 10);
    output[17 + outpos] = (input[18 + inpos] >>> (29 - 7)) | (input[19 + inpos] << 7);
    output[18 + outpos] = (input[19 + inpos] >>> (29 - 4)) | (input[20 + inpos] << 4);
    output[19 + outpos] = (input[20 + inpos] >>> (29 - 1)) | (input[21 + inpos] << 1) | (input[22 + inpos] << 30);
    output[20 + outpos] = (input[22 + inpos] >>> (29 - 27)) | (input[23 + inpos] << 27);
    output[21 + outpos] = (input[23 + inpos] >>> (29 - 24)) | (input[24 + inpos] << 24);
    output[22 + outpos] = (input[24 + inpos] >>> (29 - 21)) | (input[25 + inpos] << 21);
    output[23 + outpos] = (input[25 + inpos] >>> (29 - 18)) | (input[26 + inpos] << 18);
    output[24 + outpos] = (input[26 + inpos] >>> (29 - 15)) | (input[27 + inpos] << 15);
    output[25 + outpos] = (input[27 + inpos] >>> (29 - 12)) | (input[28 + inpos] << 12);
    output[26 + outpos] = (input[28 + inpos] >>> (29 - 9)) | (input[29 + inpos] << 9);
    output[27 + outpos] = (input[29 + inpos] >>> (29 - 6)) | (input[30 + inpos] << 6);
    output[28 + outpos] = (input[30 + inpos] >>> (29 - 3)) | (input[31 + inpos] << 3);
}

function fastpackwithoutmask3(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 3) |
        (input[2 + inpos] << 6) |
        (input[3 + inpos] << 9) |
        (input[4 + inpos] << 12) |
        (input[5 + inpos] << 15) |
        (input[6 + inpos] << 18) |
        (input[7 + inpos] << 21) |
        (input[8 + inpos] << 24) |
        (input[9 + inpos] << 27) |
        (input[10 + inpos] << 30);
    output[1 + outpos] =
        (input[10 + inpos] >>> (3 - 1)) |
        (input[11 + inpos] << 1) |
        (input[12 + inpos] << 4) |
        (input[13 + inpos] << 7) |
        (input[14 + inpos] << 10) |
        (input[15 + inpos] << 13) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 19) |
        (input[18 + inpos] << 22) |
        (input[19 + inpos] << 25) |
        (input[20 + inpos] << 28) |
        (input[21 + inpos] << 31);
    output[2 + outpos] =
        (input[21 + inpos] >>> (3 - 2)) |
        (input[22 + inpos] << 2) |
        (input[23 + inpos] << 5) |
        (input[24 + inpos] << 8) |
        (input[25 + inpos] << 11) |
        (input[26 + inpos] << 14) |
        (input[27 + inpos] << 17) |
        (input[28 + inpos] << 20) |
        (input[29 + inpos] << 23) |
        (input[30 + inpos] << 26) |
        (input[31 + inpos] << 29);
}

function fastpackwithoutmask30(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 30);
    output[1 + outpos] = (input[1 + inpos] >>> (30 - 28)) | (input[2 + inpos] << 28);
    output[2 + outpos] = (input[2 + inpos] >>> (30 - 26)) | (input[3 + inpos] << 26);
    output[3 + outpos] = (input[3 + inpos] >>> (30 - 24)) | (input[4 + inpos] << 24);
    output[4 + outpos] = (input[4 + inpos] >>> (30 - 22)) | (input[5 + inpos] << 22);
    output[5 + outpos] = (input[5 + inpos] >>> (30 - 20)) | (input[6 + inpos] << 20);
    output[6 + outpos] = (input[6 + inpos] >>> (30 - 18)) | (input[7 + inpos] << 18);
    output[7 + outpos] = (input[7 + inpos] >>> (30 - 16)) | (input[8 + inpos] << 16);
    output[8 + outpos] = (input[8 + inpos] >>> (30 - 14)) | (input[9 + inpos] << 14);
    output[9 + outpos] = (input[9 + inpos] >>> (30 - 12)) | (input[10 + inpos] << 12);
    output[10 + outpos] = (input[10 + inpos] >>> (30 - 10)) | (input[11 + inpos] << 10);
    output[11 + outpos] = (input[11 + inpos] >>> (30 - 8)) | (input[12 + inpos] << 8);
    output[12 + outpos] = (input[12 + inpos] >>> (30 - 6)) | (input[13 + inpos] << 6);
    output[13 + outpos] = (input[13 + inpos] >>> (30 - 4)) | (input[14 + inpos] << 4);
    output[14 + outpos] = (input[14 + inpos] >>> (30 - 2)) | (input[15 + inpos] << 2);
    output[15 + outpos] = input[16 + inpos] | (input[17 + inpos] << 30);
    output[16 + outpos] = (input[17 + inpos] >>> (30 - 28)) | (input[18 + inpos] << 28);
    output[17 + outpos] = (input[18 + inpos] >>> (30 - 26)) | (input[19 + inpos] << 26);
    output[18 + outpos] = (input[19 + inpos] >>> (30 - 24)) | (input[20 + inpos] << 24);
    output[19 + outpos] = (input[20 + inpos] >>> (30 - 22)) | (input[21 + inpos] << 22);
    output[20 + outpos] = (input[21 + inpos] >>> (30 - 20)) | (input[22 + inpos] << 20);
    output[21 + outpos] = (input[22 + inpos] >>> (30 - 18)) | (input[23 + inpos] << 18);
    output[22 + outpos] = (input[23 + inpos] >>> (30 - 16)) | (input[24 + inpos] << 16);
    output[23 + outpos] = (input[24 + inpos] >>> (30 - 14)) | (input[25 + inpos] << 14);
    output[24 + outpos] = (input[25 + inpos] >>> (30 - 12)) | (input[26 + inpos] << 12);
    output[25 + outpos] = (input[26 + inpos] >>> (30 - 10)) | (input[27 + inpos] << 10);
    output[26 + outpos] = (input[27 + inpos] >>> (30 - 8)) | (input[28 + inpos] << 8);
    output[27 + outpos] = (input[28 + inpos] >>> (30 - 6)) | (input[29 + inpos] << 6);
    output[28 + outpos] = (input[29 + inpos] >>> (30 - 4)) | (input[30 + inpos] << 4);
    output[29 + outpos] = (input[30 + inpos] >>> (30 - 2)) | (input[31 + inpos] << 2);
}

function fastpackwithoutmask31(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 31);
    output[1 + outpos] = (input[1 + inpos] >>> (31 - 30)) | (input[2 + inpos] << 30);
    output[2 + outpos] = (input[2 + inpos] >>> (31 - 29)) | (input[3 + inpos] << 29);
    output[3 + outpos] = (input[3 + inpos] >>> (31 - 28)) | (input[4 + inpos] << 28);
    output[4 + outpos] = (input[4 + inpos] >>> (31 - 27)) | (input[5 + inpos] << 27);
    output[5 + outpos] = (input[5 + inpos] >>> (31 - 26)) | (input[6 + inpos] << 26);
    output[6 + outpos] = (input[6 + inpos] >>> (31 - 25)) | (input[7 + inpos] << 25);
    output[7 + outpos] = (input[7 + inpos] >>> (31 - 24)) | (input[8 + inpos] << 24);
    output[8 + outpos] = (input[8 + inpos] >>> (31 - 23)) | (input[9 + inpos] << 23);
    output[9 + outpos] = (input[9 + inpos] >>> (31 - 22)) | (input[10 + inpos] << 22);
    output[10 + outpos] = (input[10 + inpos] >>> (31 - 21)) | (input[11 + inpos] << 21);
    output[11 + outpos] = (input[11 + inpos] >>> (31 - 20)) | (input[12 + inpos] << 20);
    output[12 + outpos] = (input[12 + inpos] >>> (31 - 19)) | (input[13 + inpos] << 19);
    output[13 + outpos] = (input[13 + inpos] >>> (31 - 18)) | (input[14 + inpos] << 18);
    output[14 + outpos] = (input[14 + inpos] >>> (31 - 17)) | (input[15 + inpos] << 17);
    output[15 + outpos] = (input[15 + inpos] >>> (31 - 16)) | (input[16 + inpos] << 16);
    output[16 + outpos] = (input[16 + inpos] >>> (31 - 15)) | (input[17 + inpos] << 15);
    output[17 + outpos] = (input[17 + inpos] >>> (31 - 14)) | (input[18 + inpos] << 14);
    output[18 + outpos] = (input[18 + inpos] >>> (31 - 13)) | (input[19 + inpos] << 13);
    output[19 + outpos] = (input[19 + inpos] >>> (31 - 12)) | (input[20 + inpos] << 12);
    output[20 + outpos] = (input[20 + inpos] >>> (31 - 11)) | (input[21 + inpos] << 11);
    output[21 + outpos] = (input[21 + inpos] >>> (31 - 10)) | (input[22 + inpos] << 10);
    output[22 + outpos] = (input[22 + inpos] >>> (31 - 9)) | (input[23 + inpos] << 9);
    output[23 + outpos] = (input[23 + inpos] >>> (31 - 8)) | (input[24 + inpos] << 8);
    output[24 + outpos] = (input[24 + inpos] >>> (31 - 7)) | (input[25 + inpos] << 7);
    output[25 + outpos] = (input[25 + inpos] >>> (31 - 6)) | (input[26 + inpos] << 6);
    output[26 + outpos] = (input[26 + inpos] >>> (31 - 5)) | (input[27 + inpos] << 5);
    output[27 + outpos] = (input[27 + inpos] >>> (31 - 4)) | (input[28 + inpos] << 4);
    output[28 + outpos] = (input[28 + inpos] >>> (31 - 3)) | (input[29 + inpos] << 3);
    output[29 + outpos] = (input[29 + inpos] >>> (31 - 2)) | (input[30 + inpos] << 2);
    output[30 + outpos] = (input[30 + inpos] >>> (31 - 1)) | (input[31 + inpos] << 1);
}

function fastpackwithoutmask32(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    arraycopy(input, inpos, output, outpos, 32);
}

function fastpackwithoutmask4(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 4) |
        (input[2 + inpos] << 8) |
        (input[3 + inpos] << 12) |
        (input[4 + inpos] << 16) |
        (input[5 + inpos] << 20) |
        (input[6 + inpos] << 24) |
        (input[7 + inpos] << 28);
    output[1 + outpos] =
        input[8 + inpos] |
        (input[9 + inpos] << 4) |
        (input[10 + inpos] << 8) |
        (input[11 + inpos] << 12) |
        (input[12 + inpos] << 16) |
        (input[13 + inpos] << 20) |
        (input[14 + inpos] << 24) |
        (input[15 + inpos] << 28);
    output[2 + outpos] =
        input[16 + inpos] |
        (input[17 + inpos] << 4) |
        (input[18 + inpos] << 8) |
        (input[19 + inpos] << 12) |
        (input[20 + inpos] << 16) |
        (input[21 + inpos] << 20) |
        (input[22 + inpos] << 24) |
        (input[23 + inpos] << 28);
    output[3 + outpos] =
        input[24 + inpos] |
        (input[25 + inpos] << 4) |
        (input[26 + inpos] << 8) |
        (input[27 + inpos] << 12) |
        (input[28 + inpos] << 16) |
        (input[29 + inpos] << 20) |
        (input[30 + inpos] << 24) |
        (input[31 + inpos] << 28);
}

function fastpackwithoutmask5(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 5) |
        (input[2 + inpos] << 10) |
        (input[3 + inpos] << 15) |
        (input[4 + inpos] << 20) |
        (input[5 + inpos] << 25) |
        (input[6 + inpos] << 30);
    output[1 + outpos] =
        (input[6 + inpos] >>> (5 - 3)) |
        (input[7 + inpos] << 3) |
        (input[8 + inpos] << 8) |
        (input[9 + inpos] << 13) |
        (input[10 + inpos] << 18) |
        (input[11 + inpos] << 23) |
        (input[12 + inpos] << 28);
    output[2 + outpos] =
        (input[12 + inpos] >>> (5 - 1)) |
        (input[13 + inpos] << 1) |
        (input[14 + inpos] << 6) |
        (input[15 + inpos] << 11) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 21) |
        (input[18 + inpos] << 26) |
        (input[19 + inpos] << 31);
    output[3 + outpos] =
        (input[19 + inpos] >>> (5 - 4)) |
        (input[20 + inpos] << 4) |
        (input[21 + inpos] << 9) |
        (input[22 + inpos] << 14) |
        (input[23 + inpos] << 19) |
        (input[24 + inpos] << 24) |
        (input[25 + inpos] << 29);
    output[4 + outpos] =
        (input[25 + inpos] >>> (5 - 2)) |
        (input[26 + inpos] << 2) |
        (input[27 + inpos] << 7) |
        (input[28 + inpos] << 12) |
        (input[29 + inpos] << 17) |
        (input[30 + inpos] << 22) |
        (input[31 + inpos] << 27);
}

function fastpackwithoutmask6(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 6) |
        (input[2 + inpos] << 12) |
        (input[3 + inpos] << 18) |
        (input[4 + inpos] << 24) |
        (input[5 + inpos] << 30);
    output[1 + outpos] =
        (input[5 + inpos] >>> (6 - 4)) |
        (input[6 + inpos] << 4) |
        (input[7 + inpos] << 10) |
        (input[8 + inpos] << 16) |
        (input[9 + inpos] << 22) |
        (input[10 + inpos] << 28);
    output[2 + outpos] =
        (input[10 + inpos] >>> (6 - 2)) |
        (input[11 + inpos] << 2) |
        (input[12 + inpos] << 8) |
        (input[13 + inpos] << 14) |
        (input[14 + inpos] << 20) |
        (input[15 + inpos] << 26);
    output[3 + outpos] =
        input[16 + inpos] |
        (input[17 + inpos] << 6) |
        (input[18 + inpos] << 12) |
        (input[19 + inpos] << 18) |
        (input[20 + inpos] << 24) |
        (input[21 + inpos] << 30);
    output[4 + outpos] =
        (input[21 + inpos] >>> (6 - 4)) |
        (input[22 + inpos] << 4) |
        (input[23 + inpos] << 10) |
        (input[24 + inpos] << 16) |
        (input[25 + inpos] << 22) |
        (input[26 + inpos] << 28);
    output[5 + outpos] =
        (input[26 + inpos] >>> (6 - 2)) |
        (input[27 + inpos] << 2) |
        (input[28 + inpos] << 8) |
        (input[29 + inpos] << 14) |
        (input[30 + inpos] << 20) |
        (input[31 + inpos] << 26);
}

function fastpackwithoutmask7(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] =
        input[inpos] |
        (input[1 + inpos] << 7) |
        (input[2 + inpos] << 14) |
        (input[3 + inpos] << 21) |
        (input[4 + inpos] << 28);
    output[1 + outpos] =
        (input[4 + inpos] >>> (7 - 3)) |
        (input[5 + inpos] << 3) |
        (input[6 + inpos] << 10) |
        (input[7 + inpos] << 17) |
        (input[8 + inpos] << 24) |
        (input[9 + inpos] << 31);
    output[2 + outpos] =
        (input[9 + inpos] >>> (7 - 6)) |
        (input[10 + inpos] << 6) |
        (input[11 + inpos] << 13) |
        (input[12 + inpos] << 20) |
        (input[13 + inpos] << 27);
    output[3 + outpos] =
        (input[13 + inpos] >>> (7 - 2)) |
        (input[14 + inpos] << 2) |
        (input[15 + inpos] << 9) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 23) |
        (input[18 + inpos] << 30);
    output[4 + outpos] =
        (input[18 + inpos] >>> (7 - 5)) |
        (input[19 + inpos] << 5) |
        (input[20 + inpos] << 12) |
        (input[21 + inpos] << 19) |
        (input[22 + inpos] << 26);
    output[5 + outpos] =
        (input[22 + inpos] >>> (7 - 1)) |
        (input[23 + inpos] << 1) |
        (input[24 + inpos] << 8) |
        (input[25 + inpos] << 15) |
        (input[26 + inpos] << 22) |
        (input[27 + inpos] << 29);
    output[6 + outpos] =
        (input[27 + inpos] >>> (7 - 4)) |
        (input[28 + inpos] << 4) |
        (input[29 + inpos] << 11) |
        (input[30 + inpos] << 18) |
        (input[31 + inpos] << 25);
}

function fastpackwithoutmask8(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 8) | (input[2 + inpos] << 16) | (input[3 + inpos] << 24);
    output[1 + outpos] =
        input[4 + inpos] | (input[5 + inpos] << 8) | (input[6 + inpos] << 16) | (input[7 + inpos] << 24);
    output[2 + outpos] =
        input[8 + inpos] | (input[9 + inpos] << 8) | (input[10 + inpos] << 16) | (input[11 + inpos] << 24);
    output[3 + outpos] =
        input[12 + inpos] | (input[13 + inpos] << 8) | (input[14 + inpos] << 16) | (input[15 + inpos] << 24);
    output[4 + outpos] =
        input[16 + inpos] | (input[17 + inpos] << 8) | (input[18 + inpos] << 16) | (input[19 + inpos] << 24);
    output[5 + outpos] =
        input[20 + inpos] | (input[21 + inpos] << 8) | (input[22 + inpos] << 16) | (input[23 + inpos] << 24);
    output[6 + outpos] =
        input[24 + inpos] | (input[25 + inpos] << 8) | (input[26 + inpos] << 16) | (input[27 + inpos] << 24);
    output[7 + outpos] =
        input[28 + inpos] | (input[29 + inpos] << 8) | (input[30 + inpos] << 16) | (input[31 + inpos] << 24);
}

function fastpackwithoutmask9(input: Uint32Array, inpos: number, output: Uint32Array, outpos: number) {
    output[outpos] = input[inpos] | (input[1 + inpos] << 9) | (input[2 + inpos] << 18) | (input[3 + inpos] << 27);
    output[1 + outpos] =
        (input[3 + inpos] >>> (9 - 4)) |
        (input[4 + inpos] << 4) |
        (input[5 + inpos] << 13) |
        (input[6 + inpos] << 22) |
        (input[7 + inpos] << 31);
    output[2 + outpos] =
        (input[7 + inpos] >>> (9 - 8)) | (input[8 + inpos] << 8) | (input[9 + inpos] << 17) | (input[10 + inpos] << 26);
    output[3 + outpos] =
        (input[10 + inpos] >>> (9 - 3)) |
        (input[11 + inpos] << 3) |
        (input[12 + inpos] << 12) |
        (input[13 + inpos] << 21) |
        (input[14 + inpos] << 30);
    output[4 + outpos] =
        (input[14 + inpos] >>> (9 - 7)) |
        (input[15 + inpos] << 7) |
        (input[16 + inpos] << 16) |
        (input[17 + inpos] << 25);
    output[5 + outpos] =
        (input[17 + inpos] >>> (9 - 2)) |
        (input[18 + inpos] << 2) |
        (input[19 + inpos] << 11) |
        (input[20 + inpos] << 20) |
        (input[21 + inpos] << 29);
    output[6 + outpos] =
        (input[21 + inpos] >>> (9 - 6)) |
        (input[22 + inpos] << 6) |
        (input[23 + inpos] << 15) |
        (input[24 + inpos] << 24);
    output[7 + outpos] =
        (input[24 + inpos] >>> (9 - 1)) |
        (input[25 + inpos] << 1) |
        (input[26 + inpos] << 10) |
        (input[27 + inpos] << 19) |
        (input[28 + inpos] << 28);
    output[8 + outpos] =
        (input[28 + inpos] >>> (9 - 5)) |
        (input[29 + inpos] << 5) |
        (input[30 + inpos] << 14) |
        (input[31 + inpos] << 23);
}

/**
 * Pack the 32 numberegers
 *
 * @param input
 *                source array
 * @param inpos
 *                starting ponumber in the source array
 * @param output
 *                output array
 * @param outpos
 *                starting ponumber in the output array
 * @param bit
 *                how many bits to use per numbereger
 */
export function fastunpack(model: {
    input: Uint32Array;
    inpos: number;
    output: Uint32Array;
    outpos: number;
    bit: number;
}) {
    switch (model.bit) {
        case 0:
            fastunpack0(model);
            break;
        case 1:
            fastunpack1(model);
            break;
        case 2:
            fastunpack2(model);
            break;
        case 3:
            fastunpack3(model);
            break;
        case 4:
            fastunpack4(model);
            break;
        case 5:
            fastunpack5(model);
            break;
        case 6:
            fastunpack6(model);
            break;
        case 7:
            fastunpack7(model);
            break;
        case 8:
            fastunpack8(model);
            break;
        case 9:
            fastunpack9(model);
            break;
        case 10:
            fastunpack10(model);
            break;
        case 11:
            fastunpack11(model);
            break;
        case 12:
            fastunpack12(model);
            break;
        case 13:
            fastunpack13(model);
            break;
        case 14:
            fastunpack14(model);
            break;
        case 15:
            fastunpack15(model);
            break;
        case 16:
            fastunpack16(model);
            break;
        case 17:
            fastunpack17(model);
            break;
        case 18:
            fastunpack18(model);
            break;
        case 19:
            fastunpack19(model);
            break;
        case 20:
            fastunpack20(model);
            break;
        case 21:
            fastunpack21(model);
            break;
        case 22:
            fastunpack22(model);
            break;
        case 23:
            fastunpack23(model);
            break;
        case 24:
            fastunpack24(model);
            break;
        case 25:
            fastunpack25(model);
            break;
        case 26:
            fastunpack26(model);
            break;
        case 27:
            fastunpack27(model);
            break;
        case 28:
            fastunpack28(model);
            break;
        case 29:
            fastunpack29(model);
            break;
        case 30:
            fastunpack30(model);
            break;
        case 31:
            fastunpack31(model);
            break;
        case 32:
            fastunpack32(model);
            break;
        default:
            throw new Error("Unsupported bit width.");
    }
}

function fastunpack0(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output.fill(0, model.outpos, model.outpos + 32);
}

function fastunpack1(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 1;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 1) & 1;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 2) & 1;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 3) & 1;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 4) & 1;
    model.output[5 + model.outpos] = (model.input[model.inpos] >>> 5) & 1;
    model.output[6 + model.outpos] = (model.input[model.inpos] >>> 6) & 1;
    model.output[7 + model.outpos] = (model.input[model.inpos] >>> 7) & 1;
    model.output[8 + model.outpos] = (model.input[model.inpos] >>> 8) & 1;
    model.output[9 + model.outpos] = (model.input[model.inpos] >>> 9) & 1;
    model.output[10 + model.outpos] = (model.input[model.inpos] >>> 10) & 1;
    model.output[11 + model.outpos] = (model.input[model.inpos] >>> 11) & 1;
    model.output[12 + model.outpos] = (model.input[model.inpos] >>> 12) & 1;
    model.output[13 + model.outpos] = (model.input[model.inpos] >>> 13) & 1;
    model.output[14 + model.outpos] = (model.input[model.inpos] >>> 14) & 1;
    model.output[15 + model.outpos] = (model.input[model.inpos] >>> 15) & 1;
    model.output[16 + model.outpos] = (model.input[model.inpos] >>> 16) & 1;
    model.output[17 + model.outpos] = (model.input[model.inpos] >>> 17) & 1;
    model.output[18 + model.outpos] = (model.input[model.inpos] >>> 18) & 1;
    model.output[19 + model.outpos] = (model.input[model.inpos] >>> 19) & 1;
    model.output[20 + model.outpos] = (model.input[model.inpos] >>> 20) & 1;
    model.output[21 + model.outpos] = (model.input[model.inpos] >>> 21) & 1;
    model.output[22 + model.outpos] = (model.input[model.inpos] >>> 22) & 1;
    model.output[23 + model.outpos] = (model.input[model.inpos] >>> 23) & 1;
    model.output[24 + model.outpos] = (model.input[model.inpos] >>> 24) & 1;
    model.output[25 + model.outpos] = (model.input[model.inpos] >>> 25) & 1;
    model.output[26 + model.outpos] = (model.input[model.inpos] >>> 26) & 1;
    model.output[27 + model.outpos] = (model.input[model.inpos] >>> 27) & 1;
    model.output[28 + model.outpos] = (model.input[model.inpos] >>> 28) & 1;
    model.output[29 + model.outpos] = (model.input[model.inpos] >>> 29) & 1;
    model.output[30 + model.outpos] = (model.input[model.inpos] >>> 30) & 1;
    model.output[31 + model.outpos] = model.input[model.inpos] >>> 31;
}

function fastunpack10(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 1023;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 10) & 1023;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 20) & 1023;
    model.output[3 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 255) << (10 - 8));
    model.output[4 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 1023;
    model.output[5 + model.outpos] = (model.input[1 + model.inpos] >>> 18) & 1023;
    model.output[6 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 63) << (10 - 6));
    model.output[7 + model.outpos] = (model.input[2 + model.inpos] >>> 6) & 1023;
    model.output[8 + model.outpos] = (model.input[2 + model.inpos] >>> 16) & 1023;
    model.output[9 + model.outpos] =
        (model.input[2 + model.inpos] >>> 26) | ((model.input[3 + model.inpos] & 15) << (10 - 4));
    model.output[10 + model.outpos] = (model.input[3 + model.inpos] >>> 4) & 1023;
    model.output[11 + model.outpos] = (model.input[3 + model.inpos] >>> 14) & 1023;
    model.output[12 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 3) << (10 - 2));
    model.output[13 + model.outpos] = (model.input[4 + model.inpos] >>> 2) & 1023;
    model.output[14 + model.outpos] = (model.input[4 + model.inpos] >>> 12) & 1023;
    model.output[15 + model.outpos] = model.input[4 + model.inpos] >>> 22;
    model.output[16 + model.outpos] = (model.input[5 + model.inpos] >>> 0) & 1023;
    model.output[17 + model.outpos] = (model.input[5 + model.inpos] >>> 10) & 1023;
    model.output[18 + model.outpos] = (model.input[5 + model.inpos] >>> 20) & 1023;
    model.output[19 + model.outpos] =
        (model.input[5 + model.inpos] >>> 30) | ((model.input[6 + model.inpos] & 255) << (10 - 8));
    model.output[20 + model.outpos] = (model.input[6 + model.inpos] >>> 8) & 1023;
    model.output[21 + model.outpos] = (model.input[6 + model.inpos] >>> 18) & 1023;
    model.output[22 + model.outpos] =
        (model.input[6 + model.inpos] >>> 28) | ((model.input[7 + model.inpos] & 63) << (10 - 6));
    model.output[23 + model.outpos] = (model.input[7 + model.inpos] >>> 6) & 1023;
    model.output[24 + model.outpos] = (model.input[7 + model.inpos] >>> 16) & 1023;
    model.output[25 + model.outpos] =
        (model.input[7 + model.inpos] >>> 26) | ((model.input[8 + model.inpos] & 15) << (10 - 4));
    model.output[26 + model.outpos] = (model.input[8 + model.inpos] >>> 4) & 1023;
    model.output[27 + model.outpos] = (model.input[8 + model.inpos] >>> 14) & 1023;
    model.output[28 + model.outpos] =
        (model.input[8 + model.inpos] >>> 24) | ((model.input[9 + model.inpos] & 3) << (10 - 2));
    model.output[29 + model.outpos] = (model.input[9 + model.inpos] >>> 2) & 1023;
    model.output[30 + model.outpos] = (model.input[9 + model.inpos] >>> 12) & 1023;
    model.output[31 + model.outpos] = model.input[9 + model.inpos] >>> 22;
}

function fastunpack11(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 2047;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 11) & 2047;
    model.output[2 + model.outpos] =
        (model.input[model.inpos] >>> 22) | ((model.input[1 + model.inpos] & 1) << (11 - 1));
    model.output[3 + model.outpos] = (model.input[1 + model.inpos] >>> 1) & 2047;
    model.output[4 + model.outpos] = (model.input[1 + model.inpos] >>> 12) & 2047;
    model.output[5 + model.outpos] =
        (model.input[1 + model.inpos] >>> 23) | ((model.input[2 + model.inpos] & 3) << (11 - 2));
    model.output[6 + model.outpos] = (model.input[2 + model.inpos] >>> 2) & 2047;
    model.output[7 + model.outpos] = (model.input[2 + model.inpos] >>> 13) & 2047;
    model.output[8 + model.outpos] =
        (model.input[2 + model.inpos] >>> 24) | ((model.input[3 + model.inpos] & 7) << (11 - 3));
    model.output[9 + model.outpos] = (model.input[3 + model.inpos] >>> 3) & 2047;
    model.output[10 + model.outpos] = (model.input[3 + model.inpos] >>> 14) & 2047;
    model.output[11 + model.outpos] =
        (model.input[3 + model.inpos] >>> 25) | ((model.input[4 + model.inpos] & 15) << (11 - 4));
    model.output[12 + model.outpos] = (model.input[4 + model.inpos] >>> 4) & 2047;
    model.output[13 + model.outpos] = (model.input[4 + model.inpos] >>> 15) & 2047;
    model.output[14 + model.outpos] =
        (model.input[4 + model.inpos] >>> 26) | ((model.input[5 + model.inpos] & 31) << (11 - 5));
    model.output[15 + model.outpos] = (model.input[5 + model.inpos] >>> 5) & 2047;
    model.output[16 + model.outpos] = (model.input[5 + model.inpos] >>> 16) & 2047;
    model.output[17 + model.outpos] =
        (model.input[5 + model.inpos] >>> 27) | ((model.input[6 + model.inpos] & 63) << (11 - 6));
    model.output[18 + model.outpos] = (model.input[6 + model.inpos] >>> 6) & 2047;
    model.output[19 + model.outpos] = (model.input[6 + model.inpos] >>> 17) & 2047;
    model.output[20 + model.outpos] =
        (model.input[6 + model.inpos] >>> 28) | ((model.input[7 + model.inpos] & 127) << (11 - 7));
    model.output[21 + model.outpos] = (model.input[7 + model.inpos] >>> 7) & 2047;
    model.output[22 + model.outpos] = (model.input[7 + model.inpos] >>> 18) & 2047;
    model.output[23 + model.outpos] =
        (model.input[7 + model.inpos] >>> 29) | ((model.input[8 + model.inpos] & 255) << (11 - 8));
    model.output[24 + model.outpos] = (model.input[8 + model.inpos] >>> 8) & 2047;
    model.output[25 + model.outpos] = (model.input[8 + model.inpos] >>> 19) & 2047;
    model.output[26 + model.outpos] =
        (model.input[8 + model.inpos] >>> 30) | ((model.input[9 + model.inpos] & 511) << (11 - 9));
    model.output[27 + model.outpos] = (model.input[9 + model.inpos] >>> 9) & 2047;
    model.output[28 + model.outpos] = (model.input[9 + model.inpos] >>> 20) & 2047;
    model.output[29 + model.outpos] =
        (model.input[9 + model.inpos] >>> 31) | ((model.input[10 + model.inpos] & 1023) << (11 - 10));
    model.output[30 + model.outpos] = (model.input[10 + model.inpos] >>> 10) & 2047;
    model.output[31 + model.outpos] = model.input[10 + model.inpos] >>> 21;
}

function fastunpack12(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 4095;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 12) & 4095;
    model.output[2 + model.outpos] =
        (model.input[model.inpos] >>> 24) | ((model.input[1 + model.inpos] & 15) << (12 - 4));
    model.output[3 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 4095;
    model.output[4 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 4095;
    model.output[5 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 255) << (12 - 8));
    model.output[6 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 4095;
    model.output[7 + model.outpos] = model.input[2 + model.inpos] >>> 20;
    model.output[8 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 4095;
    model.output[9 + model.outpos] = (model.input[3 + model.inpos] >>> 12) & 4095;
    model.output[10 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 15) << (12 - 4));
    model.output[11 + model.outpos] = (model.input[4 + model.inpos] >>> 4) & 4095;
    model.output[12 + model.outpos] = (model.input[4 + model.inpos] >>> 16) & 4095;
    model.output[13 + model.outpos] =
        (model.input[4 + model.inpos] >>> 28) | ((model.input[5 + model.inpos] & 255) << (12 - 8));
    model.output[14 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 4095;
    model.output[15 + model.outpos] = model.input[5 + model.inpos] >>> 20;
    model.output[16 + model.outpos] = (model.input[6 + model.inpos] >>> 0) & 4095;
    model.output[17 + model.outpos] = (model.input[6 + model.inpos] >>> 12) & 4095;
    model.output[18 + model.outpos] =
        (model.input[6 + model.inpos] >>> 24) | ((model.input[7 + model.inpos] & 15) << (12 - 4));
    model.output[19 + model.outpos] = (model.input[7 + model.inpos] >>> 4) & 4095;
    model.output[20 + model.outpos] = (model.input[7 + model.inpos] >>> 16) & 4095;
    model.output[21 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 255) << (12 - 8));
    model.output[22 + model.outpos] = (model.input[8 + model.inpos] >>> 8) & 4095;
    model.output[23 + model.outpos] = model.input[8 + model.inpos] >>> 20;
    model.output[24 + model.outpos] = (model.input[9 + model.inpos] >>> 0) & 4095;
    model.output[25 + model.outpos] = (model.input[9 + model.inpos] >>> 12) & 4095;
    model.output[26 + model.outpos] =
        (model.input[9 + model.inpos] >>> 24) | ((model.input[10 + model.inpos] & 15) << (12 - 4));
    model.output[27 + model.outpos] = (model.input[10 + model.inpos] >>> 4) & 4095;
    model.output[28 + model.outpos] = (model.input[10 + model.inpos] >>> 16) & 4095;
    model.output[29 + model.outpos] =
        (model.input[10 + model.inpos] >>> 28) | ((model.input[11 + model.inpos] & 255) << (12 - 8));
    model.output[30 + model.outpos] = (model.input[11 + model.inpos] >>> 8) & 4095;
    model.output[31 + model.outpos] = model.input[11 + model.inpos] >>> 20;
}

function fastunpack13(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 8191;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 13) & 8191;
    model.output[2 + model.outpos] =
        (model.input[model.inpos] >>> 26) | ((model.input[1 + model.inpos] & 127) << (13 - 7));
    model.output[3 + model.outpos] = (model.input[1 + model.inpos] >>> 7) & 8191;
    model.output[4 + model.outpos] =
        (model.input[1 + model.inpos] >>> 20) | ((model.input[2 + model.inpos] & 1) << (13 - 1));
    model.output[5 + model.outpos] = (model.input[2 + model.inpos] >>> 1) & 8191;
    model.output[6 + model.outpos] = (model.input[2 + model.inpos] >>> 14) & 8191;
    model.output[7 + model.outpos] =
        (model.input[2 + model.inpos] >>> 27) | ((model.input[3 + model.inpos] & 255) << (13 - 8));
    model.output[8 + model.outpos] = (model.input[3 + model.inpos] >>> 8) & 8191;
    model.output[9 + model.outpos] =
        (model.input[3 + model.inpos] >>> 21) | ((model.input[4 + model.inpos] & 3) << (13 - 2));
    model.output[10 + model.outpos] = (model.input[4 + model.inpos] >>> 2) & 8191;
    model.output[11 + model.outpos] = (model.input[4 + model.inpos] >>> 15) & 8191;
    model.output[12 + model.outpos] =
        (model.input[4 + model.inpos] >>> 28) | ((model.input[5 + model.inpos] & 511) << (13 - 9));
    model.output[13 + model.outpos] = (model.input[5 + model.inpos] >>> 9) & 8191;
    model.output[14 + model.outpos] =
        (model.input[5 + model.inpos] >>> 22) | ((model.input[6 + model.inpos] & 7) << (13 - 3));
    model.output[15 + model.outpos] = (model.input[6 + model.inpos] >>> 3) & 8191;
    model.output[16 + model.outpos] = (model.input[6 + model.inpos] >>> 16) & 8191;
    model.output[17 + model.outpos] =
        (model.input[6 + model.inpos] >>> 29) | ((model.input[7 + model.inpos] & 1023) << (13 - 10));
    model.output[18 + model.outpos] = (model.input[7 + model.inpos] >>> 10) & 8191;
    model.output[19 + model.outpos] =
        (model.input[7 + model.inpos] >>> 23) | ((model.input[8 + model.inpos] & 15) << (13 - 4));
    model.output[20 + model.outpos] = (model.input[8 + model.inpos] >>> 4) & 8191;
    model.output[21 + model.outpos] = (model.input[8 + model.inpos] >>> 17) & 8191;
    model.output[22 + model.outpos] =
        (model.input[8 + model.inpos] >>> 30) | ((model.input[9 + model.inpos] & 2047) << (13 - 11));
    model.output[23 + model.outpos] = (model.input[9 + model.inpos] >>> 11) & 8191;
    model.output[24 + model.outpos] =
        (model.input[9 + model.inpos] >>> 24) | ((model.input[10 + model.inpos] & 31) << (13 - 5));
    model.output[25 + model.outpos] = (model.input[10 + model.inpos] >>> 5) & 8191;
    model.output[26 + model.outpos] = (model.input[10 + model.inpos] >>> 18) & 8191;
    model.output[27 + model.outpos] =
        (model.input[10 + model.inpos] >>> 31) | ((model.input[11 + model.inpos] & 4095) << (13 - 12));
    model.output[28 + model.outpos] = (model.input[11 + model.inpos] >>> 12) & 8191;
    model.output[29 + model.outpos] =
        (model.input[11 + model.inpos] >>> 25) | ((model.input[12 + model.inpos] & 63) << (13 - 6));
    model.output[30 + model.outpos] = (model.input[12 + model.inpos] >>> 6) & 8191;
    model.output[31 + model.outpos] = model.input[12 + model.inpos] >>> 19;
}

function fastunpack14(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 16383;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 14) & 16383;
    model.output[2 + model.outpos] =
        (model.input[model.inpos] >>> 28) | ((model.input[1 + model.inpos] & 1023) << (14 - 10));
    model.output[3 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 16383;
    model.output[4 + model.outpos] =
        (model.input[1 + model.inpos] >>> 24) | ((model.input[2 + model.inpos] & 63) << (14 - 6));
    model.output[5 + model.outpos] = (model.input[2 + model.inpos] >>> 6) & 16383;
    model.output[6 + model.outpos] =
        (model.input[2 + model.inpos] >>> 20) | ((model.input[3 + model.inpos] & 3) << (14 - 2));
    model.output[7 + model.outpos] = (model.input[3 + model.inpos] >>> 2) & 16383;
    model.output[8 + model.outpos] = (model.input[3 + model.inpos] >>> 16) & 16383;
    model.output[9 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 4095) << (14 - 12));
    model.output[10 + model.outpos] = (model.input[4 + model.inpos] >>> 12) & 16383;
    model.output[11 + model.outpos] =
        (model.input[4 + model.inpos] >>> 26) | ((model.input[5 + model.inpos] & 255) << (14 - 8));
    model.output[12 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 16383;
    model.output[13 + model.outpos] =
        (model.input[5 + model.inpos] >>> 22) | ((model.input[6 + model.inpos] & 15) << (14 - 4));
    model.output[14 + model.outpos] = (model.input[6 + model.inpos] >>> 4) & 16383;
    model.output[15 + model.outpos] = model.input[6 + model.inpos] >>> 18;
    model.output[16 + model.outpos] = (model.input[7 + model.inpos] >>> 0) & 16383;
    model.output[17 + model.outpos] = (model.input[7 + model.inpos] >>> 14) & 16383;
    model.output[18 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 1023) << (14 - 10));
    model.output[19 + model.outpos] = (model.input[8 + model.inpos] >>> 10) & 16383;
    model.output[20 + model.outpos] =
        (model.input[8 + model.inpos] >>> 24) | ((model.input[9 + model.inpos] & 63) << (14 - 6));
    model.output[21 + model.outpos] = (model.input[9 + model.inpos] >>> 6) & 16383;
    model.output[22 + model.outpos] =
        (model.input[9 + model.inpos] >>> 20) | ((model.input[10 + model.inpos] & 3) << (14 - 2));
    model.output[23 + model.outpos] = (model.input[10 + model.inpos] >>> 2) & 16383;
    model.output[24 + model.outpos] = (model.input[10 + model.inpos] >>> 16) & 16383;
    model.output[25 + model.outpos] =
        (model.input[10 + model.inpos] >>> 30) | ((model.input[11 + model.inpos] & 4095) << (14 - 12));
    model.output[26 + model.outpos] = (model.input[11 + model.inpos] >>> 12) & 16383;
    model.output[27 + model.outpos] =
        (model.input[11 + model.inpos] >>> 26) | ((model.input[12 + model.inpos] & 255) << (14 - 8));
    model.output[28 + model.outpos] = (model.input[12 + model.inpos] >>> 8) & 16383;
    model.output[29 + model.outpos] =
        (model.input[12 + model.inpos] >>> 22) | ((model.input[13 + model.inpos] & 15) << (14 - 4));
    model.output[30 + model.outpos] = (model.input[13 + model.inpos] >>> 4) & 16383;
    model.output[31 + model.outpos] = model.input[13 + model.inpos] >>> 18;
}

function fastunpack15(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 32767;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 15) & 32767;
    model.output[2 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 8191) << (15 - 13));
    model.output[3 + model.outpos] = (model.input[1 + model.inpos] >>> 13) & 32767;
    model.output[4 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 2047) << (15 - 11));
    model.output[5 + model.outpos] = (model.input[2 + model.inpos] >>> 11) & 32767;
    model.output[6 + model.outpos] =
        (model.input[2 + model.inpos] >>> 26) | ((model.input[3 + model.inpos] & 511) << (15 - 9));
    model.output[7 + model.outpos] = (model.input[3 + model.inpos] >>> 9) & 32767;
    model.output[8 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 127) << (15 - 7));
    model.output[9 + model.outpos] = (model.input[4 + model.inpos] >>> 7) & 32767;
    model.output[10 + model.outpos] =
        (model.input[4 + model.inpos] >>> 22) | ((model.input[5 + model.inpos] & 31) << (15 - 5));
    model.output[11 + model.outpos] = (model.input[5 + model.inpos] >>> 5) & 32767;
    model.output[12 + model.outpos] =
        (model.input[5 + model.inpos] >>> 20) | ((model.input[6 + model.inpos] & 7) << (15 - 3));
    model.output[13 + model.outpos] = (model.input[6 + model.inpos] >>> 3) & 32767;
    model.output[14 + model.outpos] =
        (model.input[6 + model.inpos] >>> 18) | ((model.input[7 + model.inpos] & 1) << (15 - 1));
    model.output[15 + model.outpos] = (model.input[7 + model.inpos] >>> 1) & 32767;
    model.output[16 + model.outpos] = (model.input[7 + model.inpos] >>> 16) & 32767;
    model.output[17 + model.outpos] =
        (model.input[7 + model.inpos] >>> 31) | ((model.input[8 + model.inpos] & 16383) << (15 - 14));
    model.output[18 + model.outpos] = (model.input[8 + model.inpos] >>> 14) & 32767;
    model.output[19 + model.outpos] =
        (model.input[8 + model.inpos] >>> 29) | ((model.input[9 + model.inpos] & 4095) << (15 - 12));
    model.output[20 + model.outpos] = (model.input[9 + model.inpos] >>> 12) & 32767;
    model.output[21 + model.outpos] =
        (model.input[9 + model.inpos] >>> 27) | ((model.input[10 + model.inpos] & 1023) << (15 - 10));
    model.output[22 + model.outpos] = (model.input[10 + model.inpos] >>> 10) & 32767;
    model.output[23 + model.outpos] =
        (model.input[10 + model.inpos] >>> 25) | ((model.input[11 + model.inpos] & 255) << (15 - 8));
    model.output[24 + model.outpos] = (model.input[11 + model.inpos] >>> 8) & 32767;
    model.output[25 + model.outpos] =
        (model.input[11 + model.inpos] >>> 23) | ((model.input[12 + model.inpos] & 63) << (15 - 6));
    model.output[26 + model.outpos] = (model.input[12 + model.inpos] >>> 6) & 32767;
    model.output[27 + model.outpos] =
        (model.input[12 + model.inpos] >>> 21) | ((model.input[13 + model.inpos] & 15) << (15 - 4));
    model.output[28 + model.outpos] = (model.input[13 + model.inpos] >>> 4) & 32767;
    model.output[29 + model.outpos] =
        (model.input[13 + model.inpos] >>> 19) | ((model.input[14 + model.inpos] & 3) << (15 - 2));
    model.output[30 + model.outpos] = (model.input[14 + model.inpos] >>> 2) & 32767;
    model.output[31 + model.outpos] = model.input[14 + model.inpos] >>> 17;
}

function fastunpack16(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 65535;
    model.output[1 + model.outpos] = model.input[model.inpos] >>> 16;
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 0) & 65535;
    model.output[3 + model.outpos] = model.input[1 + model.inpos] >>> 16;
    model.output[4 + model.outpos] = (model.input[2 + model.inpos] >>> 0) & 65535;
    model.output[5 + model.outpos] = model.input[2 + model.inpos] >>> 16;
    model.output[6 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 65535;
    model.output[7 + model.outpos] = model.input[3 + model.inpos] >>> 16;
    model.output[8 + model.outpos] = (model.input[4 + model.inpos] >>> 0) & 65535;
    model.output[9 + model.outpos] = model.input[4 + model.inpos] >>> 16;
    model.output[10 + model.outpos] = (model.input[5 + model.inpos] >>> 0) & 65535;
    model.output[11 + model.outpos] = model.input[5 + model.inpos] >>> 16;
    model.output[12 + model.outpos] = (model.input[6 + model.inpos] >>> 0) & 65535;
    model.output[13 + model.outpos] = model.input[6 + model.inpos] >>> 16;
    model.output[14 + model.outpos] = (model.input[7 + model.inpos] >>> 0) & 65535;
    model.output[15 + model.outpos] = model.input[7 + model.inpos] >>> 16;
    model.output[16 + model.outpos] = (model.input[8 + model.inpos] >>> 0) & 65535;
    model.output[17 + model.outpos] = model.input[8 + model.inpos] >>> 16;
    model.output[18 + model.outpos] = (model.input[9 + model.inpos] >>> 0) & 65535;
    model.output[19 + model.outpos] = model.input[9 + model.inpos] >>> 16;
    model.output[20 + model.outpos] = (model.input[10 + model.inpos] >>> 0) & 65535;
    model.output[21 + model.outpos] = model.input[10 + model.inpos] >>> 16;
    model.output[22 + model.outpos] = (model.input[11 + model.inpos] >>> 0) & 65535;
    model.output[23 + model.outpos] = model.input[11 + model.inpos] >>> 16;
    model.output[24 + model.outpos] = (model.input[12 + model.inpos] >>> 0) & 65535;
    model.output[25 + model.outpos] = model.input[12 + model.inpos] >>> 16;
    model.output[26 + model.outpos] = (model.input[13 + model.inpos] >>> 0) & 65535;
    model.output[27 + model.outpos] = model.input[13 + model.inpos] >>> 16;
    model.output[28 + model.outpos] = (model.input[14 + model.inpos] >>> 0) & 65535;
    model.output[29 + model.outpos] = model.input[14 + model.inpos] >>> 16;
    model.output[30 + model.outpos] = (model.input[15 + model.inpos] >>> 0) & 65535;
    model.output[31 + model.outpos] = model.input[15 + model.inpos] >>> 16;
}

function fastunpack17(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 131071;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 17) | ((model.input[1 + model.inpos] & 3) << (17 - 2));
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 2) & 131071;
    model.output[3 + model.outpos] =
        (model.input[1 + model.inpos] >>> 19) | ((model.input[2 + model.inpos] & 15) << (17 - 4));
    model.output[4 + model.outpos] = (model.input[2 + model.inpos] >>> 4) & 131071;
    model.output[5 + model.outpos] =
        (model.input[2 + model.inpos] >>> 21) | ((model.input[3 + model.inpos] & 63) << (17 - 6));
    model.output[6 + model.outpos] = (model.input[3 + model.inpos] >>> 6) & 131071;
    model.output[7 + model.outpos] =
        (model.input[3 + model.inpos] >>> 23) | ((model.input[4 + model.inpos] & 255) << (17 - 8));
    model.output[8 + model.outpos] = (model.input[4 + model.inpos] >>> 8) & 131071;
    model.output[9 + model.outpos] =
        (model.input[4 + model.inpos] >>> 25) | ((model.input[5 + model.inpos] & 1023) << (17 - 10));
    model.output[10 + model.outpos] = (model.input[5 + model.inpos] >>> 10) & 131071;
    model.output[11 + model.outpos] =
        (model.input[5 + model.inpos] >>> 27) | ((model.input[6 + model.inpos] & 4095) << (17 - 12));
    model.output[12 + model.outpos] = (model.input[6 + model.inpos] >>> 12) & 131071;
    model.output[13 + model.outpos] =
        (model.input[6 + model.inpos] >>> 29) | ((model.input[7 + model.inpos] & 16383) << (17 - 14));
    model.output[14 + model.outpos] = (model.input[7 + model.inpos] >>> 14) & 131071;
    model.output[15 + model.outpos] =
        (model.input[7 + model.inpos] >>> 31) | ((model.input[8 + model.inpos] & 65535) << (17 - 16));
    model.output[16 + model.outpos] =
        (model.input[8 + model.inpos] >>> 16) | ((model.input[9 + model.inpos] & 1) << (17 - 1));
    model.output[17 + model.outpos] = (model.input[9 + model.inpos] >>> 1) & 131071;
    model.output[18 + model.outpos] =
        (model.input[9 + model.inpos] >>> 18) | ((model.input[10 + model.inpos] & 7) << (17 - 3));
    model.output[19 + model.outpos] = (model.input[10 + model.inpos] >>> 3) & 131071;
    model.output[20 + model.outpos] =
        (model.input[10 + model.inpos] >>> 20) | ((model.input[11 + model.inpos] & 31) << (17 - 5));
    model.output[21 + model.outpos] = (model.input[11 + model.inpos] >>> 5) & 131071;
    model.output[22 + model.outpos] =
        (model.input[11 + model.inpos] >>> 22) | ((model.input[12 + model.inpos] & 127) << (17 - 7));
    model.output[23 + model.outpos] = (model.input[12 + model.inpos] >>> 7) & 131071;
    model.output[24 + model.outpos] =
        (model.input[12 + model.inpos] >>> 24) | ((model.input[13 + model.inpos] & 511) << (17 - 9));
    model.output[25 + model.outpos] = (model.input[13 + model.inpos] >>> 9) & 131071;
    model.output[26 + model.outpos] =
        (model.input[13 + model.inpos] >>> 26) | ((model.input[14 + model.inpos] & 2047) << (17 - 11));
    model.output[27 + model.outpos] = (model.input[14 + model.inpos] >>> 11) & 131071;
    model.output[28 + model.outpos] =
        (model.input[14 + model.inpos] >>> 28) | ((model.input[15 + model.inpos] & 8191) << (17 - 13));
    model.output[29 + model.outpos] = (model.input[15 + model.inpos] >>> 13) & 131071;
    model.output[30 + model.outpos] =
        (model.input[15 + model.inpos] >>> 30) | ((model.input[16 + model.inpos] & 32767) << (17 - 15));
    model.output[31 + model.outpos] = model.input[16 + model.inpos] >>> 15;
}

function fastunpack18(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 262143;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 18) | ((model.input[1 + model.inpos] & 15) << (18 - 4));
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 262143;
    model.output[3 + model.outpos] =
        (model.input[1 + model.inpos] >>> 22) | ((model.input[2 + model.inpos] & 255) << (18 - 8));
    model.output[4 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 262143;
    model.output[5 + model.outpos] =
        (model.input[2 + model.inpos] >>> 26) | ((model.input[3 + model.inpos] & 4095) << (18 - 12));
    model.output[6 + model.outpos] = (model.input[3 + model.inpos] >>> 12) & 262143;
    model.output[7 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 65535) << (18 - 16));
    model.output[8 + model.outpos] =
        (model.input[4 + model.inpos] >>> 16) | ((model.input[5 + model.inpos] & 3) << (18 - 2));
    model.output[9 + model.outpos] = (model.input[5 + model.inpos] >>> 2) & 262143;
    model.output[10 + model.outpos] =
        (model.input[5 + model.inpos] >>> 20) | ((model.input[6 + model.inpos] & 63) << (18 - 6));
    model.output[11 + model.outpos] = (model.input[6 + model.inpos] >>> 6) & 262143;
    model.output[12 + model.outpos] =
        (model.input[6 + model.inpos] >>> 24) | ((model.input[7 + model.inpos] & 1023) << (18 - 10));
    model.output[13 + model.outpos] = (model.input[7 + model.inpos] >>> 10) & 262143;
    model.output[14 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 16383) << (18 - 14));
    model.output[15 + model.outpos] = model.input[8 + model.inpos] >>> 14;
    model.output[16 + model.outpos] = (model.input[9 + model.inpos] >>> 0) & 262143;
    model.output[17 + model.outpos] =
        (model.input[9 + model.inpos] >>> 18) | ((model.input[10 + model.inpos] & 15) << (18 - 4));
    model.output[18 + model.outpos] = (model.input[10 + model.inpos] >>> 4) & 262143;
    model.output[19 + model.outpos] =
        (model.input[10 + model.inpos] >>> 22) | ((model.input[11 + model.inpos] & 255) << (18 - 8));
    model.output[20 + model.outpos] = (model.input[11 + model.inpos] >>> 8) & 262143;
    model.output[21 + model.outpos] =
        (model.input[11 + model.inpos] >>> 26) | ((model.input[12 + model.inpos] & 4095) << (18 - 12));
    model.output[22 + model.outpos] = (model.input[12 + model.inpos] >>> 12) & 262143;
    model.output[23 + model.outpos] =
        (model.input[12 + model.inpos] >>> 30) | ((model.input[13 + model.inpos] & 65535) << (18 - 16));
    model.output[24 + model.outpos] =
        (model.input[13 + model.inpos] >>> 16) | ((model.input[14 + model.inpos] & 3) << (18 - 2));
    model.output[25 + model.outpos] = (model.input[14 + model.inpos] >>> 2) & 262143;
    model.output[26 + model.outpos] =
        (model.input[14 + model.inpos] >>> 20) | ((model.input[15 + model.inpos] & 63) << (18 - 6));
    model.output[27 + model.outpos] = (model.input[15 + model.inpos] >>> 6) & 262143;
    model.output[28 + model.outpos] =
        (model.input[15 + model.inpos] >>> 24) | ((model.input[16 + model.inpos] & 1023) << (18 - 10));
    model.output[29 + model.outpos] = (model.input[16 + model.inpos] >>> 10) & 262143;
    model.output[30 + model.outpos] =
        (model.input[16 + model.inpos] >>> 28) | ((model.input[17 + model.inpos] & 16383) << (18 - 14));
    model.output[31 + model.outpos] = model.input[17 + model.inpos] >>> 14;
}

function fastunpack19(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 524287;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 19) | ((model.input[1 + model.inpos] & 63) << (19 - 6));
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 6) & 524287;
    model.output[3 + model.outpos] =
        (model.input[1 + model.inpos] >>> 25) | ((model.input[2 + model.inpos] & 4095) << (19 - 12));
    model.output[4 + model.outpos] = (model.input[2 + model.inpos] >>> 12) & 524287;
    model.output[5 + model.outpos] =
        (model.input[2 + model.inpos] >>> 31) | ((model.input[3 + model.inpos] & 262143) << (19 - 18));
    model.output[6 + model.outpos] =
        (model.input[3 + model.inpos] >>> 18) | ((model.input[4 + model.inpos] & 31) << (19 - 5));
    model.output[7 + model.outpos] = (model.input[4 + model.inpos] >>> 5) & 524287;
    model.output[8 + model.outpos] =
        (model.input[4 + model.inpos] >>> 24) | ((model.input[5 + model.inpos] & 2047) << (19 - 11));
    model.output[9 + model.outpos] = (model.input[5 + model.inpos] >>> 11) & 524287;
    model.output[10 + model.outpos] =
        (model.input[5 + model.inpos] >>> 30) | ((model.input[6 + model.inpos] & 131071) << (19 - 17));
    model.output[11 + model.outpos] =
        (model.input[6 + model.inpos] >>> 17) | ((model.input[7 + model.inpos] & 15) << (19 - 4));
    model.output[12 + model.outpos] = (model.input[7 + model.inpos] >>> 4) & 524287;
    model.output[13 + model.outpos] =
        (model.input[7 + model.inpos] >>> 23) | ((model.input[8 + model.inpos] & 1023) << (19 - 10));
    model.output[14 + model.outpos] = (model.input[8 + model.inpos] >>> 10) & 524287;
    model.output[15 + model.outpos] =
        (model.input[8 + model.inpos] >>> 29) | ((model.input[9 + model.inpos] & 65535) << (19 - 16));
    model.output[16 + model.outpos] =
        (model.input[9 + model.inpos] >>> 16) | ((model.input[10 + model.inpos] & 7) << (19 - 3));
    model.output[17 + model.outpos] = (model.input[10 + model.inpos] >>> 3) & 524287;
    model.output[18 + model.outpos] =
        (model.input[10 + model.inpos] >>> 22) | ((model.input[11 + model.inpos] & 511) << (19 - 9));
    model.output[19 + model.outpos] = (model.input[11 + model.inpos] >>> 9) & 524287;
    model.output[20 + model.outpos] =
        (model.input[11 + model.inpos] >>> 28) | ((model.input[12 + model.inpos] & 32767) << (19 - 15));
    model.output[21 + model.outpos] =
        (model.input[12 + model.inpos] >>> 15) | ((model.input[13 + model.inpos] & 3) << (19 - 2));
    model.output[22 + model.outpos] = (model.input[13 + model.inpos] >>> 2) & 524287;
    model.output[23 + model.outpos] =
        (model.input[13 + model.inpos] >>> 21) | ((model.input[14 + model.inpos] & 255) << (19 - 8));
    model.output[24 + model.outpos] = (model.input[14 + model.inpos] >>> 8) & 524287;
    model.output[25 + model.outpos] =
        (model.input[14 + model.inpos] >>> 27) | ((model.input[15 + model.inpos] & 16383) << (19 - 14));
    model.output[26 + model.outpos] =
        (model.input[15 + model.inpos] >>> 14) | ((model.input[16 + model.inpos] & 1) << (19 - 1));
    model.output[27 + model.outpos] = (model.input[16 + model.inpos] >>> 1) & 524287;
    model.output[28 + model.outpos] =
        (model.input[16 + model.inpos] >>> 20) | ((model.input[17 + model.inpos] & 127) << (19 - 7));
    model.output[29 + model.outpos] = (model.input[17 + model.inpos] >>> 7) & 524287;
    model.output[30 + model.outpos] =
        (model.input[17 + model.inpos] >>> 26) | ((model.input[18 + model.inpos] & 8191) << (19 - 13));
    model.output[31 + model.outpos] = model.input[18 + model.inpos] >>> 13;
}

function fastunpack2(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 3;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 2) & 3;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 4) & 3;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 6) & 3;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 8) & 3;
    model.output[5 + model.outpos] = (model.input[model.inpos] >>> 10) & 3;
    model.output[6 + model.outpos] = (model.input[model.inpos] >>> 12) & 3;
    model.output[7 + model.outpos] = (model.input[model.inpos] >>> 14) & 3;
    model.output[8 + model.outpos] = (model.input[model.inpos] >>> 16) & 3;
    model.output[9 + model.outpos] = (model.input[model.inpos] >>> 18) & 3;
    model.output[10 + model.outpos] = (model.input[model.inpos] >>> 20) & 3;
    model.output[11 + model.outpos] = (model.input[model.inpos] >>> 22) & 3;
    model.output[12 + model.outpos] = (model.input[model.inpos] >>> 24) & 3;
    model.output[13 + model.outpos] = (model.input[model.inpos] >>> 26) & 3;
    model.output[14 + model.outpos] = (model.input[model.inpos] >>> 28) & 3;
    model.output[15 + model.outpos] = model.input[model.inpos] >>> 30;
    model.output[16 + model.outpos] = (model.input[1 + model.inpos] >>> 0) & 3;
    model.output[17 + model.outpos] = (model.input[1 + model.inpos] >>> 2) & 3;
    model.output[18 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 3;
    model.output[19 + model.outpos] = (model.input[1 + model.inpos] >>> 6) & 3;
    model.output[20 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 3;
    model.output[21 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 3;
    model.output[22 + model.outpos] = (model.input[1 + model.inpos] >>> 12) & 3;
    model.output[23 + model.outpos] = (model.input[1 + model.inpos] >>> 14) & 3;
    model.output[24 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 3;
    model.output[25 + model.outpos] = (model.input[1 + model.inpos] >>> 18) & 3;
    model.output[26 + model.outpos] = (model.input[1 + model.inpos] >>> 20) & 3;
    model.output[27 + model.outpos] = (model.input[1 + model.inpos] >>> 22) & 3;
    model.output[28 + model.outpos] = (model.input[1 + model.inpos] >>> 24) & 3;
    model.output[29 + model.outpos] = (model.input[1 + model.inpos] >>> 26) & 3;
    model.output[30 + model.outpos] = (model.input[1 + model.inpos] >>> 28) & 3;
    model.output[31 + model.outpos] = model.input[1 + model.inpos] >>> 30;
}

function fastunpack20(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 1048575;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 20) | ((model.input[1 + model.inpos] & 255) << (20 - 8));
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 1048575;
    model.output[3 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 65535) << (20 - 16));
    model.output[4 + model.outpos] =
        (model.input[2 + model.inpos] >>> 16) | ((model.input[3 + model.inpos] & 15) << (20 - 4));
    model.output[5 + model.outpos] = (model.input[3 + model.inpos] >>> 4) & 1048575;
    model.output[6 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 4095) << (20 - 12));
    model.output[7 + model.outpos] = model.input[4 + model.inpos] >>> 12;
    model.output[8 + model.outpos] = (model.input[5 + model.inpos] >>> 0) & 1048575;
    model.output[9 + model.outpos] =
        (model.input[5 + model.inpos] >>> 20) | ((model.input[6 + model.inpos] & 255) << (20 - 8));
    model.output[10 + model.outpos] = (model.input[6 + model.inpos] >>> 8) & 1048575;
    model.output[11 + model.outpos] =
        (model.input[6 + model.inpos] >>> 28) | ((model.input[7 + model.inpos] & 65535) << (20 - 16));
    model.output[12 + model.outpos] =
        (model.input[7 + model.inpos] >>> 16) | ((model.input[8 + model.inpos] & 15) << (20 - 4));
    model.output[13 + model.outpos] = (model.input[8 + model.inpos] >>> 4) & 1048575;
    model.output[14 + model.outpos] =
        (model.input[8 + model.inpos] >>> 24) | ((model.input[9 + model.inpos] & 4095) << (20 - 12));
    model.output[15 + model.outpos] = model.input[9 + model.inpos] >>> 12;
    model.output[16 + model.outpos] = (model.input[10 + model.inpos] >>> 0) & 1048575;
    model.output[17 + model.outpos] =
        (model.input[10 + model.inpos] >>> 20) | ((model.input[11 + model.inpos] & 255) << (20 - 8));
    model.output[18 + model.outpos] = (model.input[11 + model.inpos] >>> 8) & 1048575;
    model.output[19 + model.outpos] =
        (model.input[11 + model.inpos] >>> 28) | ((model.input[12 + model.inpos] & 65535) << (20 - 16));
    model.output[20 + model.outpos] =
        (model.input[12 + model.inpos] >>> 16) | ((model.input[13 + model.inpos] & 15) << (20 - 4));
    model.output[21 + model.outpos] = (model.input[13 + model.inpos] >>> 4) & 1048575;
    model.output[22 + model.outpos] =
        (model.input[13 + model.inpos] >>> 24) | ((model.input[14 + model.inpos] & 4095) << (20 - 12));
    model.output[23 + model.outpos] = model.input[14 + model.inpos] >>> 12;
    model.output[24 + model.outpos] = (model.input[15 + model.inpos] >>> 0) & 1048575;
    model.output[25 + model.outpos] =
        (model.input[15 + model.inpos] >>> 20) | ((model.input[16 + model.inpos] & 255) << (20 - 8));
    model.output[26 + model.outpos] = (model.input[16 + model.inpos] >>> 8) & 1048575;
    model.output[27 + model.outpos] =
        (model.input[16 + model.inpos] >>> 28) | ((model.input[17 + model.inpos] & 65535) << (20 - 16));
    model.output[28 + model.outpos] =
        (model.input[17 + model.inpos] >>> 16) | ((model.input[18 + model.inpos] & 15) << (20 - 4));
    model.output[29 + model.outpos] = (model.input[18 + model.inpos] >>> 4) & 1048575;
    model.output[30 + model.outpos] =
        (model.input[18 + model.inpos] >>> 24) | ((model.input[19 + model.inpos] & 4095) << (20 - 12));
    model.output[31 + model.outpos] = model.input[19 + model.inpos] >>> 12;
}

function fastunpack21(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 2097151;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 21) | ((model.input[1 + model.inpos] & 1023) << (21 - 10));
    model.output[2 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 2097151;
    model.output[3 + model.outpos] =
        (model.input[1 + model.inpos] >>> 31) | ((model.input[2 + model.inpos] & 1048575) << (21 - 20));
    model.output[4 + model.outpos] =
        (model.input[2 + model.inpos] >>> 20) | ((model.input[3 + model.inpos] & 511) << (21 - 9));
    model.output[5 + model.outpos] = (model.input[3 + model.inpos] >>> 9) & 2097151;
    model.output[6 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 524287) << (21 - 19));
    model.output[7 + model.outpos] =
        (model.input[4 + model.inpos] >>> 19) | ((model.input[5 + model.inpos] & 255) << (21 - 8));
    model.output[8 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 2097151;
    model.output[9 + model.outpos] =
        (model.input[5 + model.inpos] >>> 29) | ((model.input[6 + model.inpos] & 262143) << (21 - 18));
    model.output[10 + model.outpos] =
        (model.input[6 + model.inpos] >>> 18) | ((model.input[7 + model.inpos] & 127) << (21 - 7));
    model.output[11 + model.outpos] = (model.input[7 + model.inpos] >>> 7) & 2097151;
    model.output[12 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 131071) << (21 - 17));
    model.output[13 + model.outpos] =
        (model.input[8 + model.inpos] >>> 17) | ((model.input[9 + model.inpos] & 63) << (21 - 6));
    model.output[14 + model.outpos] = (model.input[9 + model.inpos] >>> 6) & 2097151;
    model.output[15 + model.outpos] =
        (model.input[9 + model.inpos] >>> 27) | ((model.input[10 + model.inpos] & 65535) << (21 - 16));
    model.output[16 + model.outpos] =
        (model.input[10 + model.inpos] >>> 16) | ((model.input[11 + model.inpos] & 31) << (21 - 5));
    model.output[17 + model.outpos] = (model.input[11 + model.inpos] >>> 5) & 2097151;
    model.output[18 + model.outpos] =
        (model.input[11 + model.inpos] >>> 26) | ((model.input[12 + model.inpos] & 32767) << (21 - 15));
    model.output[19 + model.outpos] =
        (model.input[12 + model.inpos] >>> 15) | ((model.input[13 + model.inpos] & 15) << (21 - 4));
    model.output[20 + model.outpos] = (model.input[13 + model.inpos] >>> 4) & 2097151;
    model.output[21 + model.outpos] =
        (model.input[13 + model.inpos] >>> 25) | ((model.input[14 + model.inpos] & 16383) << (21 - 14));
    model.output[22 + model.outpos] =
        (model.input[14 + model.inpos] >>> 14) | ((model.input[15 + model.inpos] & 7) << (21 - 3));
    model.output[23 + model.outpos] = (model.input[15 + model.inpos] >>> 3) & 2097151;
    model.output[24 + model.outpos] =
        (model.input[15 + model.inpos] >>> 24) | ((model.input[16 + model.inpos] & 8191) << (21 - 13));
    model.output[25 + model.outpos] =
        (model.input[16 + model.inpos] >>> 13) | ((model.input[17 + model.inpos] & 3) << (21 - 2));
    model.output[26 + model.outpos] = (model.input[17 + model.inpos] >>> 2) & 2097151;
    model.output[27 + model.outpos] =
        (model.input[17 + model.inpos] >>> 23) | ((model.input[18 + model.inpos] & 4095) << (21 - 12));
    model.output[28 + model.outpos] =
        (model.input[18 + model.inpos] >>> 12) | ((model.input[19 + model.inpos] & 1) << (21 - 1));
    model.output[29 + model.outpos] = (model.input[19 + model.inpos] >>> 1) & 2097151;
    model.output[30 + model.outpos] =
        (model.input[19 + model.inpos] >>> 22) | ((model.input[20 + model.inpos] & 2047) << (21 - 11));
    model.output[31 + model.outpos] = model.input[20 + model.inpos] >>> 11;
}

function fastunpack22(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 4194303;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 22) | ((model.input[1 + model.inpos] & 4095) << (22 - 12));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 12) | ((model.input[2 + model.inpos] & 3) << (22 - 2));
    model.output[3 + model.outpos] = (model.input[2 + model.inpos] >>> 2) & 4194303;
    model.output[4 + model.outpos] =
        (model.input[2 + model.inpos] >>> 24) | ((model.input[3 + model.inpos] & 16383) << (22 - 14));
    model.output[5 + model.outpos] =
        (model.input[3 + model.inpos] >>> 14) | ((model.input[4 + model.inpos] & 15) << (22 - 4));
    model.output[6 + model.outpos] = (model.input[4 + model.inpos] >>> 4) & 4194303;
    model.output[7 + model.outpos] =
        (model.input[4 + model.inpos] >>> 26) | ((model.input[5 + model.inpos] & 65535) << (22 - 16));
    model.output[8 + model.outpos] =
        (model.input[5 + model.inpos] >>> 16) | ((model.input[6 + model.inpos] & 63) << (22 - 6));
    model.output[9 + model.outpos] = (model.input[6 + model.inpos] >>> 6) & 4194303;
    model.output[10 + model.outpos] =
        (model.input[6 + model.inpos] >>> 28) | ((model.input[7 + model.inpos] & 262143) << (22 - 18));
    model.output[11 + model.outpos] =
        (model.input[7 + model.inpos] >>> 18) | ((model.input[8 + model.inpos] & 255) << (22 - 8));
    model.output[12 + model.outpos] = (model.input[8 + model.inpos] >>> 8) & 4194303;
    model.output[13 + model.outpos] =
        (model.input[8 + model.inpos] >>> 30) | ((model.input[9 + model.inpos] & 1048575) << (22 - 20));
    model.output[14 + model.outpos] =
        (model.input[9 + model.inpos] >>> 20) | ((model.input[10 + model.inpos] & 1023) << (22 - 10));
    model.output[15 + model.outpos] = model.input[10 + model.inpos] >>> 10;
    model.output[16 + model.outpos] = (model.input[11 + model.inpos] >>> 0) & 4194303;
    model.output[17 + model.outpos] =
        (model.input[11 + model.inpos] >>> 22) | ((model.input[12 + model.inpos] & 4095) << (22 - 12));
    model.output[18 + model.outpos] =
        (model.input[12 + model.inpos] >>> 12) | ((model.input[13 + model.inpos] & 3) << (22 - 2));
    model.output[19 + model.outpos] = (model.input[13 + model.inpos] >>> 2) & 4194303;
    model.output[20 + model.outpos] =
        (model.input[13 + model.inpos] >>> 24) | ((model.input[14 + model.inpos] & 16383) << (22 - 14));
    model.output[21 + model.outpos] =
        (model.input[14 + model.inpos] >>> 14) | ((model.input[15 + model.inpos] & 15) << (22 - 4));
    model.output[22 + model.outpos] = (model.input[15 + model.inpos] >>> 4) & 4194303;
    model.output[23 + model.outpos] =
        (model.input[15 + model.inpos] >>> 26) | ((model.input[16 + model.inpos] & 65535) << (22 - 16));
    model.output[24 + model.outpos] =
        (model.input[16 + model.inpos] >>> 16) | ((model.input[17 + model.inpos] & 63) << (22 - 6));
    model.output[25 + model.outpos] = (model.input[17 + model.inpos] >>> 6) & 4194303;
    model.output[26 + model.outpos] =
        (model.input[17 + model.inpos] >>> 28) | ((model.input[18 + model.inpos] & 262143) << (22 - 18));
    model.output[27 + model.outpos] =
        (model.input[18 + model.inpos] >>> 18) | ((model.input[19 + model.inpos] & 255) << (22 - 8));
    model.output[28 + model.outpos] = (model.input[19 + model.inpos] >>> 8) & 4194303;
    model.output[29 + model.outpos] =
        (model.input[19 + model.inpos] >>> 30) | ((model.input[20 + model.inpos] & 1048575) << (22 - 20));
    model.output[30 + model.outpos] =
        (model.input[20 + model.inpos] >>> 20) | ((model.input[21 + model.inpos] & 1023) << (22 - 10));
    model.output[31 + model.outpos] = model.input[21 + model.inpos] >>> 10;
}

function fastunpack23(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 8388607;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 23) | ((model.input[1 + model.inpos] & 16383) << (23 - 14));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 14) | ((model.input[2 + model.inpos] & 31) << (23 - 5));
    model.output[3 + model.outpos] = (model.input[2 + model.inpos] >>> 5) & 8388607;
    model.output[4 + model.outpos] =
        (model.input[2 + model.inpos] >>> 28) | ((model.input[3 + model.inpos] & 524287) << (23 - 19));
    model.output[5 + model.outpos] =
        (model.input[3 + model.inpos] >>> 19) | ((model.input[4 + model.inpos] & 1023) << (23 - 10));
    model.output[6 + model.outpos] =
        (model.input[4 + model.inpos] >>> 10) | ((model.input[5 + model.inpos] & 1) << (23 - 1));
    model.output[7 + model.outpos] = (model.input[5 + model.inpos] >>> 1) & 8388607;
    model.output[8 + model.outpos] =
        (model.input[5 + model.inpos] >>> 24) | ((model.input[6 + model.inpos] & 32767) << (23 - 15));
    model.output[9 + model.outpos] =
        (model.input[6 + model.inpos] >>> 15) | ((model.input[7 + model.inpos] & 63) << (23 - 6));
    model.output[10 + model.outpos] = (model.input[7 + model.inpos] >>> 6) & 8388607;
    model.output[11 + model.outpos] =
        (model.input[7 + model.inpos] >>> 29) | ((model.input[8 + model.inpos] & 1048575) << (23 - 20));
    model.output[12 + model.outpos] =
        (model.input[8 + model.inpos] >>> 20) | ((model.input[9 + model.inpos] & 2047) << (23 - 11));
    model.output[13 + model.outpos] =
        (model.input[9 + model.inpos] >>> 11) | ((model.input[10 + model.inpos] & 3) << (23 - 2));
    model.output[14 + model.outpos] = (model.input[10 + model.inpos] >>> 2) & 8388607;
    model.output[15 + model.outpos] =
        (model.input[10 + model.inpos] >>> 25) | ((model.input[11 + model.inpos] & 65535) << (23 - 16));
    model.output[16 + model.outpos] =
        (model.input[11 + model.inpos] >>> 16) | ((model.input[12 + model.inpos] & 127) << (23 - 7));
    model.output[17 + model.outpos] = (model.input[12 + model.inpos] >>> 7) & 8388607;
    model.output[18 + model.outpos] =
        (model.input[12 + model.inpos] >>> 30) | ((model.input[13 + model.inpos] & 2097151) << (23 - 21));
    model.output[19 + model.outpos] =
        (model.input[13 + model.inpos] >>> 21) | ((model.input[14 + model.inpos] & 4095) << (23 - 12));
    model.output[20 + model.outpos] =
        (model.input[14 + model.inpos] >>> 12) | ((model.input[15 + model.inpos] & 7) << (23 - 3));
    model.output[21 + model.outpos] = (model.input[15 + model.inpos] >>> 3) & 8388607;
    model.output[22 + model.outpos] =
        (model.input[15 + model.inpos] >>> 26) | ((model.input[16 + model.inpos] & 131071) << (23 - 17));
    model.output[23 + model.outpos] =
        (model.input[16 + model.inpos] >>> 17) | ((model.input[17 + model.inpos] & 255) << (23 - 8));
    model.output[24 + model.outpos] = (model.input[17 + model.inpos] >>> 8) & 8388607;
    model.output[25 + model.outpos] =
        (model.input[17 + model.inpos] >>> 31) | ((model.input[18 + model.inpos] & 4194303) << (23 - 22));
    model.output[26 + model.outpos] =
        (model.input[18 + model.inpos] >>> 22) | ((model.input[19 + model.inpos] & 8191) << (23 - 13));
    model.output[27 + model.outpos] =
        (model.input[19 + model.inpos] >>> 13) | ((model.input[20 + model.inpos] & 15) << (23 - 4));
    model.output[28 + model.outpos] = (model.input[20 + model.inpos] >>> 4) & 8388607;
    model.output[29 + model.outpos] =
        (model.input[20 + model.inpos] >>> 27) | ((model.input[21 + model.inpos] & 262143) << (23 - 18));
    model.output[30 + model.outpos] =
        (model.input[21 + model.inpos] >>> 18) | ((model.input[22 + model.inpos] & 511) << (23 - 9));
    model.output[31 + model.outpos] = model.input[22 + model.inpos] >>> 9;
}

function fastunpack24(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 16777215;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 24) | ((model.input[1 + model.inpos] & 65535) << (24 - 16));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 16) | ((model.input[2 + model.inpos] & 255) << (24 - 8));
    model.output[3 + model.outpos] = model.input[2 + model.inpos] >>> 8;
    model.output[4 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 16777215;
    model.output[5 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 65535) << (24 - 16));
    model.output[6 + model.outpos] =
        (model.input[4 + model.inpos] >>> 16) | ((model.input[5 + model.inpos] & 255) << (24 - 8));
    model.output[7 + model.outpos] = model.input[5 + model.inpos] >>> 8;
    model.output[8 + model.outpos] = (model.input[6 + model.inpos] >>> 0) & 16777215;
    model.output[9 + model.outpos] =
        (model.input[6 + model.inpos] >>> 24) | ((model.input[7 + model.inpos] & 65535) << (24 - 16));
    model.output[10 + model.outpos] =
        (model.input[7 + model.inpos] >>> 16) | ((model.input[8 + model.inpos] & 255) << (24 - 8));
    model.output[11 + model.outpos] = model.input[8 + model.inpos] >>> 8;
    model.output[12 + model.outpos] = (model.input[9 + model.inpos] >>> 0) & 16777215;
    model.output[13 + model.outpos] =
        (model.input[9 + model.inpos] >>> 24) | ((model.input[10 + model.inpos] & 65535) << (24 - 16));
    model.output[14 + model.outpos] =
        (model.input[10 + model.inpos] >>> 16) | ((model.input[11 + model.inpos] & 255) << (24 - 8));
    model.output[15 + model.outpos] = model.input[11 + model.inpos] >>> 8;
    model.output[16 + model.outpos] = (model.input[12 + model.inpos] >>> 0) & 16777215;
    model.output[17 + model.outpos] =
        (model.input[12 + model.inpos] >>> 24) | ((model.input[13 + model.inpos] & 65535) << (24 - 16));
    model.output[18 + model.outpos] =
        (model.input[13 + model.inpos] >>> 16) | ((model.input[14 + model.inpos] & 255) << (24 - 8));
    model.output[19 + model.outpos] = model.input[14 + model.inpos] >>> 8;
    model.output[20 + model.outpos] = (model.input[15 + model.inpos] >>> 0) & 16777215;
    model.output[21 + model.outpos] =
        (model.input[15 + model.inpos] >>> 24) | ((model.input[16 + model.inpos] & 65535) << (24 - 16));
    model.output[22 + model.outpos] =
        (model.input[16 + model.inpos] >>> 16) | ((model.input[17 + model.inpos] & 255) << (24 - 8));
    model.output[23 + model.outpos] = model.input[17 + model.inpos] >>> 8;
    model.output[24 + model.outpos] = (model.input[18 + model.inpos] >>> 0) & 16777215;
    model.output[25 + model.outpos] =
        (model.input[18 + model.inpos] >>> 24) | ((model.input[19 + model.inpos] & 65535) << (24 - 16));
    model.output[26 + model.outpos] =
        (model.input[19 + model.inpos] >>> 16) | ((model.input[20 + model.inpos] & 255) << (24 - 8));
    model.output[27 + model.outpos] = model.input[20 + model.inpos] >>> 8;
    model.output[28 + model.outpos] = (model.input[21 + model.inpos] >>> 0) & 16777215;
    model.output[29 + model.outpos] =
        (model.input[21 + model.inpos] >>> 24) | ((model.input[22 + model.inpos] & 65535) << (24 - 16));
    model.output[30 + model.outpos] =
        (model.input[22 + model.inpos] >>> 16) | ((model.input[23 + model.inpos] & 255) << (24 - 8));
    model.output[31 + model.outpos] = model.input[23 + model.inpos] >>> 8;
}

function fastunpack25(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 33554431;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 25) | ((model.input[1 + model.inpos] & 262143) << (25 - 18));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 18) | ((model.input[2 + model.inpos] & 2047) << (25 - 11));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 11) | ((model.input[3 + model.inpos] & 15) << (25 - 4));
    model.output[4 + model.outpos] = (model.input[3 + model.inpos] >>> 4) & 33554431;
    model.output[5 + model.outpos] =
        (model.input[3 + model.inpos] >>> 29) | ((model.input[4 + model.inpos] & 4194303) << (25 - 22));
    model.output[6 + model.outpos] =
        (model.input[4 + model.inpos] >>> 22) | ((model.input[5 + model.inpos] & 32767) << (25 - 15));
    model.output[7 + model.outpos] =
        (model.input[5 + model.inpos] >>> 15) | ((model.input[6 + model.inpos] & 255) << (25 - 8));
    model.output[8 + model.outpos] =
        (model.input[6 + model.inpos] >>> 8) | ((model.input[7 + model.inpos] & 1) << (25 - 1));
    model.output[9 + model.outpos] = (model.input[7 + model.inpos] >>> 1) & 33554431;
    model.output[10 + model.outpos] =
        (model.input[7 + model.inpos] >>> 26) | ((model.input[8 + model.inpos] & 524287) << (25 - 19));
    model.output[11 + model.outpos] =
        (model.input[8 + model.inpos] >>> 19) | ((model.input[9 + model.inpos] & 4095) << (25 - 12));
    model.output[12 + model.outpos] =
        (model.input[9 + model.inpos] >>> 12) | ((model.input[10 + model.inpos] & 31) << (25 - 5));
    model.output[13 + model.outpos] = (model.input[10 + model.inpos] >>> 5) & 33554431;
    model.output[14 + model.outpos] =
        (model.input[10 + model.inpos] >>> 30) | ((model.input[11 + model.inpos] & 8388607) << (25 - 23));
    model.output[15 + model.outpos] =
        (model.input[11 + model.inpos] >>> 23) | ((model.input[12 + model.inpos] & 65535) << (25 - 16));
    model.output[16 + model.outpos] =
        (model.input[12 + model.inpos] >>> 16) | ((model.input[13 + model.inpos] & 511) << (25 - 9));
    model.output[17 + model.outpos] =
        (model.input[13 + model.inpos] >>> 9) | ((model.input[14 + model.inpos] & 3) << (25 - 2));
    model.output[18 + model.outpos] = (model.input[14 + model.inpos] >>> 2) & 33554431;
    model.output[19 + model.outpos] =
        (model.input[14 + model.inpos] >>> 27) | ((model.input[15 + model.inpos] & 1048575) << (25 - 20));
    model.output[20 + model.outpos] =
        (model.input[15 + model.inpos] >>> 20) | ((model.input[16 + model.inpos] & 8191) << (25 - 13));
    model.output[21 + model.outpos] =
        (model.input[16 + model.inpos] >>> 13) | ((model.input[17 + model.inpos] & 63) << (25 - 6));
    model.output[22 + model.outpos] = (model.input[17 + model.inpos] >>> 6) & 33554431;
    model.output[23 + model.outpos] =
        (model.input[17 + model.inpos] >>> 31) | ((model.input[18 + model.inpos] & 16777215) << (25 - 24));
    model.output[24 + model.outpos] =
        (model.input[18 + model.inpos] >>> 24) | ((model.input[19 + model.inpos] & 131071) << (25 - 17));
    model.output[25 + model.outpos] =
        (model.input[19 + model.inpos] >>> 17) | ((model.input[20 + model.inpos] & 1023) << (25 - 10));
    model.output[26 + model.outpos] =
        (model.input[20 + model.inpos] >>> 10) | ((model.input[21 + model.inpos] & 7) << (25 - 3));
    model.output[27 + model.outpos] = (model.input[21 + model.inpos] >>> 3) & 33554431;
    model.output[28 + model.outpos] =
        (model.input[21 + model.inpos] >>> 28) | ((model.input[22 + model.inpos] & 2097151) << (25 - 21));
    model.output[29 + model.outpos] =
        (model.input[22 + model.inpos] >>> 21) | ((model.input[23 + model.inpos] & 16383) << (25 - 14));
    model.output[30 + model.outpos] =
        (model.input[23 + model.inpos] >>> 14) | ((model.input[24 + model.inpos] & 127) << (25 - 7));
    model.output[31 + model.outpos] = model.input[24 + model.inpos] >>> 7;
}

function fastunpack26(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 67108863;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 26) | ((model.input[1 + model.inpos] & 1048575) << (26 - 20));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 20) | ((model.input[2 + model.inpos] & 16383) << (26 - 14));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 14) | ((model.input[3 + model.inpos] & 255) << (26 - 8));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 8) | ((model.input[4 + model.inpos] & 3) << (26 - 2));
    model.output[5 + model.outpos] = (model.input[4 + model.inpos] >>> 2) & 67108863;
    model.output[6 + model.outpos] =
        (model.input[4 + model.inpos] >>> 28) | ((model.input[5 + model.inpos] & 4194303) << (26 - 22));
    model.output[7 + model.outpos] =
        (model.input[5 + model.inpos] >>> 22) | ((model.input[6 + model.inpos] & 65535) << (26 - 16));
    model.output[8 + model.outpos] =
        (model.input[6 + model.inpos] >>> 16) | ((model.input[7 + model.inpos] & 1023) << (26 - 10));
    model.output[9 + model.outpos] =
        (model.input[7 + model.inpos] >>> 10) | ((model.input[8 + model.inpos] & 15) << (26 - 4));
    model.output[10 + model.outpos] = (model.input[8 + model.inpos] >>> 4) & 67108863;
    model.output[11 + model.outpos] =
        (model.input[8 + model.inpos] >>> 30) | ((model.input[9 + model.inpos] & 16777215) << (26 - 24));
    model.output[12 + model.outpos] =
        (model.input[9 + model.inpos] >>> 24) | ((model.input[10 + model.inpos] & 262143) << (26 - 18));
    model.output[13 + model.outpos] =
        (model.input[10 + model.inpos] >>> 18) | ((model.input[11 + model.inpos] & 4095) << (26 - 12));
    model.output[14 + model.outpos] =
        (model.input[11 + model.inpos] >>> 12) | ((model.input[12 + model.inpos] & 63) << (26 - 6));
    model.output[15 + model.outpos] = model.input[12 + model.inpos] >>> 6;
    model.output[16 + model.outpos] = (model.input[13 + model.inpos] >>> 0) & 67108863;
    model.output[17 + model.outpos] =
        (model.input[13 + model.inpos] >>> 26) | ((model.input[14 + model.inpos] & 1048575) << (26 - 20));
    model.output[18 + model.outpos] =
        (model.input[14 + model.inpos] >>> 20) | ((model.input[15 + model.inpos] & 16383) << (26 - 14));
    model.output[19 + model.outpos] =
        (model.input[15 + model.inpos] >>> 14) | ((model.input[16 + model.inpos] & 255) << (26 - 8));
    model.output[20 + model.outpos] =
        (model.input[16 + model.inpos] >>> 8) | ((model.input[17 + model.inpos] & 3) << (26 - 2));
    model.output[21 + model.outpos] = (model.input[17 + model.inpos] >>> 2) & 67108863;
    model.output[22 + model.outpos] =
        (model.input[17 + model.inpos] >>> 28) | ((model.input[18 + model.inpos] & 4194303) << (26 - 22));
    model.output[23 + model.outpos] =
        (model.input[18 + model.inpos] >>> 22) | ((model.input[19 + model.inpos] & 65535) << (26 - 16));
    model.output[24 + model.outpos] =
        (model.input[19 + model.inpos] >>> 16) | ((model.input[20 + model.inpos] & 1023) << (26 - 10));
    model.output[25 + model.outpos] =
        (model.input[20 + model.inpos] >>> 10) | ((model.input[21 + model.inpos] & 15) << (26 - 4));
    model.output[26 + model.outpos] = (model.input[21 + model.inpos] >>> 4) & 67108863;
    model.output[27 + model.outpos] =
        (model.input[21 + model.inpos] >>> 30) | ((model.input[22 + model.inpos] & 16777215) << (26 - 24));
    model.output[28 + model.outpos] =
        (model.input[22 + model.inpos] >>> 24) | ((model.input[23 + model.inpos] & 262143) << (26 - 18));
    model.output[29 + model.outpos] =
        (model.input[23 + model.inpos] >>> 18) | ((model.input[24 + model.inpos] & 4095) << (26 - 12));
    model.output[30 + model.outpos] =
        (model.input[24 + model.inpos] >>> 12) | ((model.input[25 + model.inpos] & 63) << (26 - 6));
    model.output[31 + model.outpos] = model.input[25 + model.inpos] >>> 6;
}

function fastunpack27(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 134217727;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 27) | ((model.input[1 + model.inpos] & 4194303) << (27 - 22));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 22) | ((model.input[2 + model.inpos] & 131071) << (27 - 17));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 17) | ((model.input[3 + model.inpos] & 4095) << (27 - 12));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 12) | ((model.input[4 + model.inpos] & 127) << (27 - 7));
    model.output[5 + model.outpos] =
        (model.input[4 + model.inpos] >>> 7) | ((model.input[5 + model.inpos] & 3) << (27 - 2));
    model.output[6 + model.outpos] = (model.input[5 + model.inpos] >>> 2) & 134217727;
    model.output[7 + model.outpos] =
        (model.input[5 + model.inpos] >>> 29) | ((model.input[6 + model.inpos] & 16777215) << (27 - 24));
    model.output[8 + model.outpos] =
        (model.input[6 + model.inpos] >>> 24) | ((model.input[7 + model.inpos] & 524287) << (27 - 19));
    model.output[9 + model.outpos] =
        (model.input[7 + model.inpos] >>> 19) | ((model.input[8 + model.inpos] & 16383) << (27 - 14));
    model.output[10 + model.outpos] =
        (model.input[8 + model.inpos] >>> 14) | ((model.input[9 + model.inpos] & 511) << (27 - 9));
    model.output[11 + model.outpos] =
        (model.input[9 + model.inpos] >>> 9) | ((model.input[10 + model.inpos] & 15) << (27 - 4));
    model.output[12 + model.outpos] = (model.input[10 + model.inpos] >>> 4) & 134217727;
    model.output[13 + model.outpos] =
        (model.input[10 + model.inpos] >>> 31) | ((model.input[11 + model.inpos] & 67108863) << (27 - 26));
    model.output[14 + model.outpos] =
        (model.input[11 + model.inpos] >>> 26) | ((model.input[12 + model.inpos] & 2097151) << (27 - 21));
    model.output[15 + model.outpos] =
        (model.input[12 + model.inpos] >>> 21) | ((model.input[13 + model.inpos] & 65535) << (27 - 16));
    model.output[16 + model.outpos] =
        (model.input[13 + model.inpos] >>> 16) | ((model.input[14 + model.inpos] & 2047) << (27 - 11));
    model.output[17 + model.outpos] =
        (model.input[14 + model.inpos] >>> 11) | ((model.input[15 + model.inpos] & 63) << (27 - 6));
    model.output[18 + model.outpos] =
        (model.input[15 + model.inpos] >>> 6) | ((model.input[16 + model.inpos] & 1) << (27 - 1));
    model.output[19 + model.outpos] = (model.input[16 + model.inpos] >>> 1) & 134217727;
    model.output[20 + model.outpos] =
        (model.input[16 + model.inpos] >>> 28) | ((model.input[17 + model.inpos] & 8388607) << (27 - 23));
    model.output[21 + model.outpos] =
        (model.input[17 + model.inpos] >>> 23) | ((model.input[18 + model.inpos] & 262143) << (27 - 18));
    model.output[22 + model.outpos] =
        (model.input[18 + model.inpos] >>> 18) | ((model.input[19 + model.inpos] & 8191) << (27 - 13));
    model.output[23 + model.outpos] =
        (model.input[19 + model.inpos] >>> 13) | ((model.input[20 + model.inpos] & 255) << (27 - 8));
    model.output[24 + model.outpos] =
        (model.input[20 + model.inpos] >>> 8) | ((model.input[21 + model.inpos] & 7) << (27 - 3));
    model.output[25 + model.outpos] = (model.input[21 + model.inpos] >>> 3) & 134217727;
    model.output[26 + model.outpos] =
        (model.input[21 + model.inpos] >>> 30) | ((model.input[22 + model.inpos] & 33554431) << (27 - 25));
    model.output[27 + model.outpos] =
        (model.input[22 + model.inpos] >>> 25) | ((model.input[23 + model.inpos] & 1048575) << (27 - 20));
    model.output[28 + model.outpos] =
        (model.input[23 + model.inpos] >>> 20) | ((model.input[24 + model.inpos] & 32767) << (27 - 15));
    model.output[29 + model.outpos] =
        (model.input[24 + model.inpos] >>> 15) | ((model.input[25 + model.inpos] & 1023) << (27 - 10));
    model.output[30 + model.outpos] =
        (model.input[25 + model.inpos] >>> 10) | ((model.input[26 + model.inpos] & 31) << (27 - 5));
    model.output[31 + model.outpos] = model.input[26 + model.inpos] >>> 5;
}

function fastunpack28(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 268435455;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 28) | ((model.input[1 + model.inpos] & 16777215) << (28 - 24));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 24) | ((model.input[2 + model.inpos] & 1048575) << (28 - 20));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 20) | ((model.input[3 + model.inpos] & 65535) << (28 - 16));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 16) | ((model.input[4 + model.inpos] & 4095) << (28 - 12));
    model.output[5 + model.outpos] =
        (model.input[4 + model.inpos] >>> 12) | ((model.input[5 + model.inpos] & 255) << (28 - 8));
    model.output[6 + model.outpos] =
        (model.input[5 + model.inpos] >>> 8) | ((model.input[6 + model.inpos] & 15) << (28 - 4));
    model.output[7 + model.outpos] = model.input[6 + model.inpos] >>> 4;
    model.output[8 + model.outpos] = (model.input[7 + model.inpos] >>> 0) & 268435455;
    model.output[9 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 16777215) << (28 - 24));
    model.output[10 + model.outpos] =
        (model.input[8 + model.inpos] >>> 24) | ((model.input[9 + model.inpos] & 1048575) << (28 - 20));
    model.output[11 + model.outpos] =
        (model.input[9 + model.inpos] >>> 20) | ((model.input[10 + model.inpos] & 65535) << (28 - 16));
    model.output[12 + model.outpos] =
        (model.input[10 + model.inpos] >>> 16) | ((model.input[11 + model.inpos] & 4095) << (28 - 12));
    model.output[13 + model.outpos] =
        (model.input[11 + model.inpos] >>> 12) | ((model.input[12 + model.inpos] & 255) << (28 - 8));
    model.output[14 + model.outpos] =
        (model.input[12 + model.inpos] >>> 8) | ((model.input[13 + model.inpos] & 15) << (28 - 4));
    model.output[15 + model.outpos] = model.input[13 + model.inpos] >>> 4;
    model.output[16 + model.outpos] = (model.input[14 + model.inpos] >>> 0) & 268435455;
    model.output[17 + model.outpos] =
        (model.input[14 + model.inpos] >>> 28) | ((model.input[15 + model.inpos] & 16777215) << (28 - 24));
    model.output[18 + model.outpos] =
        (model.input[15 + model.inpos] >>> 24) | ((model.input[16 + model.inpos] & 1048575) << (28 - 20));
    model.output[19 + model.outpos] =
        (model.input[16 + model.inpos] >>> 20) | ((model.input[17 + model.inpos] & 65535) << (28 - 16));
    model.output[20 + model.outpos] =
        (model.input[17 + model.inpos] >>> 16) | ((model.input[18 + model.inpos] & 4095) << (28 - 12));
    model.output[21 + model.outpos] =
        (model.input[18 + model.inpos] >>> 12) | ((model.input[19 + model.inpos] & 255) << (28 - 8));
    model.output[22 + model.outpos] =
        (model.input[19 + model.inpos] >>> 8) | ((model.input[20 + model.inpos] & 15) << (28 - 4));
    model.output[23 + model.outpos] = model.input[20 + model.inpos] >>> 4;
    model.output[24 + model.outpos] = (model.input[21 + model.inpos] >>> 0) & 268435455;
    model.output[25 + model.outpos] =
        (model.input[21 + model.inpos] >>> 28) | ((model.input[22 + model.inpos] & 16777215) << (28 - 24));
    model.output[26 + model.outpos] =
        (model.input[22 + model.inpos] >>> 24) | ((model.input[23 + model.inpos] & 1048575) << (28 - 20));
    model.output[27 + model.outpos] =
        (model.input[23 + model.inpos] >>> 20) | ((model.input[24 + model.inpos] & 65535) << (28 - 16));
    model.output[28 + model.outpos] =
        (model.input[24 + model.inpos] >>> 16) | ((model.input[25 + model.inpos] & 4095) << (28 - 12));
    model.output[29 + model.outpos] =
        (model.input[25 + model.inpos] >>> 12) | ((model.input[26 + model.inpos] & 255) << (28 - 8));
    model.output[30 + model.outpos] =
        (model.input[26 + model.inpos] >>> 8) | ((model.input[27 + model.inpos] & 15) << (28 - 4));
    model.output[31 + model.outpos] = model.input[27 + model.inpos] >>> 4;
}

function fastunpack29(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 536870911;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 29) | ((model.input[1 + model.inpos] & 67108863) << (29 - 26));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 26) | ((model.input[2 + model.inpos] & 8388607) << (29 - 23));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 23) | ((model.input[3 + model.inpos] & 1048575) << (29 - 20));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 20) | ((model.input[4 + model.inpos] & 131071) << (29 - 17));
    model.output[5 + model.outpos] =
        (model.input[4 + model.inpos] >>> 17) | ((model.input[5 + model.inpos] & 16383) << (29 - 14));
    model.output[6 + model.outpos] =
        (model.input[5 + model.inpos] >>> 14) | ((model.input[6 + model.inpos] & 2047) << (29 - 11));
    model.output[7 + model.outpos] =
        (model.input[6 + model.inpos] >>> 11) | ((model.input[7 + model.inpos] & 255) << (29 - 8));
    model.output[8 + model.outpos] =
        (model.input[7 + model.inpos] >>> 8) | ((model.input[8 + model.inpos] & 31) << (29 - 5));
    model.output[9 + model.outpos] =
        (model.input[8 + model.inpos] >>> 5) | ((model.input[9 + model.inpos] & 3) << (29 - 2));
    model.output[10 + model.outpos] = (model.input[9 + model.inpos] >>> 2) & 536870911;
    model.output[11 + model.outpos] =
        (model.input[9 + model.inpos] >>> 31) | ((model.input[10 + model.inpos] & 268435455) << (29 - 28));
    model.output[12 + model.outpos] =
        (model.input[10 + model.inpos] >>> 28) | ((model.input[11 + model.inpos] & 33554431) << (29 - 25));
    model.output[13 + model.outpos] =
        (model.input[11 + model.inpos] >>> 25) | ((model.input[12 + model.inpos] & 4194303) << (29 - 22));
    model.output[14 + model.outpos] =
        (model.input[12 + model.inpos] >>> 22) | ((model.input[13 + model.inpos] & 524287) << (29 - 19));
    model.output[15 + model.outpos] =
        (model.input[13 + model.inpos] >>> 19) | ((model.input[14 + model.inpos] & 65535) << (29 - 16));
    model.output[16 + model.outpos] =
        (model.input[14 + model.inpos] >>> 16) | ((model.input[15 + model.inpos] & 8191) << (29 - 13));
    model.output[17 + model.outpos] =
        (model.input[15 + model.inpos] >>> 13) | ((model.input[16 + model.inpos] & 1023) << (29 - 10));
    model.output[18 + model.outpos] =
        (model.input[16 + model.inpos] >>> 10) | ((model.input[17 + model.inpos] & 127) << (29 - 7));
    model.output[19 + model.outpos] =
        (model.input[17 + model.inpos] >>> 7) | ((model.input[18 + model.inpos] & 15) << (29 - 4));
    model.output[20 + model.outpos] =
        (model.input[18 + model.inpos] >>> 4) | ((model.input[19 + model.inpos] & 1) << (29 - 1));
    model.output[21 + model.outpos] = (model.input[19 + model.inpos] >>> 1) & 536870911;
    model.output[22 + model.outpos] =
        (model.input[19 + model.inpos] >>> 30) | ((model.input[20 + model.inpos] & 134217727) << (29 - 27));
    model.output[23 + model.outpos] =
        (model.input[20 + model.inpos] >>> 27) | ((model.input[21 + model.inpos] & 16777215) << (29 - 24));
    model.output[24 + model.outpos] =
        (model.input[21 + model.inpos] >>> 24) | ((model.input[22 + model.inpos] & 2097151) << (29 - 21));
    model.output[25 + model.outpos] =
        (model.input[22 + model.inpos] >>> 21) | ((model.input[23 + model.inpos] & 262143) << (29 - 18));
    model.output[26 + model.outpos] =
        (model.input[23 + model.inpos] >>> 18) | ((model.input[24 + model.inpos] & 32767) << (29 - 15));
    model.output[27 + model.outpos] =
        (model.input[24 + model.inpos] >>> 15) | ((model.input[25 + model.inpos] & 4095) << (29 - 12));
    model.output[28 + model.outpos] =
        (model.input[25 + model.inpos] >>> 12) | ((model.input[26 + model.inpos] & 511) << (29 - 9));
    model.output[29 + model.outpos] =
        (model.input[26 + model.inpos] >>> 9) | ((model.input[27 + model.inpos] & 63) << (29 - 6));
    model.output[30 + model.outpos] =
        (model.input[27 + model.inpos] >>> 6) | ((model.input[28 + model.inpos] & 7) << (29 - 3));
    model.output[31 + model.outpos] = model.input[28 + model.inpos] >>> 3;
}

function fastunpack3(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 7;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 3) & 7;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 6) & 7;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 9) & 7;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 12) & 7;
    model.output[5 + model.outpos] = (model.input[model.inpos] >>> 15) & 7;
    model.output[6 + model.outpos] = (model.input[model.inpos] >>> 18) & 7;
    model.output[7 + model.outpos] = (model.input[model.inpos] >>> 21) & 7;
    model.output[8 + model.outpos] = (model.input[model.inpos] >>> 24) & 7;
    model.output[9 + model.outpos] = (model.input[model.inpos] >>> 27) & 7;
    model.output[10 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 1) << (3 - 1));
    model.output[11 + model.outpos] = (model.input[1 + model.inpos] >>> 1) & 7;
    model.output[12 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 7;
    model.output[13 + model.outpos] = (model.input[1 + model.inpos] >>> 7) & 7;
    model.output[14 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 7;
    model.output[15 + model.outpos] = (model.input[1 + model.inpos] >>> 13) & 7;
    model.output[16 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 7;
    model.output[17 + model.outpos] = (model.input[1 + model.inpos] >>> 19) & 7;
    model.output[18 + model.outpos] = (model.input[1 + model.inpos] >>> 22) & 7;
    model.output[19 + model.outpos] = (model.input[1 + model.inpos] >>> 25) & 7;
    model.output[20 + model.outpos] = (model.input[1 + model.inpos] >>> 28) & 7;
    model.output[21 + model.outpos] =
        (model.input[1 + model.inpos] >>> 31) | ((model.input[2 + model.inpos] & 3) << (3 - 2));
    model.output[22 + model.outpos] = (model.input[2 + model.inpos] >>> 2) & 7;
    model.output[23 + model.outpos] = (model.input[2 + model.inpos] >>> 5) & 7;
    model.output[24 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 7;
    model.output[25 + model.outpos] = (model.input[2 + model.inpos] >>> 11) & 7;
    model.output[26 + model.outpos] = (model.input[2 + model.inpos] >>> 14) & 7;
    model.output[27 + model.outpos] = (model.input[2 + model.inpos] >>> 17) & 7;
    model.output[28 + model.outpos] = (model.input[2 + model.inpos] >>> 20) & 7;
    model.output[29 + model.outpos] = (model.input[2 + model.inpos] >>> 23) & 7;
    model.output[30 + model.outpos] = (model.input[2 + model.inpos] >>> 26) & 7;
    model.output[31 + model.outpos] = model.input[2 + model.inpos] >>> 29;
}

function fastunpack30(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 1073741823;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 268435455) << (30 - 28));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 67108863) << (30 - 26));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 26) | ((model.input[3 + model.inpos] & 16777215) << (30 - 24));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 24) | ((model.input[4 + model.inpos] & 4194303) << (30 - 22));
    model.output[5 + model.outpos] =
        (model.input[4 + model.inpos] >>> 22) | ((model.input[5 + model.inpos] & 1048575) << (30 - 20));
    model.output[6 + model.outpos] =
        (model.input[5 + model.inpos] >>> 20) | ((model.input[6 + model.inpos] & 262143) << (30 - 18));
    model.output[7 + model.outpos] =
        (model.input[6 + model.inpos] >>> 18) | ((model.input[7 + model.inpos] & 65535) << (30 - 16));
    model.output[8 + model.outpos] =
        (model.input[7 + model.inpos] >>> 16) | ((model.input[8 + model.inpos] & 16383) << (30 - 14));
    model.output[9 + model.outpos] =
        (model.input[8 + model.inpos] >>> 14) | ((model.input[9 + model.inpos] & 4095) << (30 - 12));
    model.output[10 + model.outpos] =
        (model.input[9 + model.inpos] >>> 12) | ((model.input[10 + model.inpos] & 1023) << (30 - 10));
    model.output[11 + model.outpos] =
        (model.input[10 + model.inpos] >>> 10) | ((model.input[11 + model.inpos] & 255) << (30 - 8));
    model.output[12 + model.outpos] =
        (model.input[11 + model.inpos] >>> 8) | ((model.input[12 + model.inpos] & 63) << (30 - 6));
    model.output[13 + model.outpos] =
        (model.input[12 + model.inpos] >>> 6) | ((model.input[13 + model.inpos] & 15) << (30 - 4));
    model.output[14 + model.outpos] =
        (model.input[13 + model.inpos] >>> 4) | ((model.input[14 + model.inpos] & 3) << (30 - 2));
    model.output[15 + model.outpos] = model.input[14 + model.inpos] >>> 2;
    model.output[16 + model.outpos] = (model.input[15 + model.inpos] >>> 0) & 1073741823;
    model.output[17 + model.outpos] =
        (model.input[15 + model.inpos] >>> 30) | ((model.input[16 + model.inpos] & 268435455) << (30 - 28));
    model.output[18 + model.outpos] =
        (model.input[16 + model.inpos] >>> 28) | ((model.input[17 + model.inpos] & 67108863) << (30 - 26));
    model.output[19 + model.outpos] =
        (model.input[17 + model.inpos] >>> 26) | ((model.input[18 + model.inpos] & 16777215) << (30 - 24));
    model.output[20 + model.outpos] =
        (model.input[18 + model.inpos] >>> 24) | ((model.input[19 + model.inpos] & 4194303) << (30 - 22));
    model.output[21 + model.outpos] =
        (model.input[19 + model.inpos] >>> 22) | ((model.input[20 + model.inpos] & 1048575) << (30 - 20));
    model.output[22 + model.outpos] =
        (model.input[20 + model.inpos] >>> 20) | ((model.input[21 + model.inpos] & 262143) << (30 - 18));
    model.output[23 + model.outpos] =
        (model.input[21 + model.inpos] >>> 18) | ((model.input[22 + model.inpos] & 65535) << (30 - 16));
    model.output[24 + model.outpos] =
        (model.input[22 + model.inpos] >>> 16) | ((model.input[23 + model.inpos] & 16383) << (30 - 14));
    model.output[25 + model.outpos] =
        (model.input[23 + model.inpos] >>> 14) | ((model.input[24 + model.inpos] & 4095) << (30 - 12));
    model.output[26 + model.outpos] =
        (model.input[24 + model.inpos] >>> 12) | ((model.input[25 + model.inpos] & 1023) << (30 - 10));
    model.output[27 + model.outpos] =
        (model.input[25 + model.inpos] >>> 10) | ((model.input[26 + model.inpos] & 255) << (30 - 8));
    model.output[28 + model.outpos] =
        (model.input[26 + model.inpos] >>> 8) | ((model.input[27 + model.inpos] & 63) << (30 - 6));
    model.output[29 + model.outpos] =
        (model.input[27 + model.inpos] >>> 6) | ((model.input[28 + model.inpos] & 15) << (30 - 4));
    model.output[30 + model.outpos] =
        (model.input[28 + model.inpos] >>> 4) | ((model.input[29 + model.inpos] & 3) << (30 - 2));
    model.output[31 + model.outpos] = model.input[29 + model.inpos] >>> 2;
}

function fastunpack31(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 2147483647;
    model.output[1 + model.outpos] =
        (model.input[model.inpos] >>> 31) | ((model.input[1 + model.inpos] & 1073741823) << (31 - 30));
    model.output[2 + model.outpos] =
        (model.input[1 + model.inpos] >>> 30) | ((model.input[2 + model.inpos] & 536870911) << (31 - 29));
    model.output[3 + model.outpos] =
        (model.input[2 + model.inpos] >>> 29) | ((model.input[3 + model.inpos] & 268435455) << (31 - 28));
    model.output[4 + model.outpos] =
        (model.input[3 + model.inpos] >>> 28) | ((model.input[4 + model.inpos] & 134217727) << (31 - 27));
    model.output[5 + model.outpos] =
        (model.input[4 + model.inpos] >>> 27) | ((model.input[5 + model.inpos] & 67108863) << (31 - 26));
    model.output[6 + model.outpos] =
        (model.input[5 + model.inpos] >>> 26) | ((model.input[6 + model.inpos] & 33554431) << (31 - 25));
    model.output[7 + model.outpos] =
        (model.input[6 + model.inpos] >>> 25) | ((model.input[7 + model.inpos] & 16777215) << (31 - 24));
    model.output[8 + model.outpos] =
        (model.input[7 + model.inpos] >>> 24) | ((model.input[8 + model.inpos] & 8388607) << (31 - 23));
    model.output[9 + model.outpos] =
        (model.input[8 + model.inpos] >>> 23) | ((model.input[9 + model.inpos] & 4194303) << (31 - 22));
    model.output[10 + model.outpos] =
        (model.input[9 + model.inpos] >>> 22) | ((model.input[10 + model.inpos] & 2097151) << (31 - 21));
    model.output[11 + model.outpos] =
        (model.input[10 + model.inpos] >>> 21) | ((model.input[11 + model.inpos] & 1048575) << (31 - 20));
    model.output[12 + model.outpos] =
        (model.input[11 + model.inpos] >>> 20) | ((model.input[12 + model.inpos] & 524287) << (31 - 19));
    model.output[13 + model.outpos] =
        (model.input[12 + model.inpos] >>> 19) | ((model.input[13 + model.inpos] & 262143) << (31 - 18));
    model.output[14 + model.outpos] =
        (model.input[13 + model.inpos] >>> 18) | ((model.input[14 + model.inpos] & 131071) << (31 - 17));
    model.output[15 + model.outpos] =
        (model.input[14 + model.inpos] >>> 17) | ((model.input[15 + model.inpos] & 65535) << (31 - 16));
    model.output[16 + model.outpos] =
        (model.input[15 + model.inpos] >>> 16) | ((model.input[16 + model.inpos] & 32767) << (31 - 15));
    model.output[17 + model.outpos] =
        (model.input[16 + model.inpos] >>> 15) | ((model.input[17 + model.inpos] & 16383) << (31 - 14));
    model.output[18 + model.outpos] =
        (model.input[17 + model.inpos] >>> 14) | ((model.input[18 + model.inpos] & 8191) << (31 - 13));
    model.output[19 + model.outpos] =
        (model.input[18 + model.inpos] >>> 13) | ((model.input[19 + model.inpos] & 4095) << (31 - 12));
    model.output[20 + model.outpos] =
        (model.input[19 + model.inpos] >>> 12) | ((model.input[20 + model.inpos] & 2047) << (31 - 11));
    model.output[21 + model.outpos] =
        (model.input[20 + model.inpos] >>> 11) | ((model.input[21 + model.inpos] & 1023) << (31 - 10));
    model.output[22 + model.outpos] =
        (model.input[21 + model.inpos] >>> 10) | ((model.input[22 + model.inpos] & 511) << (31 - 9));
    model.output[23 + model.outpos] =
        (model.input[22 + model.inpos] >>> 9) | ((model.input[23 + model.inpos] & 255) << (31 - 8));
    model.output[24 + model.outpos] =
        (model.input[23 + model.inpos] >>> 8) | ((model.input[24 + model.inpos] & 127) << (31 - 7));
    model.output[25 + model.outpos] =
        (model.input[24 + model.inpos] >>> 7) | ((model.input[25 + model.inpos] & 63) << (31 - 6));
    model.output[26 + model.outpos] =
        (model.input[25 + model.inpos] >>> 6) | ((model.input[26 + model.inpos] & 31) << (31 - 5));
    model.output[27 + model.outpos] =
        (model.input[26 + model.inpos] >>> 5) | ((model.input[27 + model.inpos] & 15) << (31 - 4));
    model.output[28 + model.outpos] =
        (model.input[27 + model.inpos] >>> 4) | ((model.input[28 + model.inpos] & 7) << (31 - 3));
    model.output[29 + model.outpos] =
        (model.input[28 + model.inpos] >>> 3) | ((model.input[29 + model.inpos] & 3) << (31 - 2));
    model.output[30 + model.outpos] =
        (model.input[29 + model.inpos] >>> 2) | ((model.input[30 + model.inpos] & 1) << (31 - 1));
    model.output[31 + model.outpos] = model.input[30 + model.inpos] >>> 1;
}

function fastunpack32(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    for (let i = 0; i < 32; i++) model.output[model.outpos + i] = model.input[model.inpos + i];
}

function fastunpack4(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 15;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 4) & 15;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 8) & 15;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 12) & 15;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 16) & 15;
    model.output[5 + model.outpos] = (model.input[model.inpos] >>> 20) & 15;
    model.output[6 + model.outpos] = (model.input[model.inpos] >>> 24) & 15;
    model.output[7 + model.outpos] = model.input[model.inpos] >>> 28;
    model.output[8 + model.outpos] = (model.input[1 + model.inpos] >>> 0) & 15;
    model.output[9 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 15;
    model.output[10 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 15;
    model.output[11 + model.outpos] = (model.input[1 + model.inpos] >>> 12) & 15;
    model.output[12 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 15;
    model.output[13 + model.outpos] = (model.input[1 + model.inpos] >>> 20) & 15;
    model.output[14 + model.outpos] = (model.input[1 + model.inpos] >>> 24) & 15;
    model.output[15 + model.outpos] = model.input[1 + model.inpos] >>> 28;
    model.output[16 + model.outpos] = (model.input[2 + model.inpos] >>> 0) & 15;
    model.output[17 + model.outpos] = (model.input[2 + model.inpos] >>> 4) & 15;
    model.output[18 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 15;
    model.output[19 + model.outpos] = (model.input[2 + model.inpos] >>> 12) & 15;
    model.output[20 + model.outpos] = (model.input[2 + model.inpos] >>> 16) & 15;
    model.output[21 + model.outpos] = (model.input[2 + model.inpos] >>> 20) & 15;
    model.output[22 + model.outpos] = (model.input[2 + model.inpos] >>> 24) & 15;
    model.output[23 + model.outpos] = model.input[2 + model.inpos] >>> 28;
    model.output[24 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 15;
    model.output[25 + model.outpos] = (model.input[3 + model.inpos] >>> 4) & 15;
    model.output[26 + model.outpos] = (model.input[3 + model.inpos] >>> 8) & 15;
    model.output[27 + model.outpos] = (model.input[3 + model.inpos] >>> 12) & 15;
    model.output[28 + model.outpos] = (model.input[3 + model.inpos] >>> 16) & 15;
    model.output[29 + model.outpos] = (model.input[3 + model.inpos] >>> 20) & 15;
    model.output[30 + model.outpos] = (model.input[3 + model.inpos] >>> 24) & 15;
    model.output[31 + model.outpos] = model.input[3 + model.inpos] >>> 28;
}

function fastunpack5(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 31;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 5) & 31;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 10) & 31;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 15) & 31;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 20) & 31;
    model.output[5 + model.outpos] = (model.input[model.inpos] >>> 25) & 31;
    model.output[6 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 7) << (5 - 3));
    model.output[7 + model.outpos] = (model.input[1 + model.inpos] >>> 3) & 31;
    model.output[8 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 31;
    model.output[9 + model.outpos] = (model.input[1 + model.inpos] >>> 13) & 31;
    model.output[10 + model.outpos] = (model.input[1 + model.inpos] >>> 18) & 31;
    model.output[11 + model.outpos] = (model.input[1 + model.inpos] >>> 23) & 31;
    model.output[12 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 1) << (5 - 1));
    model.output[13 + model.outpos] = (model.input[2 + model.inpos] >>> 1) & 31;
    model.output[14 + model.outpos] = (model.input[2 + model.inpos] >>> 6) & 31;
    model.output[15 + model.outpos] = (model.input[2 + model.inpos] >>> 11) & 31;
    model.output[16 + model.outpos] = (model.input[2 + model.inpos] >>> 16) & 31;
    model.output[17 + model.outpos] = (model.input[2 + model.inpos] >>> 21) & 31;
    model.output[18 + model.outpos] = (model.input[2 + model.inpos] >>> 26) & 31;
    model.output[19 + model.outpos] =
        (model.input[2 + model.inpos] >>> 31) | ((model.input[3 + model.inpos] & 15) << (5 - 4));
    model.output[20 + model.outpos] = (model.input[3 + model.inpos] >>> 4) & 31;
    model.output[21 + model.outpos] = (model.input[3 + model.inpos] >>> 9) & 31;
    model.output[22 + model.outpos] = (model.input[3 + model.inpos] >>> 14) & 31;
    model.output[23 + model.outpos] = (model.input[3 + model.inpos] >>> 19) & 31;
    model.output[24 + model.outpos] = (model.input[3 + model.inpos] >>> 24) & 31;
    model.output[25 + model.outpos] =
        (model.input[3 + model.inpos] >>> 29) | ((model.input[4 + model.inpos] & 3) << (5 - 2));
    model.output[26 + model.outpos] = (model.input[4 + model.inpos] >>> 2) & 31;
    model.output[27 + model.outpos] = (model.input[4 + model.inpos] >>> 7) & 31;
    model.output[28 + model.outpos] = (model.input[4 + model.inpos] >>> 12) & 31;
    model.output[29 + model.outpos] = (model.input[4 + model.inpos] >>> 17) & 31;
    model.output[30 + model.outpos] = (model.input[4 + model.inpos] >>> 22) & 31;
    model.output[31 + model.outpos] = model.input[4 + model.inpos] >>> 27;
}

function fastunpack6(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 63;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 6) & 63;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 12) & 63;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 18) & 63;
    model.output[4 + model.outpos] = (model.input[model.inpos] >>> 24) & 63;
    model.output[5 + model.outpos] =
        (model.input[model.inpos] >>> 30) | ((model.input[1 + model.inpos] & 15) << (6 - 4));
    model.output[6 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 63;
    model.output[7 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 63;
    model.output[8 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 63;
    model.output[9 + model.outpos] = (model.input[1 + model.inpos] >>> 22) & 63;
    model.output[10 + model.outpos] =
        (model.input[1 + model.inpos] >>> 28) | ((model.input[2 + model.inpos] & 3) << (6 - 2));
    model.output[11 + model.outpos] = (model.input[2 + model.inpos] >>> 2) & 63;
    model.output[12 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 63;
    model.output[13 + model.outpos] = (model.input[2 + model.inpos] >>> 14) & 63;
    model.output[14 + model.outpos] = (model.input[2 + model.inpos] >>> 20) & 63;
    model.output[15 + model.outpos] = model.input[2 + model.inpos] >>> 26;
    model.output[16 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 63;
    model.output[17 + model.outpos] = (model.input[3 + model.inpos] >>> 6) & 63;
    model.output[18 + model.outpos] = (model.input[3 + model.inpos] >>> 12) & 63;
    model.output[19 + model.outpos] = (model.input[3 + model.inpos] >>> 18) & 63;
    model.output[20 + model.outpos] = (model.input[3 + model.inpos] >>> 24) & 63;
    model.output[21 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 15) << (6 - 4));
    model.output[22 + model.outpos] = (model.input[4 + model.inpos] >>> 4) & 63;
    model.output[23 + model.outpos] = (model.input[4 + model.inpos] >>> 10) & 63;
    model.output[24 + model.outpos] = (model.input[4 + model.inpos] >>> 16) & 63;
    model.output[25 + model.outpos] = (model.input[4 + model.inpos] >>> 22) & 63;
    model.output[26 + model.outpos] =
        (model.input[4 + model.inpos] >>> 28) | ((model.input[5 + model.inpos] & 3) << (6 - 2));
    model.output[27 + model.outpos] = (model.input[5 + model.inpos] >>> 2) & 63;
    model.output[28 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 63;
    model.output[29 + model.outpos] = (model.input[5 + model.inpos] >>> 14) & 63;
    model.output[30 + model.outpos] = (model.input[5 + model.inpos] >>> 20) & 63;
    model.output[31 + model.outpos] = model.input[5 + model.inpos] >>> 26;
}

function fastunpack7(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 127;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 7) & 127;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 14) & 127;
    model.output[3 + model.outpos] = (model.input[model.inpos] >>> 21) & 127;
    model.output[4 + model.outpos] =
        (model.input[model.inpos] >>> 28) | ((model.input[1 + model.inpos] & 7) << (7 - 3));
    model.output[5 + model.outpos] = (model.input[1 + model.inpos] >>> 3) & 127;
    model.output[6 + model.outpos] = (model.input[1 + model.inpos] >>> 10) & 127;
    model.output[7 + model.outpos] = (model.input[1 + model.inpos] >>> 17) & 127;
    model.output[8 + model.outpos] = (model.input[1 + model.inpos] >>> 24) & 127;
    model.output[9 + model.outpos] =
        (model.input[1 + model.inpos] >>> 31) | ((model.input[2 + model.inpos] & 63) << (7 - 6));
    model.output[10 + model.outpos] = (model.input[2 + model.inpos] >>> 6) & 127;
    model.output[11 + model.outpos] = (model.input[2 + model.inpos] >>> 13) & 127;
    model.output[12 + model.outpos] = (model.input[2 + model.inpos] >>> 20) & 127;
    model.output[13 + model.outpos] =
        (model.input[2 + model.inpos] >>> 27) | ((model.input[3 + model.inpos] & 3) << (7 - 2));
    model.output[14 + model.outpos] = (model.input[3 + model.inpos] >>> 2) & 127;
    model.output[15 + model.outpos] = (model.input[3 + model.inpos] >>> 9) & 127;
    model.output[16 + model.outpos] = (model.input[3 + model.inpos] >>> 16) & 127;
    model.output[17 + model.outpos] = (model.input[3 + model.inpos] >>> 23) & 127;
    model.output[18 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 31) << (7 - 5));
    model.output[19 + model.outpos] = (model.input[4 + model.inpos] >>> 5) & 127;
    model.output[20 + model.outpos] = (model.input[4 + model.inpos] >>> 12) & 127;
    model.output[21 + model.outpos] = (model.input[4 + model.inpos] >>> 19) & 127;
    model.output[22 + model.outpos] =
        (model.input[4 + model.inpos] >>> 26) | ((model.input[5 + model.inpos] & 1) << (7 - 1));
    model.output[23 + model.outpos] = (model.input[5 + model.inpos] >>> 1) & 127;
    model.output[24 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 127;
    model.output[25 + model.outpos] = (model.input[5 + model.inpos] >>> 15) & 127;
    model.output[26 + model.outpos] = (model.input[5 + model.inpos] >>> 22) & 127;
    model.output[27 + model.outpos] =
        (model.input[5 + model.inpos] >>> 29) | ((model.input[6 + model.inpos] & 15) << (7 - 4));
    model.output[28 + model.outpos] = (model.input[6 + model.inpos] >>> 4) & 127;
    model.output[29 + model.outpos] = (model.input[6 + model.inpos] >>> 11) & 127;
    model.output[30 + model.outpos] = (model.input[6 + model.inpos] >>> 18) & 127;
    model.output[31 + model.outpos] = model.input[6 + model.inpos] >>> 25;
}

function fastunpack8(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 255;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 8) & 255;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 16) & 255;
    model.output[3 + model.outpos] = model.input[model.inpos] >>> 24;
    model.output[4 + model.outpos] = (model.input[1 + model.inpos] >>> 0) & 255;
    model.output[5 + model.outpos] = (model.input[1 + model.inpos] >>> 8) & 255;
    model.output[6 + model.outpos] = (model.input[1 + model.inpos] >>> 16) & 255;
    model.output[7 + model.outpos] = model.input[1 + model.inpos] >>> 24;
    model.output[8 + model.outpos] = (model.input[2 + model.inpos] >>> 0) & 255;
    model.output[9 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 255;
    model.output[10 + model.outpos] = (model.input[2 + model.inpos] >>> 16) & 255;
    model.output[11 + model.outpos] = model.input[2 + model.inpos] >>> 24;
    model.output[12 + model.outpos] = (model.input[3 + model.inpos] >>> 0) & 255;
    model.output[13 + model.outpos] = (model.input[3 + model.inpos] >>> 8) & 255;
    model.output[14 + model.outpos] = (model.input[3 + model.inpos] >>> 16) & 255;
    model.output[15 + model.outpos] = model.input[3 + model.inpos] >>> 24;
    model.output[16 + model.outpos] = (model.input[4 + model.inpos] >>> 0) & 255;
    model.output[17 + model.outpos] = (model.input[4 + model.inpos] >>> 8) & 255;
    model.output[18 + model.outpos] = (model.input[4 + model.inpos] >>> 16) & 255;
    model.output[19 + model.outpos] = model.input[4 + model.inpos] >>> 24;
    model.output[20 + model.outpos] = (model.input[5 + model.inpos] >>> 0) & 255;
    model.output[21 + model.outpos] = (model.input[5 + model.inpos] >>> 8) & 255;
    model.output[22 + model.outpos] = (model.input[5 + model.inpos] >>> 16) & 255;
    model.output[23 + model.outpos] = model.input[5 + model.inpos] >>> 24;
    model.output[24 + model.outpos] = (model.input[6 + model.inpos] >>> 0) & 255;
    model.output[25 + model.outpos] = (model.input[6 + model.inpos] >>> 8) & 255;
    model.output[26 + model.outpos] = (model.input[6 + model.inpos] >>> 16) & 255;
    model.output[27 + model.outpos] = model.input[6 + model.inpos] >>> 24;
    model.output[28 + model.outpos] = (model.input[7 + model.inpos] >>> 0) & 255;
    model.output[29 + model.outpos] = (model.input[7 + model.inpos] >>> 8) & 255;
    model.output[30 + model.outpos] = (model.input[7 + model.inpos] >>> 16) & 255;
    model.output[31 + model.outpos] = model.input[7 + model.inpos] >>> 24;
}

function fastunpack9(model: { input: Uint32Array; inpos: number; output: Uint32Array; outpos: number }) {
    model.output[model.outpos] = (model.input[model.inpos] >>> 0) & 511;
    model.output[1 + model.outpos] = (model.input[model.inpos] >>> 9) & 511;
    model.output[2 + model.outpos] = (model.input[model.inpos] >>> 18) & 511;
    model.output[3 + model.outpos] =
        (model.input[model.inpos] >>> 27) | ((model.input[1 + model.inpos] & 15) << (9 - 4));
    model.output[4 + model.outpos] = (model.input[1 + model.inpos] >>> 4) & 511;
    model.output[5 + model.outpos] = (model.input[1 + model.inpos] >>> 13) & 511;
    model.output[6 + model.outpos] = (model.input[1 + model.inpos] >>> 22) & 511;
    model.output[7 + model.outpos] =
        (model.input[1 + model.inpos] >>> 31) | ((model.input[2 + model.inpos] & 255) << (9 - 8));
    model.output[8 + model.outpos] = (model.input[2 + model.inpos] >>> 8) & 511;
    model.output[9 + model.outpos] = (model.input[2 + model.inpos] >>> 17) & 511;
    model.output[10 + model.outpos] =
        (model.input[2 + model.inpos] >>> 26) | ((model.input[3 + model.inpos] & 7) << (9 - 3));
    model.output[11 + model.outpos] = (model.input[3 + model.inpos] >>> 3) & 511;
    model.output[12 + model.outpos] = (model.input[3 + model.inpos] >>> 12) & 511;
    model.output[13 + model.outpos] = (model.input[3 + model.inpos] >>> 21) & 511;
    model.output[14 + model.outpos] =
        (model.input[3 + model.inpos] >>> 30) | ((model.input[4 + model.inpos] & 127) << (9 - 7));
    model.output[15 + model.outpos] = (model.input[4 + model.inpos] >>> 7) & 511;
    model.output[16 + model.outpos] = (model.input[4 + model.inpos] >>> 16) & 511;
    model.output[17 + model.outpos] =
        (model.input[4 + model.inpos] >>> 25) | ((model.input[5 + model.inpos] & 3) << (9 - 2));
    model.output[18 + model.outpos] = (model.input[5 + model.inpos] >>> 2) & 511;
    model.output[19 + model.outpos] = (model.input[5 + model.inpos] >>> 11) & 511;
    model.output[20 + model.outpos] = (model.input[5 + model.inpos] >>> 20) & 511;
    model.output[21 + model.outpos] =
        (model.input[5 + model.inpos] >>> 29) | ((model.input[6 + model.inpos] & 63) << (9 - 6));
    model.output[22 + model.outpos] = (model.input[6 + model.inpos] >>> 6) & 511;
    model.output[23 + model.outpos] = (model.input[6 + model.inpos] >>> 15) & 511;
    model.output[24 + model.outpos] =
        (model.input[6 + model.inpos] >>> 24) | ((model.input[7 + model.inpos] & 1) << (9 - 1));
    model.output[25 + model.outpos] = (model.input[7 + model.inpos] >>> 1) & 511;
    model.output[26 + model.outpos] = (model.input[7 + model.inpos] >>> 10) & 511;
    model.output[27 + model.outpos] = (model.input[7 + model.inpos] >>> 19) & 511;
    model.output[28 + model.outpos] =
        (model.input[7 + model.inpos] >>> 28) | ((model.input[8 + model.inpos] & 31) << (9 - 5));
    model.output[29 + model.outpos] = (model.input[8 + model.inpos] >>> 5) & 511;
    model.output[30 + model.outpos] = (model.input[8 + model.inpos] >>> 14) & 511;
    model.output[31 + model.outpos] = model.input[8 + model.inpos] >>> 23;
}
