import { arraycopy } from './util';

/**
   * Pack 32 numberegers
   *
   * @param inArray
   *                source array
   * @param inpos
   *                position in source array
   * @param outArray
   *                output array
   * @param outpos
   *                position in output array
   * @param bit
   *                number of bits to use per numbereger
   */
export function fastpack(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number, bit: number) {
  switch (bit) {
  case 0:
    fastpack0(inArray, inpos, outArray, outpos);
    break;
  case 1:
    fastpack1(inArray, inpos, outArray, outpos);
    break;
  case 2:
    fastpack2(inArray, inpos, outArray, outpos);
    break;
  case 3:
    fastpack3(inArray, inpos, outArray, outpos);
    break;
  case 4:
    fastpack4(inArray, inpos, outArray, outpos);
    break;
  case 5:
    fastpack5(inArray, inpos, outArray, outpos);
    break;
  case 6:
    fastpack6(inArray, inpos, outArray, outpos);
    break;
  case 7:
    fastpack7(inArray, inpos, outArray, outpos);
    break;
  case 8:
    fastpack8(inArray, inpos, outArray, outpos);
    break;
  case 9:
    fastpack9(inArray, inpos, outArray, outpos);
    break;
  case 10:
    fastpack10(inArray, inpos, outArray, outpos);
    break;
  case 11:
    fastpack11(inArray, inpos, outArray, outpos);
    break;
  case 12:
    fastpack12(inArray, inpos, outArray, outpos);
    break;
  case 13:
    fastpack13(inArray, inpos, outArray, outpos);
    break;
  case 14:
    fastpack14(inArray, inpos, outArray, outpos);
    break;
  case 15:
    fastpack15(inArray, inpos, outArray, outpos);
    break;
  case 16:
    fastpack16(inArray, inpos, outArray, outpos);
    break;
  case 17:
    fastpack17(inArray, inpos, outArray, outpos);
    break;
  case 18:
    fastpack18(inArray, inpos, outArray, outpos);
    break;
  case 19:
    fastpack19(inArray, inpos, outArray, outpos);
    break;
  case 20:
    fastpack20(inArray, inpos, outArray, outpos);
    break;
  case 21:
    fastpack21(inArray, inpos, outArray, outpos);
    break;
  case 22:
    fastpack22(inArray, inpos, outArray, outpos);
    break;
  case 23:
    fastpack23(inArray, inpos, outArray, outpos);
    break;
  case 24:
    fastpack24(inArray, inpos, outArray, outpos);
    break;
  case 25:
    fastpack25(inArray, inpos, outArray, outpos);
    break;
  case 26:
    fastpack26(inArray, inpos, outArray, outpos);
    break;
  case 27:
    fastpack27(inArray, inpos, outArray, outpos);
    break;
  case 28:
    fastpack28(inArray, inpos, outArray, outpos);
    break;
  case 29:
    fastpack29(inArray, inpos, outArray, outpos);
    break;
  case 30:
    fastpack30(inArray, inpos, outArray, outpos);
    break;
  case 31:
    fastpack31(inArray, inpos, outArray, outpos);
    break;
  case 32:
    fastpack32(inArray, inpos, outArray, outpos);
    break;
  default:
    throw new Error("Unsupported bit width.");
  }
}

function fastpack0(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  // nothing
}

function fastpack1(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 1)
| ((inArray[1 + inpos] & 1) << 1)
| ((inArray[2 + inpos] & 1) << 2)
| ((inArray[3 + inpos] & 1) << 3)
| ((inArray[4 + inpos] & 1) << 4)
| ((inArray[5 + inpos] & 1) << 5)
| ((inArray[6 + inpos] & 1) << 6)
| ((inArray[7 + inpos] & 1) << 7)
| ((inArray[8 + inpos] & 1) << 8)
| ((inArray[9 + inpos] & 1) << 9)
| ((inArray[10 + inpos] & 1) << 10)
| ((inArray[11 + inpos] & 1) << 11)
| ((inArray[12 + inpos] & 1) << 12)
| ((inArray[13 + inpos] & 1) << 13)
| ((inArray[14 + inpos] & 1) << 14)
| ((inArray[15 + inpos] & 1) << 15)
| ((inArray[16 + inpos] & 1) << 16)
| ((inArray[17 + inpos] & 1) << 17)
| ((inArray[18 + inpos] & 1) << 18)
| ((inArray[19 + inpos] & 1) << 19)
| ((inArray[20 + inpos] & 1) << 20)
| ((inArray[21 + inpos] & 1) << 21)
| ((inArray[22 + inpos] & 1) << 22)
| ((inArray[23 + inpos] & 1) << 23)
| ((inArray[24 + inpos] & 1) << 24)
| ((inArray[25 + inpos] & 1) << 25)
| ((inArray[26 + inpos] & 1) << 26)
| ((inArray[27 + inpos] & 1) << 27)
| ((inArray[28 + inpos] & 1) << 28)
| ((inArray[29 + inpos] & 1) << 29)
| ((inArray[30 + inpos] & 1) << 30)
| ((inArray[31 + inpos]) << 31);
}

function fastpack10(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 1023)
| ((inArray[1 + inpos] & 1023) << 10)
| ((inArray[2 + inpos] & 1023) << 20)
| ((inArray[3 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[3 + inpos] & 1023) >>> (10 - 8))
| ((inArray[4 + inpos] & 1023) << 8)
| ((inArray[5 + inpos] & 1023) << 18)
| ((inArray[6 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[6 + inpos] & 1023) >>> (10 - 6))
| ((inArray[7 + inpos] & 1023) << 6)
| ((inArray[8 + inpos] & 1023) << 16)
| ((inArray[9 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[9 + inpos] & 1023) >>> (10 - 4))
| ((inArray[10 + inpos] & 1023) << 4)
| ((inArray[11 + inpos] & 1023) << 14)
| ((inArray[12 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[12 + inpos] & 1023) >>> (10 - 2))
| ((inArray[13 + inpos] & 1023) << 2)
| ((inArray[14 + inpos] & 1023) << 12)
| ((inArray[15 + inpos]) << 22);
  outArray[5 + outpos] = (inArray[16 + inpos] & 1023)
| ((inArray[17 + inpos] & 1023) << 10)
| ((inArray[18 + inpos] & 1023) << 20)
| ((inArray[19 + inpos]) << 30);
  outArray[6 + outpos] = ((inArray[19 + inpos] & 1023) >>> (10 - 8))
| ((inArray[20 + inpos] & 1023) << 8)
| ((inArray[21 + inpos] & 1023) << 18)
| ((inArray[22 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[22 + inpos] & 1023) >>> (10 - 6))
| ((inArray[23 + inpos] & 1023) << 6)
| ((inArray[24 + inpos] & 1023) << 16)
| ((inArray[25 + inpos]) << 26);
  outArray[8 + outpos] = ((inArray[25 + inpos] & 1023) >>> (10 - 4))
| ((inArray[26 + inpos] & 1023) << 4)
| ((inArray[27 + inpos] & 1023) << 14)
| ((inArray[28 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[28 + inpos] & 1023) >>> (10 - 2))
| ((inArray[29 + inpos] & 1023) << 2)
| ((inArray[30 + inpos] & 1023) << 12)
| ((inArray[31 + inpos]) << 22);
}

function fastpack11(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 2047)
| ((inArray[1 + inpos] & 2047) << 11)
| ((inArray[2 + inpos]) << 22);
  outArray[1 + outpos] = ((inArray[2 + inpos] & 2047) >>> (11 - 1))
| ((inArray[3 + inpos] & 2047) << 1)
| ((inArray[4 + inpos] & 2047) << 12)
| ((inArray[5 + inpos]) << 23);
  outArray[2 + outpos] = ((inArray[5 + inpos] & 2047) >>> (11 - 2))
| ((inArray[6 + inpos] & 2047) << 2)
| ((inArray[7 + inpos] & 2047) << 13)
| ((inArray[8 + inpos]) << 24);
  outArray[3 + outpos] = ((inArray[8 + inpos] & 2047) >>> (11 - 3))
| ((inArray[9 + inpos] & 2047) << 3)
| ((inArray[10 + inpos] & 2047) << 14)
| ((inArray[11 + inpos]) << 25);
  outArray[4 + outpos] = ((inArray[11 + inpos] & 2047) >>> (11 - 4))
| ((inArray[12 + inpos] & 2047) << 4)
| ((inArray[13 + inpos] & 2047) << 15)
| ((inArray[14 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[14 + inpos] & 2047) >>> (11 - 5))
| ((inArray[15 + inpos] & 2047) << 5)
| ((inArray[16 + inpos] & 2047) << 16)
| ((inArray[17 + inpos]) << 27);
  outArray[6 + outpos] = ((inArray[17 + inpos] & 2047) >>> (11 - 6))
| ((inArray[18 + inpos] & 2047) << 6)
| ((inArray[19 + inpos] & 2047) << 17)
| ((inArray[20 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[20 + inpos] & 2047) >>> (11 - 7))
| ((inArray[21 + inpos] & 2047) << 7)
| ((inArray[22 + inpos] & 2047) << 18)
| ((inArray[23 + inpos]) << 29);
  outArray[8 + outpos] = ((inArray[23 + inpos] & 2047) >>> (11 - 8))
| ((inArray[24 + inpos] & 2047) << 8)
| ((inArray[25 + inpos] & 2047) << 19)
| ((inArray[26 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[26 + inpos] & 2047) >>> (11 - 9))
| ((inArray[27 + inpos] & 2047) << 9)
| ((inArray[28 + inpos] & 2047) << 20)
| ((inArray[29 + inpos]) << 31);
  outArray[10 + outpos] = ((inArray[29 + inpos] & 2047) >>> (11 - 10))
| ((inArray[30 + inpos] & 2047) << 10)
| ((inArray[31 + inpos]) << 21);
}

function fastpack12(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 4095)
| ((inArray[1 + inpos] & 4095) << 12)
| ((inArray[2 + inpos]) << 24);
  outArray[1 + outpos] = ((inArray[2 + inpos] & 4095) >>> (12 - 4))
| ((inArray[3 + inpos] & 4095) << 4)
| ((inArray[4 + inpos] & 4095) << 16)
| ((inArray[5 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[5 + inpos] & 4095) >>> (12 - 8))
| ((inArray[6 + inpos] & 4095) << 8)
| ((inArray[7 + inpos]) << 20);
  outArray[3 + outpos] = (inArray[8 + inpos] & 4095)
| ((inArray[9 + inpos] & 4095) << 12)
| ((inArray[10 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[10 + inpos] & 4095) >>> (12 - 4))
| ((inArray[11 + inpos] & 4095) << 4)
| ((inArray[12 + inpos] & 4095) << 16)
| ((inArray[13 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[13 + inpos] & 4095) >>> (12 - 8))
| ((inArray[14 + inpos] & 4095) << 8)
| ((inArray[15 + inpos]) << 20);
  outArray[6 + outpos] = (inArray[16 + inpos] & 4095)
| ((inArray[17 + inpos] & 4095) << 12)
| ((inArray[18 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[18 + inpos] & 4095) >>> (12 - 4))
| ((inArray[19 + inpos] & 4095) << 4)
| ((inArray[20 + inpos] & 4095) << 16)
| ((inArray[21 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[21 + inpos] & 4095) >>> (12 - 8))
| ((inArray[22 + inpos] & 4095) << 8)
| ((inArray[23 + inpos]) << 20);
  outArray[9 + outpos] = (inArray[24 + inpos] & 4095)
| ((inArray[25 + inpos] & 4095) << 12)
| ((inArray[26 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[26 + inpos] & 4095) >>> (12 - 4))
| ((inArray[27 + inpos] & 4095) << 4)
| ((inArray[28 + inpos] & 4095) << 16)
| ((inArray[29 + inpos]) << 28);
  outArray[11 + outpos] = ((inArray[29 + inpos] & 4095) >>> (12 - 8))
| ((inArray[30 + inpos] & 4095) << 8)
| ((inArray[31 + inpos]) << 20);
}

function fastpack13(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 8191)
| ((inArray[1 + inpos] & 8191) << 13)
| ((inArray[2 + inpos]) << 26);
  outArray[1 + outpos] = ((inArray[2 + inpos] & 8191) >>> (13 - 7))
| ((inArray[3 + inpos] & 8191) << 7)
| ((inArray[4 + inpos]) << 20);
  outArray[2 + outpos] = ((inArray[4 + inpos] & 8191) >>> (13 - 1))
| ((inArray[5 + inpos] & 8191) << 1)
| ((inArray[6 + inpos] & 8191) << 14)
| ((inArray[7 + inpos]) << 27);
  outArray[3 + outpos] = ((inArray[7 + inpos] & 8191) >>> (13 - 8))
| ((inArray[8 + inpos] & 8191) << 8)
| ((inArray[9 + inpos]) << 21);
  outArray[4 + outpos] = ((inArray[9 + inpos] & 8191) >>> (13 - 2))
| ((inArray[10 + inpos] & 8191) << 2)
| ((inArray[11 + inpos] & 8191) << 15)
| ((inArray[12 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[12 + inpos] & 8191) >>> (13 - 9))
| ((inArray[13 + inpos] & 8191) << 9)
| ((inArray[14 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[14 + inpos] & 8191) >>> (13 - 3))
| ((inArray[15 + inpos] & 8191) << 3)
| ((inArray[16 + inpos] & 8191) << 16)
| ((inArray[17 + inpos]) << 29);
  outArray[7 + outpos] = ((inArray[17 + inpos] & 8191) >>> (13 - 10))
| ((inArray[18 + inpos] & 8191) << 10)
| ((inArray[19 + inpos]) << 23);
  outArray[8 + outpos] = ((inArray[19 + inpos] & 8191) >>> (13 - 4))
| ((inArray[20 + inpos] & 8191) << 4)
| ((inArray[21 + inpos] & 8191) << 17)
| ((inArray[22 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[22 + inpos] & 8191) >>> (13 - 11))
| ((inArray[23 + inpos] & 8191) << 11)
| ((inArray[24 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[24 + inpos] & 8191) >>> (13 - 5))
| ((inArray[25 + inpos] & 8191) << 5)
| ((inArray[26 + inpos] & 8191) << 18)
| ((inArray[27 + inpos]) << 31);
  outArray[11 + outpos] = ((inArray[27 + inpos] & 8191) >>> (13 - 12))
| ((inArray[28 + inpos] & 8191) << 12)
| ((inArray[29 + inpos]) << 25);
  outArray[12 + outpos] = ((inArray[29 + inpos] & 8191) >>> (13 - 6))
| ((inArray[30 + inpos] & 8191) << 6)
| ((inArray[31 + inpos]) << 19);
}

function fastpack14(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 16383)
| ((inArray[1 + inpos] & 16383) << 14)
| ((inArray[2 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[2 + inpos] & 16383) >>> (14 - 10))
| ((inArray[3 + inpos] & 16383) << 10)
| ((inArray[4 + inpos]) << 24);
  outArray[2 + outpos] = ((inArray[4 + inpos] & 16383) >>> (14 - 6))
| ((inArray[5 + inpos] & 16383) << 6)
| ((inArray[6 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[6 + inpos] & 16383) >>> (14 - 2))
| ((inArray[7 + inpos] & 16383) << 2)
| ((inArray[8 + inpos] & 16383) << 16)
| ((inArray[9 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[9 + inpos] & 16383) >>> (14 - 12))
| ((inArray[10 + inpos] & 16383) << 12)
| ((inArray[11 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[11 + inpos] & 16383) >>> (14 - 8))
| ((inArray[12 + inpos] & 16383) << 8)
| ((inArray[13 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[13 + inpos] & 16383) >>> (14 - 4))
| ((inArray[14 + inpos] & 16383) << 4)
| ((inArray[15 + inpos]) << 18);
  outArray[7 + outpos] = (inArray[16 + inpos] & 16383)
| ((inArray[17 + inpos] & 16383) << 14)
| ((inArray[18 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[18 + inpos] & 16383) >>> (14 - 10))
| ((inArray[19 + inpos] & 16383) << 10)
| ((inArray[20 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[20 + inpos] & 16383) >>> (14 - 6))
| ((inArray[21 + inpos] & 16383) << 6)
| ((inArray[22 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[22 + inpos] & 16383) >>> (14 - 2))
| ((inArray[23 + inpos] & 16383) << 2)
| ((inArray[24 + inpos] & 16383) << 16)
| ((inArray[25 + inpos]) << 30);
  outArray[11 + outpos] = ((inArray[25 + inpos] & 16383) >>> (14 - 12))
| ((inArray[26 + inpos] & 16383) << 12)
| ((inArray[27 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[27 + inpos] & 16383) >>> (14 - 8))
| ((inArray[28 + inpos] & 16383) << 8)
| ((inArray[29 + inpos]) << 22);
  outArray[13 + outpos] = ((inArray[29 + inpos] & 16383) >>> (14 - 4))
| ((inArray[30 + inpos] & 16383) << 4)
| ((inArray[31 + inpos]) << 18);
}

function fastpack15(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 32767)
| ((inArray[1 + inpos] & 32767) << 15)
| ((inArray[2 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[2 + inpos] & 32767) >>> (15 - 13))
| ((inArray[3 + inpos] & 32767) << 13)
| ((inArray[4 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[4 + inpos] & 32767) >>> (15 - 11))
| ((inArray[5 + inpos] & 32767) << 11)
| ((inArray[6 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[6 + inpos] & 32767) >>> (15 - 9))
| ((inArray[7 + inpos] & 32767) << 9)
| ((inArray[8 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[8 + inpos] & 32767) >>> (15 - 7))
| ((inArray[9 + inpos] & 32767) << 7)
| ((inArray[10 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[10 + inpos] & 32767) >>> (15 - 5))
| ((inArray[11 + inpos] & 32767) << 5)
| ((inArray[12 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[12 + inpos] & 32767) >>> (15 - 3))
| ((inArray[13 + inpos] & 32767) << 3)
| ((inArray[14 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[14 + inpos] & 32767) >>> (15 - 1))
| ((inArray[15 + inpos] & 32767) << 1)
| ((inArray[16 + inpos] & 32767) << 16)
| ((inArray[17 + inpos]) << 31);
  outArray[8 + outpos] = ((inArray[17 + inpos] & 32767) >>> (15 - 14))
| ((inArray[18 + inpos] & 32767) << 14)
| ((inArray[19 + inpos]) << 29);
  outArray[9 + outpos] = ((inArray[19 + inpos] & 32767) >>> (15 - 12))
| ((inArray[20 + inpos] & 32767) << 12)
| ((inArray[21 + inpos]) << 27);
  outArray[10 + outpos] = ((inArray[21 + inpos] & 32767) >>> (15 - 10))
| ((inArray[22 + inpos] & 32767) << 10)
| ((inArray[23 + inpos]) << 25);
  outArray[11 + outpos] = ((inArray[23 + inpos] & 32767) >>> (15 - 8))
| ((inArray[24 + inpos] & 32767) << 8)
| ((inArray[25 + inpos]) << 23);
  outArray[12 + outpos] = ((inArray[25 + inpos] & 32767) >>> (15 - 6))
| ((inArray[26 + inpos] & 32767) << 6)
| ((inArray[27 + inpos]) << 21);
  outArray[13 + outpos] = ((inArray[27 + inpos] & 32767) >>> (15 - 4))
| ((inArray[28 + inpos] & 32767) << 4)
| ((inArray[29 + inpos]) << 19);
  outArray[14 + outpos] = ((inArray[29 + inpos] & 32767) >>> (15 - 2))
| ((inArray[30 + inpos] & 32767) << 2)
| ((inArray[31 + inpos]) << 17);
}

function fastpack16(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 65535)
| ((inArray[1 + inpos]) << 16);
  outArray[1 + outpos] = (inArray[2 + inpos] & 65535)
| ((inArray[3 + inpos]) << 16);
  outArray[2 + outpos] = (inArray[4 + inpos] & 65535)
| ((inArray[5 + inpos]) << 16);
  outArray[3 + outpos] = (inArray[6 + inpos] & 65535)
| ((inArray[7 + inpos]) << 16);
  outArray[4 + outpos] = (inArray[8 + inpos] & 65535)
| ((inArray[9 + inpos]) << 16);
  outArray[5 + outpos] = (inArray[10 + inpos] & 65535)
| ((inArray[11 + inpos]) << 16);
  outArray[6 + outpos] = (inArray[12 + inpos] & 65535)
| ((inArray[13 + inpos]) << 16);
  outArray[7 + outpos] = (inArray[14 + inpos] & 65535)
| ((inArray[15 + inpos]) << 16);
  outArray[8 + outpos] = (inArray[16 + inpos] & 65535)
| ((inArray[17 + inpos]) << 16);
  outArray[9 + outpos] = (inArray[18 + inpos] & 65535)
| ((inArray[19 + inpos]) << 16);
  outArray[10 + outpos] = (inArray[20 + inpos] & 65535)
| ((inArray[21 + inpos]) << 16);
  outArray[11 + outpos] = (inArray[22 + inpos] & 65535)
| ((inArray[23 + inpos]) << 16);
  outArray[12 + outpos] = (inArray[24 + inpos] & 65535)
| ((inArray[25 + inpos]) << 16);
  outArray[13 + outpos] = (inArray[26 + inpos] & 65535)
| ((inArray[27 + inpos]) << 16);
  outArray[14 + outpos] = (inArray[28 + inpos] & 65535)
| ((inArray[29 + inpos]) << 16);
  outArray[15 + outpos] = (inArray[30 + inpos] & 65535)
| ((inArray[31 + inpos]) << 16);
}

function fastpack17(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 131071)
| ((inArray[1 + inpos]) << 17);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 131071) >>> (17 - 2))
| ((inArray[2 + inpos] & 131071) << 2)
| ((inArray[3 + inpos]) << 19);
  outArray[2 + outpos] = ((inArray[3 + inpos] & 131071) >>> (17 - 4))
| ((inArray[4 + inpos] & 131071) << 4)
| ((inArray[5 + inpos]) << 21);
  outArray[3 + outpos] = ((inArray[5 + inpos] & 131071) >>> (17 - 6))
| ((inArray[6 + inpos] & 131071) << 6)
| ((inArray[7 + inpos]) << 23);
  outArray[4 + outpos] = ((inArray[7 + inpos] & 131071) >>> (17 - 8))
| ((inArray[8 + inpos] & 131071) << 8)
| ((inArray[9 + inpos]) << 25);
  outArray[5 + outpos] = ((inArray[9 + inpos] & 131071) >>> (17 - 10))
| ((inArray[10 + inpos] & 131071) << 10)
| ((inArray[11 + inpos]) << 27);
  outArray[6 + outpos] = ((inArray[11 + inpos] & 131071) >>> (17 - 12))
| ((inArray[12 + inpos] & 131071) << 12)
| ((inArray[13 + inpos]) << 29);
  outArray[7 + outpos] = ((inArray[13 + inpos] & 131071) >>> (17 - 14))
| ((inArray[14 + inpos] & 131071) << 14)
| ((inArray[15 + inpos]) << 31);
  outArray[8 + outpos] = ((inArray[15 + inpos] & 131071) >>> (17 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[9 + outpos] = ((inArray[16 + inpos] & 131071) >>> (17 - 1))
| ((inArray[17 + inpos] & 131071) << 1)
| ((inArray[18 + inpos]) << 18);
  outArray[10 + outpos] = ((inArray[18 + inpos] & 131071) >>> (17 - 3))
| ((inArray[19 + inpos] & 131071) << 3)
| ((inArray[20 + inpos]) << 20);
  outArray[11 + outpos] = ((inArray[20 + inpos] & 131071) >>> (17 - 5))
| ((inArray[21 + inpos] & 131071) << 5)
| ((inArray[22 + inpos]) << 22);
  outArray[12 + outpos] = ((inArray[22 + inpos] & 131071) >>> (17 - 7))
| ((inArray[23 + inpos] & 131071) << 7)
| ((inArray[24 + inpos]) << 24);
  outArray[13 + outpos] = ((inArray[24 + inpos] & 131071) >>> (17 - 9))
| ((inArray[25 + inpos] & 131071) << 9)
| ((inArray[26 + inpos]) << 26);
  outArray[14 + outpos] = ((inArray[26 + inpos] & 131071) >>> (17 - 11))
| ((inArray[27 + inpos] & 131071) << 11)
| ((inArray[28 + inpos]) << 28);
  outArray[15 + outpos] = ((inArray[28 + inpos] & 131071) >>> (17 - 13))
| ((inArray[29 + inpos] & 131071) << 13)
| ((inArray[30 + inpos]) << 30);
  outArray[16 + outpos] = ((inArray[30 + inpos] & 131071) >>> (17 - 15))
| ((inArray[31 + inpos]) << 15);
}

function fastpack18(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 262143)
| ((inArray[1 + inpos]) << 18);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 262143) >>> (18 - 4))
| ((inArray[2 + inpos] & 262143) << 4)
| ((inArray[3 + inpos]) << 22);
  outArray[2 + outpos] = ((inArray[3 + inpos] & 262143) >>> (18 - 8))
| ((inArray[4 + inpos] & 262143) << 8)
| ((inArray[5 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[5 + inpos] & 262143) >>> (18 - 12))
| ((inArray[6 + inpos] & 262143) << 12)
| ((inArray[7 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[7 + inpos] & 262143) >>> (18 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[5 + outpos] = ((inArray[8 + inpos] & 262143) >>> (18 - 2))
| ((inArray[9 + inpos] & 262143) << 2)
| ((inArray[10 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[10 + inpos] & 262143) >>> (18 - 6))
| ((inArray[11 + inpos] & 262143) << 6)
| ((inArray[12 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[12 + inpos] & 262143) >>> (18 - 10))
| ((inArray[13 + inpos] & 262143) << 10)
| ((inArray[14 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[14 + inpos] & 262143) >>> (18 - 14))
| ((inArray[15 + inpos]) << 14);
  outArray[9 + outpos] = (inArray[16 + inpos] & 262143)
| ((inArray[17 + inpos]) << 18);
  outArray[10 + outpos] = ((inArray[17 + inpos] & 262143) >>> (18 - 4))
| ((inArray[18 + inpos] & 262143) << 4)
| ((inArray[19 + inpos]) << 22);
  outArray[11 + outpos] = ((inArray[19 + inpos] & 262143) >>> (18 - 8))
| ((inArray[20 + inpos] & 262143) << 8)
| ((inArray[21 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[21 + inpos] & 262143) >>> (18 - 12))
| ((inArray[22 + inpos] & 262143) << 12)
| ((inArray[23 + inpos]) << 30);
  outArray[13 + outpos] = ((inArray[23 + inpos] & 262143) >>> (18 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[24 + inpos] & 262143) >>> (18 - 2))
| ((inArray[25 + inpos] & 262143) << 2)
| ((inArray[26 + inpos]) << 20);
  outArray[15 + outpos] = ((inArray[26 + inpos] & 262143) >>> (18 - 6))
| ((inArray[27 + inpos] & 262143) << 6)
| ((inArray[28 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[28 + inpos] & 262143) >>> (18 - 10))
| ((inArray[29 + inpos] & 262143) << 10)
| ((inArray[30 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[30 + inpos] & 262143) >>> (18 - 14))
| ((inArray[31 + inpos]) << 14);
}

function fastpack19(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 524287)
| ((inArray[1 + inpos]) << 19);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 524287) >>> (19 - 6))
| ((inArray[2 + inpos] & 524287) << 6)
| ((inArray[3 + inpos]) << 25);
  outArray[2 + outpos] = ((inArray[3 + inpos] & 524287) >>> (19 - 12))
| ((inArray[4 + inpos] & 524287) << 12)
| ((inArray[5 + inpos]) << 31);
  outArray[3 + outpos] = ((inArray[5 + inpos] & 524287) >>> (19 - 18))
| ((inArray[6 + inpos]) << 18);
  outArray[4 + outpos] = ((inArray[6 + inpos] & 524287) >>> (19 - 5))
| ((inArray[7 + inpos] & 524287) << 5)
| ((inArray[8 + inpos]) << 24);
  outArray[5 + outpos] = ((inArray[8 + inpos] & 524287) >>> (19 - 11))
| ((inArray[9 + inpos] & 524287) << 11)
| ((inArray[10 + inpos]) << 30);
  outArray[6 + outpos] = ((inArray[10 + inpos] & 524287) >>> (19 - 17))
| ((inArray[11 + inpos]) << 17);
  outArray[7 + outpos] = ((inArray[11 + inpos] & 524287) >>> (19 - 4))
| ((inArray[12 + inpos] & 524287) << 4)
| ((inArray[13 + inpos]) << 23);
  outArray[8 + outpos] = ((inArray[13 + inpos] & 524287) >>> (19 - 10))
| ((inArray[14 + inpos] & 524287) << 10)
| ((inArray[15 + inpos]) << 29);
  outArray[9 + outpos] = ((inArray[15 + inpos] & 524287) >>> (19 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[10 + outpos] = ((inArray[16 + inpos] & 524287) >>> (19 - 3))
| ((inArray[17 + inpos] & 524287) << 3)
| ((inArray[18 + inpos]) << 22);
  outArray[11 + outpos] = ((inArray[18 + inpos] & 524287) >>> (19 - 9))
| ((inArray[19 + inpos] & 524287) << 9)
| ((inArray[20 + inpos]) << 28);
  outArray[12 + outpos] = ((inArray[20 + inpos] & 524287) >>> (19 - 15))
| ((inArray[21 + inpos]) << 15);
  outArray[13 + outpos] = ((inArray[21 + inpos] & 524287) >>> (19 - 2))
| ((inArray[22 + inpos] & 524287) << 2)
| ((inArray[23 + inpos]) << 21);
  outArray[14 + outpos] = ((inArray[23 + inpos] & 524287) >>> (19 - 8))
| ((inArray[24 + inpos] & 524287) << 8)
| ((inArray[25 + inpos]) << 27);
  outArray[15 + outpos] = ((inArray[25 + inpos] & 524287) >>> (19 - 14))
| ((inArray[26 + inpos]) << 14);
  outArray[16 + outpos] = ((inArray[26 + inpos] & 524287) >>> (19 - 1))
| ((inArray[27 + inpos] & 524287) << 1)
| ((inArray[28 + inpos]) << 20);
  outArray[17 + outpos] = ((inArray[28 + inpos] & 524287) >>> (19 - 7))
| ((inArray[29 + inpos] & 524287) << 7)
| ((inArray[30 + inpos]) << 26);
  outArray[18 + outpos] = ((inArray[30 + inpos] & 524287) >>> (19 - 13))
| ((inArray[31 + inpos]) << 13);
}

function fastpack2(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 3)
| ((inArray[1 + inpos] & 3) << 2)
| ((inArray[2 + inpos] & 3) << 4)
| ((inArray[3 + inpos] & 3) << 6)
| ((inArray[4 + inpos] & 3) << 8)
| ((inArray[5 + inpos] & 3) << 10)
| ((inArray[6 + inpos] & 3) << 12)
| ((inArray[7 + inpos] & 3) << 14)
| ((inArray[8 + inpos] & 3) << 16)
| ((inArray[9 + inpos] & 3) << 18)
| ((inArray[10 + inpos] & 3) << 20)
| ((inArray[11 + inpos] & 3) << 22)
| ((inArray[12 + inpos] & 3) << 24)
| ((inArray[13 + inpos] & 3) << 26)
| ((inArray[14 + inpos] & 3) << 28)
| ((inArray[15 + inpos]) << 30);
  outArray[1 + outpos] = (inArray[16 + inpos] & 3)
| ((inArray[17 + inpos] & 3) << 2)
| ((inArray[18 + inpos] & 3) << 4)
| ((inArray[19 + inpos] & 3) << 6)
| ((inArray[20 + inpos] & 3) << 8)
| ((inArray[21 + inpos] & 3) << 10)
| ((inArray[22 + inpos] & 3) << 12)
| ((inArray[23 + inpos] & 3) << 14)
| ((inArray[24 + inpos] & 3) << 16)
| ((inArray[25 + inpos] & 3) << 18)
| ((inArray[26 + inpos] & 3) << 20)
| ((inArray[27 + inpos] & 3) << 22)
| ((inArray[28 + inpos] & 3) << 24)
| ((inArray[29 + inpos] & 3) << 26)
| ((inArray[30 + inpos] & 3) << 28)
| ((inArray[31 + inpos]) << 30);
}

function fastpack20(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 1048575)
| ((inArray[1 + inpos]) << 20);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 1048575) >>> (20 - 8))
| ((inArray[2 + inpos] & 1048575) << 8)
| ((inArray[3 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[3 + inpos] & 1048575) >>> (20 - 16))
| ((inArray[4 + inpos]) << 16);
  outArray[3 + outpos] = ((inArray[4 + inpos] & 1048575) >>> (20 - 4))
| ((inArray[5 + inpos] & 1048575) << 4)
| ((inArray[6 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[6 + inpos] & 1048575) >>> (20 - 12))
| ((inArray[7 + inpos]) << 12);
  outArray[5 + outpos] = (inArray[8 + inpos] & 1048575)
| ((inArray[9 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[9 + inpos] & 1048575) >>> (20 - 8))
| ((inArray[10 + inpos] & 1048575) << 8)
| ((inArray[11 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[11 + inpos] & 1048575) >>> (20 - 16))
| ((inArray[12 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[12 + inpos] & 1048575) >>> (20 - 4))
| ((inArray[13 + inpos] & 1048575) << 4)
| ((inArray[14 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[14 + inpos] & 1048575) >>> (20 - 12))
| ((inArray[15 + inpos]) << 12);
  outArray[10 + outpos] = (inArray[16 + inpos] & 1048575)
| ((inArray[17 + inpos]) << 20);
  outArray[11 + outpos] = ((inArray[17 + inpos] & 1048575) >>> (20 - 8))
| ((inArray[18 + inpos] & 1048575) << 8)
| ((inArray[19 + inpos]) << 28);
  outArray[12 + outpos] = ((inArray[19 + inpos] & 1048575) >>> (20 - 16))
| ((inArray[20 + inpos]) << 16);
  outArray[13 + outpos] = ((inArray[20 + inpos] & 1048575) >>> (20 - 4))
| ((inArray[21 + inpos] & 1048575) << 4)
| ((inArray[22 + inpos]) << 24);
  outArray[14 + outpos] = ((inArray[22 + inpos] & 1048575) >>> (20 - 12))
| ((inArray[23 + inpos]) << 12);
  outArray[15 + outpos] = (inArray[24 + inpos] & 1048575)
| ((inArray[25 + inpos]) << 20);
  outArray[16 + outpos] = ((inArray[25 + inpos] & 1048575) >>> (20 - 8))
| ((inArray[26 + inpos] & 1048575) << 8)
| ((inArray[27 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[27 + inpos] & 1048575) >>> (20 - 16))
| ((inArray[28 + inpos]) << 16);
  outArray[18 + outpos] = ((inArray[28 + inpos] & 1048575) >>> (20 - 4))
| ((inArray[29 + inpos] & 1048575) << 4)
| ((inArray[30 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[30 + inpos] & 1048575) >>> (20 - 12))
| ((inArray[31 + inpos]) << 12);
}

function fastpack21(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 2097151)
| ((inArray[1 + inpos]) << 21);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 2097151) >>> (21 - 10))
| ((inArray[2 + inpos] & 2097151) << 10)
| ((inArray[3 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[3 + inpos] & 2097151) >>> (21 - 20))
| ((inArray[4 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[4 + inpos] & 2097151) >>> (21 - 9))
| ((inArray[5 + inpos] & 2097151) << 9)
| ((inArray[6 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[6 + inpos] & 2097151) >>> (21 - 19))
| ((inArray[7 + inpos]) << 19);
  outArray[5 + outpos] = ((inArray[7 + inpos] & 2097151) >>> (21 - 8))
| ((inArray[8 + inpos] & 2097151) << 8)
| ((inArray[9 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[9 + inpos] & 2097151) >>> (21 - 18))
| ((inArray[10 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[10 + inpos] & 2097151) >>> (21 - 7))
| ((inArray[11 + inpos] & 2097151) << 7)
| ((inArray[12 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[12 + inpos] & 2097151) >>> (21 - 17))
| ((inArray[13 + inpos]) << 17);
  outArray[9 + outpos] = ((inArray[13 + inpos] & 2097151) >>> (21 - 6))
| ((inArray[14 + inpos] & 2097151) << 6)
| ((inArray[15 + inpos]) << 27);
  outArray[10 + outpos] = ((inArray[15 + inpos] & 2097151) >>> (21 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[16 + inpos] & 2097151) >>> (21 - 5))
| ((inArray[17 + inpos] & 2097151) << 5)
| ((inArray[18 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[18 + inpos] & 2097151) >>> (21 - 15))
| ((inArray[19 + inpos]) << 15);
  outArray[13 + outpos] = ((inArray[19 + inpos] & 2097151) >>> (21 - 4))
| ((inArray[20 + inpos] & 2097151) << 4)
| ((inArray[21 + inpos]) << 25);
  outArray[14 + outpos] = ((inArray[21 + inpos] & 2097151) >>> (21 - 14))
| ((inArray[22 + inpos]) << 14);
  outArray[15 + outpos] = ((inArray[22 + inpos] & 2097151) >>> (21 - 3))
| ((inArray[23 + inpos] & 2097151) << 3)
| ((inArray[24 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[24 + inpos] & 2097151) >>> (21 - 13))
| ((inArray[25 + inpos]) << 13);
  outArray[17 + outpos] = ((inArray[25 + inpos] & 2097151) >>> (21 - 2))
| ((inArray[26 + inpos] & 2097151) << 2)
| ((inArray[27 + inpos]) << 23);
  outArray[18 + outpos] = ((inArray[27 + inpos] & 2097151) >>> (21 - 12))
| ((inArray[28 + inpos]) << 12);
  outArray[19 + outpos] = ((inArray[28 + inpos] & 2097151) >>> (21 - 1))
| ((inArray[29 + inpos] & 2097151) << 1)
| ((inArray[30 + inpos]) << 22);
  outArray[20 + outpos] = ((inArray[30 + inpos] & 2097151) >>> (21 - 11))
| ((inArray[31 + inpos]) << 11);
}

function fastpack22(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 4194303)
| ((inArray[1 + inpos]) << 22);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 4194303) >>> (22 - 12))
| ((inArray[2 + inpos]) << 12);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 4194303) >>> (22 - 2))
| ((inArray[3 + inpos] & 4194303) << 2)
| ((inArray[4 + inpos]) << 24);
  outArray[3 + outpos] = ((inArray[4 + inpos] & 4194303) >>> (22 - 14))
| ((inArray[5 + inpos]) << 14);
  outArray[4 + outpos] = ((inArray[5 + inpos] & 4194303) >>> (22 - 4))
| ((inArray[6 + inpos] & 4194303) << 4)
| ((inArray[7 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[7 + inpos] & 4194303) >>> (22 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[6 + outpos] = ((inArray[8 + inpos] & 4194303) >>> (22 - 6))
| ((inArray[9 + inpos] & 4194303) << 6)
| ((inArray[10 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[10 + inpos] & 4194303) >>> (22 - 18))
| ((inArray[11 + inpos]) << 18);
  outArray[8 + outpos] = ((inArray[11 + inpos] & 4194303) >>> (22 - 8))
| ((inArray[12 + inpos] & 4194303) << 8)
| ((inArray[13 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[13 + inpos] & 4194303) >>> (22 - 20))
| ((inArray[14 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[14 + inpos] & 4194303) >>> (22 - 10))
| ((inArray[15 + inpos]) << 10);
  outArray[11 + outpos] = (inArray[16 + inpos] & 4194303)
| ((inArray[17 + inpos]) << 22);
  outArray[12 + outpos] = ((inArray[17 + inpos] & 4194303) >>> (22 - 12))
| ((inArray[18 + inpos]) << 12);
  outArray[13 + outpos] = ((inArray[18 + inpos] & 4194303) >>> (22 - 2))
| ((inArray[19 + inpos] & 4194303) << 2)
| ((inArray[20 + inpos]) << 24);
  outArray[14 + outpos] = ((inArray[20 + inpos] & 4194303) >>> (22 - 14))
| ((inArray[21 + inpos]) << 14);
  outArray[15 + outpos] = ((inArray[21 + inpos] & 4194303) >>> (22 - 4))
| ((inArray[22 + inpos] & 4194303) << 4)
| ((inArray[23 + inpos]) << 26);
  outArray[16 + outpos] = ((inArray[23 + inpos] & 4194303) >>> (22 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[17 + outpos] = ((inArray[24 + inpos] & 4194303) >>> (22 - 6))
| ((inArray[25 + inpos] & 4194303) << 6)
| ((inArray[26 + inpos]) << 28);
  outArray[18 + outpos] = ((inArray[26 + inpos] & 4194303) >>> (22 - 18))
| ((inArray[27 + inpos]) << 18);
  outArray[19 + outpos] = ((inArray[27 + inpos] & 4194303) >>> (22 - 8))
| ((inArray[28 + inpos] & 4194303) << 8)
| ((inArray[29 + inpos]) << 30);
  outArray[20 + outpos] = ((inArray[29 + inpos] & 4194303) >>> (22 - 20))
| ((inArray[30 + inpos]) << 20);
  outArray[21 + outpos] = ((inArray[30 + inpos] & 4194303) >>> (22 - 10))
| ((inArray[31 + inpos]) << 10);
}

function fastpack23(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 8388607)
| ((inArray[1 + inpos]) << 23);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 8388607) >>> (23 - 14))
| ((inArray[2 + inpos]) << 14);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 8388607) >>> (23 - 5))
| ((inArray[3 + inpos] & 8388607) << 5)
| ((inArray[4 + inpos]) << 28);
  outArray[3 + outpos] = ((inArray[4 + inpos] & 8388607) >>> (23 - 19))
| ((inArray[5 + inpos]) << 19);
  outArray[4 + outpos] = ((inArray[5 + inpos] & 8388607) >>> (23 - 10))
| ((inArray[6 + inpos]) << 10);
  outArray[5 + outpos] = ((inArray[6 + inpos] & 8388607) >>> (23 - 1))
| ((inArray[7 + inpos] & 8388607) << 1)
| ((inArray[8 + inpos]) << 24);
  outArray[6 + outpos] = ((inArray[8 + inpos] & 8388607) >>> (23 - 15))
| ((inArray[9 + inpos]) << 15);
  outArray[7 + outpos] = ((inArray[9 + inpos] & 8388607) >>> (23 - 6))
| ((inArray[10 + inpos] & 8388607) << 6)
| ((inArray[11 + inpos]) << 29);
  outArray[8 + outpos] = ((inArray[11 + inpos] & 8388607) >>> (23 - 20))
| ((inArray[12 + inpos]) << 20);
  outArray[9 + outpos] = ((inArray[12 + inpos] & 8388607) >>> (23 - 11))
| ((inArray[13 + inpos]) << 11);
  outArray[10 + outpos] = ((inArray[13 + inpos] & 8388607) >>> (23 - 2))
| ((inArray[14 + inpos] & 8388607) << 2)
| ((inArray[15 + inpos]) << 25);
  outArray[11 + outpos] = ((inArray[15 + inpos] & 8388607) >>> (23 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[12 + outpos] = ((inArray[16 + inpos] & 8388607) >>> (23 - 7))
| ((inArray[17 + inpos] & 8388607) << 7)
| ((inArray[18 + inpos]) << 30);
  outArray[13 + outpos] = ((inArray[18 + inpos] & 8388607) >>> (23 - 21))
| ((inArray[19 + inpos]) << 21);
  outArray[14 + outpos] = ((inArray[19 + inpos] & 8388607) >>> (23 - 12))
| ((inArray[20 + inpos]) << 12);
  outArray[15 + outpos] = ((inArray[20 + inpos] & 8388607) >>> (23 - 3))
| ((inArray[21 + inpos] & 8388607) << 3)
| ((inArray[22 + inpos]) << 26);
  outArray[16 + outpos] = ((inArray[22 + inpos] & 8388607) >>> (23 - 17))
| ((inArray[23 + inpos]) << 17);
  outArray[17 + outpos] = ((inArray[23 + inpos] & 8388607) >>> (23 - 8))
| ((inArray[24 + inpos] & 8388607) << 8)
| ((inArray[25 + inpos]) << 31);
  outArray[18 + outpos] = ((inArray[25 + inpos] & 8388607) >>> (23 - 22))
| ((inArray[26 + inpos]) << 22);
  outArray[19 + outpos] = ((inArray[26 + inpos] & 8388607) >>> (23 - 13))
| ((inArray[27 + inpos]) << 13);
  outArray[20 + outpos] = ((inArray[27 + inpos] & 8388607) >>> (23 - 4))
| ((inArray[28 + inpos] & 8388607) << 4)
| ((inArray[29 + inpos]) << 27);
  outArray[21 + outpos] = ((inArray[29 + inpos] & 8388607) >>> (23 - 18))
| ((inArray[30 + inpos]) << 18);
  outArray[22 + outpos] = ((inArray[30 + inpos] & 8388607) >>> (23 - 9))
| ((inArray[31 + inpos]) << 9);
}

function fastpack24(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 16777215)
| ((inArray[1 + inpos]) << 24);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[2 + inpos]) << 16);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[3 + inpos]) << 8);
  outArray[3 + outpos] = (inArray[4 + inpos] & 16777215)
| ((inArray[5 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[5 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[6 + inpos]) << 16);
  outArray[5 + outpos] = ((inArray[6 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[7 + inpos]) << 8);
  outArray[6 + outpos] = (inArray[8 + inpos] & 16777215)
| ((inArray[9 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[9 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[10 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[10 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[11 + inpos]) << 8);
  outArray[9 + outpos] = (inArray[12 + inpos] & 16777215)
| ((inArray[13 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[13 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[14 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[14 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[15 + inpos]) << 8);
  outArray[12 + outpos] = (inArray[16 + inpos] & 16777215)
| ((inArray[17 + inpos]) << 24);
  outArray[13 + outpos] = ((inArray[17 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[18 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[18 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[19 + inpos]) << 8);
  outArray[15 + outpos] = (inArray[20 + inpos] & 16777215)
| ((inArray[21 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[21 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[22 + inpos]) << 16);
  outArray[17 + outpos] = ((inArray[22 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[23 + inpos]) << 8);
  outArray[18 + outpos] = (inArray[24 + inpos] & 16777215)
| ((inArray[25 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[25 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[26 + inpos]) << 16);
  outArray[20 + outpos] = ((inArray[26 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[27 + inpos]) << 8);
  outArray[21 + outpos] = (inArray[28 + inpos] & 16777215)
| ((inArray[29 + inpos]) << 24);
  outArray[22 + outpos] = ((inArray[29 + inpos] & 16777215) >>> (24 - 16))
| ((inArray[30 + inpos]) << 16);
  outArray[23 + outpos] = ((inArray[30 + inpos] & 16777215) >>> (24 - 8))
| ((inArray[31 + inpos]) << 8);
}

function fastpack25(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 33554431)
| ((inArray[1 + inpos]) << 25);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 33554431) >>> (25 - 18))
| ((inArray[2 + inpos]) << 18);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 33554431) >>> (25 - 11))
| ((inArray[3 + inpos]) << 11);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 33554431) >>> (25 - 4))
| ((inArray[4 + inpos] & 33554431) << 4)
| ((inArray[5 + inpos]) << 29);
  outArray[4 + outpos] = ((inArray[5 + inpos] & 33554431) >>> (25 - 22))
| ((inArray[6 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[6 + inpos] & 33554431) >>> (25 - 15))
| ((inArray[7 + inpos]) << 15);
  outArray[6 + outpos] = ((inArray[7 + inpos] & 33554431) >>> (25 - 8))
| ((inArray[8 + inpos]) << 8);
  outArray[7 + outpos] = ((inArray[8 + inpos] & 33554431) >>> (25 - 1))
| ((inArray[9 + inpos] & 33554431) << 1)
| ((inArray[10 + inpos]) << 26);
  outArray[8 + outpos] = ((inArray[10 + inpos] & 33554431) >>> (25 - 19))
| ((inArray[11 + inpos]) << 19);
  outArray[9 + outpos] = ((inArray[11 + inpos] & 33554431) >>> (25 - 12))
| ((inArray[12 + inpos]) << 12);
  outArray[10 + outpos] = ((inArray[12 + inpos] & 33554431) >>> (25 - 5))
| ((inArray[13 + inpos] & 33554431) << 5)
| ((inArray[14 + inpos]) << 30);
  outArray[11 + outpos] = ((inArray[14 + inpos] & 33554431) >>> (25 - 23))
| ((inArray[15 + inpos]) << 23);
  outArray[12 + outpos] = ((inArray[15 + inpos] & 33554431) >>> (25 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[13 + outpos] = ((inArray[16 + inpos] & 33554431) >>> (25 - 9))
| ((inArray[17 + inpos]) << 9);
  outArray[14 + outpos] = ((inArray[17 + inpos] & 33554431) >>> (25 - 2))
| ((inArray[18 + inpos] & 33554431) << 2)
| ((inArray[19 + inpos]) << 27);
  outArray[15 + outpos] = ((inArray[19 + inpos] & 33554431) >>> (25 - 20))
| ((inArray[20 + inpos]) << 20);
  outArray[16 + outpos] = ((inArray[20 + inpos] & 33554431) >>> (25 - 13))
| ((inArray[21 + inpos]) << 13);
  outArray[17 + outpos] = ((inArray[21 + inpos] & 33554431) >>> (25 - 6))
| ((inArray[22 + inpos] & 33554431) << 6)
| ((inArray[23 + inpos]) << 31);
  outArray[18 + outpos] = ((inArray[23 + inpos] & 33554431) >>> (25 - 24))
| ((inArray[24 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[24 + inpos] & 33554431) >>> (25 - 17))
| ((inArray[25 + inpos]) << 17);
  outArray[20 + outpos] = ((inArray[25 + inpos] & 33554431) >>> (25 - 10))
| ((inArray[26 + inpos]) << 10);
  outArray[21 + outpos] = ((inArray[26 + inpos] & 33554431) >>> (25 - 3))
| ((inArray[27 + inpos] & 33554431) << 3)
| ((inArray[28 + inpos]) << 28);
  outArray[22 + outpos] = ((inArray[28 + inpos] & 33554431) >>> (25 - 21))
| ((inArray[29 + inpos]) << 21);
  outArray[23 + outpos] = ((inArray[29 + inpos] & 33554431) >>> (25 - 14))
| ((inArray[30 + inpos]) << 14);
  outArray[24 + outpos] = ((inArray[30 + inpos] & 33554431) >>> (25 - 7))
| ((inArray[31 + inpos]) << 7);
}

function fastpack26(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 67108863)
| ((inArray[1 + inpos]) << 26);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 67108863) >>> (26 - 20))
| ((inArray[2 + inpos]) << 20);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 67108863) >>> (26 - 14))
| ((inArray[3 + inpos]) << 14);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 67108863) >>> (26 - 8))
| ((inArray[4 + inpos]) << 8);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 67108863) >>> (26 - 2))
| ((inArray[5 + inpos] & 67108863) << 2)
| ((inArray[6 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[6 + inpos] & 67108863) >>> (26 - 22))
| ((inArray[7 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[7 + inpos] & 67108863) >>> (26 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[7 + outpos] = ((inArray[8 + inpos] & 67108863) >>> (26 - 10))
| ((inArray[9 + inpos]) << 10);
  outArray[8 + outpos] = ((inArray[9 + inpos] & 67108863) >>> (26 - 4))
| ((inArray[10 + inpos] & 67108863) << 4)
| ((inArray[11 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[11 + inpos] & 67108863) >>> (26 - 24))
| ((inArray[12 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[12 + inpos] & 67108863) >>> (26 - 18))
| ((inArray[13 + inpos]) << 18);
  outArray[11 + outpos] = ((inArray[13 + inpos] & 67108863) >>> (26 - 12))
| ((inArray[14 + inpos]) << 12);
  outArray[12 + outpos] = ((inArray[14 + inpos] & 67108863) >>> (26 - 6))
| ((inArray[15 + inpos]) << 6);
  outArray[13 + outpos] = (inArray[16 + inpos] & 67108863)
| ((inArray[17 + inpos]) << 26);
  outArray[14 + outpos] = ((inArray[17 + inpos] & 67108863) >>> (26 - 20))
| ((inArray[18 + inpos]) << 20);
  outArray[15 + outpos] = ((inArray[18 + inpos] & 67108863) >>> (26 - 14))
| ((inArray[19 + inpos]) << 14);
  outArray[16 + outpos] = ((inArray[19 + inpos] & 67108863) >>> (26 - 8))
| ((inArray[20 + inpos]) << 8);
  outArray[17 + outpos] = ((inArray[20 + inpos] & 67108863) >>> (26 - 2))
| ((inArray[21 + inpos] & 67108863) << 2)
| ((inArray[22 + inpos]) << 28);
  outArray[18 + outpos] = ((inArray[22 + inpos] & 67108863) >>> (26 - 22))
| ((inArray[23 + inpos]) << 22);
  outArray[19 + outpos] = ((inArray[23 + inpos] & 67108863) >>> (26 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[20 + outpos] = ((inArray[24 + inpos] & 67108863) >>> (26 - 10))
| ((inArray[25 + inpos]) << 10);
  outArray[21 + outpos] = ((inArray[25 + inpos] & 67108863) >>> (26 - 4))
| ((inArray[26 + inpos] & 67108863) << 4)
| ((inArray[27 + inpos]) << 30);
  outArray[22 + outpos] = ((inArray[27 + inpos] & 67108863) >>> (26 - 24))
| ((inArray[28 + inpos]) << 24);
  outArray[23 + outpos] = ((inArray[28 + inpos] & 67108863) >>> (26 - 18))
| ((inArray[29 + inpos]) << 18);
  outArray[24 + outpos] = ((inArray[29 + inpos] & 67108863) >>> (26 - 12))
| ((inArray[30 + inpos]) << 12);
  outArray[25 + outpos] = ((inArray[30 + inpos] & 67108863) >>> (26 - 6))
| ((inArray[31 + inpos]) << 6);
}

function fastpack27(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 134217727)
| ((inArray[1 + inpos]) << 27);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 134217727) >>> (27 - 22))
| ((inArray[2 + inpos]) << 22);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 134217727) >>> (27 - 17))
| ((inArray[3 + inpos]) << 17);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 134217727) >>> (27 - 12))
| ((inArray[4 + inpos]) << 12);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 134217727) >>> (27 - 7))
| ((inArray[5 + inpos]) << 7);
  outArray[5 + outpos] = ((inArray[5 + inpos] & 134217727) >>> (27 - 2))
| ((inArray[6 + inpos] & 134217727) << 2)
| ((inArray[7 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[7 + inpos] & 134217727) >>> (27 - 24))
| ((inArray[8 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[8 + inpos] & 134217727) >>> (27 - 19))
| ((inArray[9 + inpos]) << 19);
  outArray[8 + outpos] = ((inArray[9 + inpos] & 134217727) >>> (27 - 14))
| ((inArray[10 + inpos]) << 14);
  outArray[9 + outpos] = ((inArray[10 + inpos] & 134217727) >>> (27 - 9))
| ((inArray[11 + inpos]) << 9);
  outArray[10 + outpos] = ((inArray[11 + inpos] & 134217727) >>> (27 - 4))
| ((inArray[12 + inpos] & 134217727) << 4)
| ((inArray[13 + inpos]) << 31);
  outArray[11 + outpos] = ((inArray[13 + inpos] & 134217727) >>> (27 - 26))
| ((inArray[14 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[14 + inpos] & 134217727) >>> (27 - 21))
| ((inArray[15 + inpos]) << 21);
  outArray[13 + outpos] = ((inArray[15 + inpos] & 134217727) >>> (27 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[16 + inpos] & 134217727) >>> (27 - 11))
| ((inArray[17 + inpos]) << 11);
  outArray[15 + outpos] = ((inArray[17 + inpos] & 134217727) >>> (27 - 6))
| ((inArray[18 + inpos]) << 6);
  outArray[16 + outpos] = ((inArray[18 + inpos] & 134217727) >>> (27 - 1))
| ((inArray[19 + inpos] & 134217727) << 1)
| ((inArray[20 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[20 + inpos] & 134217727) >>> (27 - 23))
| ((inArray[21 + inpos]) << 23);
  outArray[18 + outpos] = ((inArray[21 + inpos] & 134217727) >>> (27 - 18))
| ((inArray[22 + inpos]) << 18);
  outArray[19 + outpos] = ((inArray[22 + inpos] & 134217727) >>> (27 - 13))
| ((inArray[23 + inpos]) << 13);
  outArray[20 + outpos] = ((inArray[23 + inpos] & 134217727) >>> (27 - 8))
| ((inArray[24 + inpos]) << 8);
  outArray[21 + outpos] = ((inArray[24 + inpos] & 134217727) >>> (27 - 3))
| ((inArray[25 + inpos] & 134217727) << 3)
| ((inArray[26 + inpos]) << 30);
  outArray[22 + outpos] = ((inArray[26 + inpos] & 134217727) >>> (27 - 25))
| ((inArray[27 + inpos]) << 25);
  outArray[23 + outpos] = ((inArray[27 + inpos] & 134217727) >>> (27 - 20))
| ((inArray[28 + inpos]) << 20);
  outArray[24 + outpos] = ((inArray[28 + inpos] & 134217727) >>> (27 - 15))
| ((inArray[29 + inpos]) << 15);
  outArray[25 + outpos] = ((inArray[29 + inpos] & 134217727) >>> (27 - 10))
| ((inArray[30 + inpos]) << 10);
  outArray[26 + outpos] = ((inArray[30 + inpos] & 134217727) >>> (27 - 5))
| ((inArray[31 + inpos]) << 5);
}

function fastpack28(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 268435455)
| ((inArray[1 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 268435455) >>> (28 - 24))
| ((inArray[2 + inpos]) << 24);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 268435455) >>> (28 - 20))
| ((inArray[3 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 268435455) >>> (28 - 16))
| ((inArray[4 + inpos]) << 16);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 268435455) >>> (28 - 12))
| ((inArray[5 + inpos]) << 12);
  outArray[5 + outpos] = ((inArray[5 + inpos] & 268435455) >>> (28 - 8))
| ((inArray[6 + inpos]) << 8);
  outArray[6 + outpos] = ((inArray[6 + inpos] & 268435455) >>> (28 - 4))
| ((inArray[7 + inpos]) << 4);
  outArray[7 + outpos] = (inArray[8 + inpos] & 268435455)
| ((inArray[9 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[9 + inpos] & 268435455) >>> (28 - 24))
| ((inArray[10 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[10 + inpos] & 268435455) >>> (28 - 20))
| ((inArray[11 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[11 + inpos] & 268435455) >>> (28 - 16))
| ((inArray[12 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[12 + inpos] & 268435455) >>> (28 - 12))
| ((inArray[13 + inpos]) << 12);
  outArray[12 + outpos] = ((inArray[13 + inpos] & 268435455) >>> (28 - 8))
| ((inArray[14 + inpos]) << 8);
  outArray[13 + outpos] = ((inArray[14 + inpos] & 268435455) >>> (28 - 4))
| ((inArray[15 + inpos]) << 4);
  outArray[14 + outpos] = (inArray[16 + inpos] & 268435455)
| ((inArray[17 + inpos]) << 28);
  outArray[15 + outpos] = ((inArray[17 + inpos] & 268435455) >>> (28 - 24))
| ((inArray[18 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[18 + inpos] & 268435455) >>> (28 - 20))
| ((inArray[19 + inpos]) << 20);
  outArray[17 + outpos] = ((inArray[19 + inpos] & 268435455) >>> (28 - 16))
| ((inArray[20 + inpos]) << 16);
  outArray[18 + outpos] = ((inArray[20 + inpos] & 268435455) >>> (28 - 12))
| ((inArray[21 + inpos]) << 12);
  outArray[19 + outpos] = ((inArray[21 + inpos] & 268435455) >>> (28 - 8))
| ((inArray[22 + inpos]) << 8);
  outArray[20 + outpos] = ((inArray[22 + inpos] & 268435455) >>> (28 - 4))
| ((inArray[23 + inpos]) << 4);
  outArray[21 + outpos] = (inArray[24 + inpos] & 268435455)
| ((inArray[25 + inpos]) << 28);
  outArray[22 + outpos] = ((inArray[25 + inpos] & 268435455) >>> (28 - 24))
| ((inArray[26 + inpos]) << 24);
  outArray[23 + outpos] = ((inArray[26 + inpos] & 268435455) >>> (28 - 20))
| ((inArray[27 + inpos]) << 20);
  outArray[24 + outpos] = ((inArray[27 + inpos] & 268435455) >>> (28 - 16))
| ((inArray[28 + inpos]) << 16);
  outArray[25 + outpos] = ((inArray[28 + inpos] & 268435455) >>> (28 - 12))
| ((inArray[29 + inpos]) << 12);
  outArray[26 + outpos] = ((inArray[29 + inpos] & 268435455) >>> (28 - 8))
| ((inArray[30 + inpos]) << 8);
  outArray[27 + outpos] = ((inArray[30 + inpos] & 268435455) >>> (28 - 4))
| ((inArray[31 + inpos]) << 4);
}

function fastpack29(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 536870911)
| ((inArray[1 + inpos]) << 29);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 536870911) >>> (29 - 26))
| ((inArray[2 + inpos]) << 26);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 536870911) >>> (29 - 23))
| ((inArray[3 + inpos]) << 23);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 536870911) >>> (29 - 20))
| ((inArray[4 + inpos]) << 20);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 536870911) >>> (29 - 17))
| ((inArray[5 + inpos]) << 17);
  outArray[5 + outpos] = ((inArray[5 + inpos] & 536870911) >>> (29 - 14))
| ((inArray[6 + inpos]) << 14);
  outArray[6 + outpos] = ((inArray[6 + inpos] & 536870911) >>> (29 - 11))
| ((inArray[7 + inpos]) << 11);
  outArray[7 + outpos] = ((inArray[7 + inpos] & 536870911) >>> (29 - 8))
| ((inArray[8 + inpos]) << 8);
  outArray[8 + outpos] = ((inArray[8 + inpos] & 536870911) >>> (29 - 5))
| ((inArray[9 + inpos]) << 5);
  outArray[9 + outpos] = ((inArray[9 + inpos] & 536870911) >>> (29 - 2))
| ((inArray[10 + inpos] & 536870911) << 2)
| ((inArray[11 + inpos]) << 31);
  outArray[10 + outpos] = ((inArray[11 + inpos] & 536870911) >>> (29 - 28))
| ((inArray[12 + inpos]) << 28);
  outArray[11 + outpos] = ((inArray[12 + inpos] & 536870911) >>> (29 - 25))
| ((inArray[13 + inpos]) << 25);
  outArray[12 + outpos] = ((inArray[13 + inpos] & 536870911) >>> (29 - 22))
| ((inArray[14 + inpos]) << 22);
  outArray[13 + outpos] = ((inArray[14 + inpos] & 536870911) >>> (29 - 19))
| ((inArray[15 + inpos]) << 19);
  outArray[14 + outpos] = ((inArray[15 + inpos] & 536870911) >>> (29 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[15 + outpos] = ((inArray[16 + inpos] & 536870911) >>> (29 - 13))
| ((inArray[17 + inpos]) << 13);
  outArray[16 + outpos] = ((inArray[17 + inpos] & 536870911) >>> (29 - 10))
| ((inArray[18 + inpos]) << 10);
  outArray[17 + outpos] = ((inArray[18 + inpos] & 536870911) >>> (29 - 7))
| ((inArray[19 + inpos]) << 7);
  outArray[18 + outpos] = ((inArray[19 + inpos] & 536870911) >>> (29 - 4))
| ((inArray[20 + inpos]) << 4);
  outArray[19 + outpos] = ((inArray[20 + inpos] & 536870911) >>> (29 - 1))
| ((inArray[21 + inpos] & 536870911) << 1)
| ((inArray[22 + inpos]) << 30);
  outArray[20 + outpos] = ((inArray[22 + inpos] & 536870911) >>> (29 - 27))
| ((inArray[23 + inpos]) << 27);
  outArray[21 + outpos] = ((inArray[23 + inpos] & 536870911) >>> (29 - 24))
| ((inArray[24 + inpos]) << 24);
  outArray[22 + outpos] = ((inArray[24 + inpos] & 536870911) >>> (29 - 21))
| ((inArray[25 + inpos]) << 21);
  outArray[23 + outpos] = ((inArray[25 + inpos] & 536870911) >>> (29 - 18))
| ((inArray[26 + inpos]) << 18);
  outArray[24 + outpos] = ((inArray[26 + inpos] & 536870911) >>> (29 - 15))
| ((inArray[27 + inpos]) << 15);
  outArray[25 + outpos] = ((inArray[27 + inpos] & 536870911) >>> (29 - 12))
| ((inArray[28 + inpos]) << 12);
  outArray[26 + outpos] = ((inArray[28 + inpos] & 536870911) >>> (29 - 9))
| ((inArray[29 + inpos]) << 9);
  outArray[27 + outpos] = ((inArray[29 + inpos] & 536870911) >>> (29 - 6))
| ((inArray[30 + inpos]) << 6);
  outArray[28 + outpos] = ((inArray[30 + inpos] & 536870911) >>> (29 - 3))
| ((inArray[31 + inpos]) << 3);
}

function fastpack3(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 7)
| ((inArray[1 + inpos] & 7) << 3)
| ((inArray[2 + inpos] & 7) << 6)
| ((inArray[3 + inpos] & 7) << 9)
| ((inArray[4 + inpos] & 7) << 12)
| ((inArray[5 + inpos] & 7) << 15)
| ((inArray[6 + inpos] & 7) << 18)
| ((inArray[7 + inpos] & 7) << 21)
| ((inArray[8 + inpos] & 7) << 24)
| ((inArray[9 + inpos] & 7) << 27)
| ((inArray[10 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[10 + inpos] & 7) >>> (3 - 1))
| ((inArray[11 + inpos] & 7) << 1)
| ((inArray[12 + inpos] & 7) << 4)
| ((inArray[13 + inpos] & 7) << 7)
| ((inArray[14 + inpos] & 7) << 10)
| ((inArray[15 + inpos] & 7) << 13)
| ((inArray[16 + inpos] & 7) << 16)
| ((inArray[17 + inpos] & 7) << 19)
| ((inArray[18 + inpos] & 7) << 22)
| ((inArray[19 + inpos] & 7) << 25)
| ((inArray[20 + inpos] & 7) << 28)
| ((inArray[21 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[21 + inpos] & 7) >>> (3 - 2))
| ((inArray[22 + inpos] & 7) << 2)
| ((inArray[23 + inpos] & 7) << 5)
| ((inArray[24 + inpos] & 7) << 8)
| ((inArray[25 + inpos] & 7) << 11)
| ((inArray[26 + inpos] & 7) << 14)
| ((inArray[27 + inpos] & 7) << 17)
| ((inArray[28 + inpos] & 7) << 20)
| ((inArray[29 + inpos] & 7) << 23)
| ((inArray[30 + inpos] & 7) << 26)
| ((inArray[31 + inpos]) << 29);
}

function fastpack30(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 1073741823)
| ((inArray[1 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 1073741823) >>> (30 - 28))
| ((inArray[2 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 1073741823) >>> (30 - 26))
| ((inArray[3 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 1073741823) >>> (30 - 24))
| ((inArray[4 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 1073741823) >>> (30 - 22))
| ((inArray[5 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[5 + inpos] & 1073741823) >>> (30 - 20))
| ((inArray[6 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[6 + inpos] & 1073741823) >>> (30 - 18))
| ((inArray[7 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[7 + inpos] & 1073741823) >>> (30 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[8 + inpos] & 1073741823) >>> (30 - 14))
| ((inArray[9 + inpos]) << 14);
  outArray[9 + outpos] = ((inArray[9 + inpos] & 1073741823) >>> (30 - 12))
| ((inArray[10 + inpos]) << 12);
  outArray[10 + outpos] = ((inArray[10 + inpos] & 1073741823) >>> (30 - 10))
| ((inArray[11 + inpos]) << 10);
  outArray[11 + outpos] = ((inArray[11 + inpos] & 1073741823) >>> (30 - 8))
| ((inArray[12 + inpos]) << 8);
  outArray[12 + outpos] = ((inArray[12 + inpos] & 1073741823) >>> (30 - 6))
| ((inArray[13 + inpos]) << 6);
  outArray[13 + outpos] = ((inArray[13 + inpos] & 1073741823) >>> (30 - 4))
| ((inArray[14 + inpos]) << 4);
  outArray[14 + outpos] = ((inArray[14 + inpos] & 1073741823) >>> (30 - 2))
| ((inArray[15 + inpos]) << 2);
  outArray[15 + outpos] = (inArray[16 + inpos] & 1073741823)
| ((inArray[17 + inpos]) << 30);
  outArray[16 + outpos] = ((inArray[17 + inpos] & 1073741823) >>> (30 - 28))
| ((inArray[18 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[18 + inpos] & 1073741823) >>> (30 - 26))
| ((inArray[19 + inpos]) << 26);
  outArray[18 + outpos] = ((inArray[19 + inpos] & 1073741823) >>> (30 - 24))
| ((inArray[20 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[20 + inpos] & 1073741823) >>> (30 - 22))
| ((inArray[21 + inpos]) << 22);
  outArray[20 + outpos] = ((inArray[21 + inpos] & 1073741823) >>> (30 - 20))
| ((inArray[22 + inpos]) << 20);
  outArray[21 + outpos] = ((inArray[22 + inpos] & 1073741823) >>> (30 - 18))
| ((inArray[23 + inpos]) << 18);
  outArray[22 + outpos] = ((inArray[23 + inpos] & 1073741823) >>> (30 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[23 + outpos] = ((inArray[24 + inpos] & 1073741823) >>> (30 - 14))
| ((inArray[25 + inpos]) << 14);
  outArray[24 + outpos] = ((inArray[25 + inpos] & 1073741823) >>> (30 - 12))
| ((inArray[26 + inpos]) << 12);
  outArray[25 + outpos] = ((inArray[26 + inpos] & 1073741823) >>> (30 - 10))
| ((inArray[27 + inpos]) << 10);
  outArray[26 + outpos] = ((inArray[27 + inpos] & 1073741823) >>> (30 - 8))
| ((inArray[28 + inpos]) << 8);
  outArray[27 + outpos] = ((inArray[28 + inpos] & 1073741823) >>> (30 - 6))
| ((inArray[29 + inpos]) << 6);
  outArray[28 + outpos] = ((inArray[29 + inpos] & 1073741823) >>> (30 - 4))
| ((inArray[30 + inpos]) << 4);
  outArray[29 + outpos] = ((inArray[30 + inpos] & 1073741823) >>> (30 - 2))
| ((inArray[31 + inpos]) << 2);
}

function fastpack31(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 2147483647)
| ((inArray[1 + inpos]) << 31);
  outArray[1 + outpos] = ((inArray[1 + inpos] & 2147483647) >>> (31 - 30))
| ((inArray[2 + inpos]) << 30);
  outArray[2 + outpos] = ((inArray[2 + inpos] & 2147483647) >>> (31 - 29))
| ((inArray[3 + inpos]) << 29);
  outArray[3 + outpos] = ((inArray[3 + inpos] & 2147483647) >>> (31 - 28))
| ((inArray[4 + inpos]) << 28);
  outArray[4 + outpos] = ((inArray[4 + inpos] & 2147483647) >>> (31 - 27))
| ((inArray[5 + inpos]) << 27);
  outArray[5 + outpos] = ((inArray[5 + inpos] & 2147483647) >>> (31 - 26))
| ((inArray[6 + inpos]) << 26);
  outArray[6 + outpos] = ((inArray[6 + inpos] & 2147483647) >>> (31 - 25))
| ((inArray[7 + inpos]) << 25);
  outArray[7 + outpos] = ((inArray[7 + inpos] & 2147483647) >>> (31 - 24))
| ((inArray[8 + inpos]) << 24);
  outArray[8 + outpos] = ((inArray[8 + inpos] & 2147483647) >>> (31 - 23))
| ((inArray[9 + inpos]) << 23);
  outArray[9 + outpos] = ((inArray[9 + inpos] & 2147483647) >>> (31 - 22))
| ((inArray[10 + inpos]) << 22);
  outArray[10 + outpos] = ((inArray[10 + inpos] & 2147483647) >>> (31 - 21))
| ((inArray[11 + inpos]) << 21);
  outArray[11 + outpos] = ((inArray[11 + inpos] & 2147483647) >>> (31 - 20))
| ((inArray[12 + inpos]) << 20);
  outArray[12 + outpos] = ((inArray[12 + inpos] & 2147483647) >>> (31 - 19))
| ((inArray[13 + inpos]) << 19);
  outArray[13 + outpos] = ((inArray[13 + inpos] & 2147483647) >>> (31 - 18))
| ((inArray[14 + inpos]) << 18);
  outArray[14 + outpos] = ((inArray[14 + inpos] & 2147483647) >>> (31 - 17))
| ((inArray[15 + inpos]) << 17);
  outArray[15 + outpos] = ((inArray[15 + inpos] & 2147483647) >>> (31 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[16 + outpos] = ((inArray[16 + inpos] & 2147483647) >>> (31 - 15))
| ((inArray[17 + inpos]) << 15);
  outArray[17 + outpos] = ((inArray[17 + inpos] & 2147483647) >>> (31 - 14))
| ((inArray[18 + inpos]) << 14);
  outArray[18 + outpos] = ((inArray[18 + inpos] & 2147483647) >>> (31 - 13))
| ((inArray[19 + inpos]) << 13);
  outArray[19 + outpos] = ((inArray[19 + inpos] & 2147483647) >>> (31 - 12))
| ((inArray[20 + inpos]) << 12);
  outArray[20 + outpos] = ((inArray[20 + inpos] & 2147483647) >>> (31 - 11))
| ((inArray[21 + inpos]) << 11);
  outArray[21 + outpos] = ((inArray[21 + inpos] & 2147483647) >>> (31 - 10))
| ((inArray[22 + inpos]) << 10);
  outArray[22 + outpos] = ((inArray[22 + inpos] & 2147483647) >>> (31 - 9))
| ((inArray[23 + inpos]) << 9);
  outArray[23 + outpos] = ((inArray[23 + inpos] & 2147483647) >>> (31 - 8))
| ((inArray[24 + inpos]) << 8);
  outArray[24 + outpos] = ((inArray[24 + inpos] & 2147483647) >>> (31 - 7))
| ((inArray[25 + inpos]) << 7);
  outArray[25 + outpos] = ((inArray[25 + inpos] & 2147483647) >>> (31 - 6))
| ((inArray[26 + inpos]) << 6);
  outArray[26 + outpos] = ((inArray[26 + inpos] & 2147483647) >>> (31 - 5))
| ((inArray[27 + inpos]) << 5);
  outArray[27 + outpos] = ((inArray[27 + inpos] & 2147483647) >>> (31 - 4))
| ((inArray[28 + inpos]) << 4);
  outArray[28 + outpos] = ((inArray[28 + inpos] & 2147483647) >>> (31 - 3))
| ((inArray[29 + inpos]) << 3);
  outArray[29 + outpos] = ((inArray[29 + inpos] & 2147483647) >>> (31 - 2))
| ((inArray[30 + inpos]) << 2);
  outArray[30 + outpos] = ((inArray[30 + inpos] & 2147483647) >>> (31 - 1))
| ((inArray[31 + inpos]) << 1);
}

function fastpack32(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  arraycopy(inArray, inpos, outArray, outpos, 32);
}

function fastpack4(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 15)
| ((inArray[1 + inpos] & 15) << 4)
| ((inArray[2 + inpos] & 15) << 8)
| ((inArray[3 + inpos] & 15) << 12)
| ((inArray[4 + inpos] & 15) << 16)
| ((inArray[5 + inpos] & 15) << 20)
| ((inArray[6 + inpos] & 15) << 24)
| ((inArray[7 + inpos]) << 28);
  outArray[1 + outpos] = (inArray[8 + inpos] & 15)
| ((inArray[9 + inpos] & 15) << 4)
| ((inArray[10 + inpos] & 15) << 8)
| ((inArray[11 + inpos] & 15) << 12)
| ((inArray[12 + inpos] & 15) << 16)
| ((inArray[13 + inpos] & 15) << 20)
| ((inArray[14 + inpos] & 15) << 24)
| ((inArray[15 + inpos]) << 28);
  outArray[2 + outpos] = (inArray[16 + inpos] & 15)
| ((inArray[17 + inpos] & 15) << 4)
| ((inArray[18 + inpos] & 15) << 8)
| ((inArray[19 + inpos] & 15) << 12)
| ((inArray[20 + inpos] & 15) << 16)
| ((inArray[21 + inpos] & 15) << 20)
| ((inArray[22 + inpos] & 15) << 24)
| ((inArray[23 + inpos]) << 28);
  outArray[3 + outpos] = (inArray[24 + inpos] & 15)
| ((inArray[25 + inpos] & 15) << 4)
| ((inArray[26 + inpos] & 15) << 8)
| ((inArray[27 + inpos] & 15) << 12)
| ((inArray[28 + inpos] & 15) << 16)
| ((inArray[29 + inpos] & 15) << 20)
| ((inArray[30 + inpos] & 15) << 24)
| ((inArray[31 + inpos]) << 28);
}

function fastpack5(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 31)
| ((inArray[1 + inpos] & 31) << 5)
| ((inArray[2 + inpos] & 31) << 10)
| ((inArray[3 + inpos] & 31) << 15)
| ((inArray[4 + inpos] & 31) << 20)
| ((inArray[5 + inpos] & 31) << 25)
| ((inArray[6 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[6 + inpos] & 31) >>> (5 - 3))
| ((inArray[7 + inpos] & 31) << 3)
| ((inArray[8 + inpos] & 31) << 8)
| ((inArray[9 + inpos] & 31) << 13)
| ((inArray[10 + inpos] & 31) << 18)
| ((inArray[11 + inpos] & 31) << 23)
| ((inArray[12 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[12 + inpos] & 31) >>> (5 - 1))
| ((inArray[13 + inpos] & 31) << 1)
| ((inArray[14 + inpos] & 31) << 6)
| ((inArray[15 + inpos] & 31) << 11)
| ((inArray[16 + inpos] & 31) << 16)
| ((inArray[17 + inpos] & 31) << 21)
| ((inArray[18 + inpos] & 31) << 26)
| ((inArray[19 + inpos]) << 31);
  outArray[3 + outpos] = ((inArray[19 + inpos] & 31) >>> (5 - 4))
| ((inArray[20 + inpos] & 31) << 4)
| ((inArray[21 + inpos] & 31) << 9)
| ((inArray[22 + inpos] & 31) << 14)
| ((inArray[23 + inpos] & 31) << 19)
| ((inArray[24 + inpos] & 31) << 24)
| ((inArray[25 + inpos]) << 29);
  outArray[4 + outpos] = ((inArray[25 + inpos] & 31) >>> (5 - 2))
| ((inArray[26 + inpos] & 31) << 2)
| ((inArray[27 + inpos] & 31) << 7)
| ((inArray[28 + inpos] & 31) << 12)
| ((inArray[29 + inpos] & 31) << 17)
| ((inArray[30 + inpos] & 31) << 22)
| ((inArray[31 + inpos]) << 27);
}

function fastpack6(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 63)
| ((inArray[1 + inpos] & 63) << 6)
| ((inArray[2 + inpos] & 63) << 12)
| ((inArray[3 + inpos] & 63) << 18)
| ((inArray[4 + inpos] & 63) << 24)
| ((inArray[5 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[5 + inpos] & 63) >>> (6 - 4))
| ((inArray[6 + inpos] & 63) << 4)
| ((inArray[7 + inpos] & 63) << 10)
| ((inArray[8 + inpos] & 63) << 16)
| ((inArray[9 + inpos] & 63) << 22)
| ((inArray[10 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[10 + inpos] & 63) >>> (6 - 2))
| ((inArray[11 + inpos] & 63) << 2)
| ((inArray[12 + inpos] & 63) << 8)
| ((inArray[13 + inpos] & 63) << 14)
| ((inArray[14 + inpos] & 63) << 20)
| ((inArray[15 + inpos]) << 26);
  outArray[3 + outpos] = (inArray[16 + inpos] & 63)
| ((inArray[17 + inpos] & 63) << 6)
| ((inArray[18 + inpos] & 63) << 12)
| ((inArray[19 + inpos] & 63) << 18)
| ((inArray[20 + inpos] & 63) << 24)
| ((inArray[21 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[21 + inpos] & 63) >>> (6 - 4))
| ((inArray[22 + inpos] & 63) << 4)
| ((inArray[23 + inpos] & 63) << 10)
| ((inArray[24 + inpos] & 63) << 16)
| ((inArray[25 + inpos] & 63) << 22)
| ((inArray[26 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[26 + inpos] & 63) >>> (6 - 2))
| ((inArray[27 + inpos] & 63) << 2)
| ((inArray[28 + inpos] & 63) << 8)
| ((inArray[29 + inpos] & 63) << 14)
| ((inArray[30 + inpos] & 63) << 20)
| ((inArray[31 + inpos]) << 26);
}

function fastpack7(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 127)
| ((inArray[1 + inpos] & 127) << 7)
| ((inArray[2 + inpos] & 127) << 14)
| ((inArray[3 + inpos] & 127) << 21)
| ((inArray[4 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[4 + inpos] & 127) >>> (7 - 3))
| ((inArray[5 + inpos] & 127) << 3)
| ((inArray[6 + inpos] & 127) << 10)
| ((inArray[7 + inpos] & 127) << 17)
| ((inArray[8 + inpos] & 127) << 24)
| ((inArray[9 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[9 + inpos] & 127) >>> (7 - 6))
| ((inArray[10 + inpos] & 127) << 6)
| ((inArray[11 + inpos] & 127) << 13)
| ((inArray[12 + inpos] & 127) << 20)
| ((inArray[13 + inpos]) << 27);
  outArray[3 + outpos] = ((inArray[13 + inpos] & 127) >>> (7 - 2))
| ((inArray[14 + inpos] & 127) << 2)
| ((inArray[15 + inpos] & 127) << 9)
| ((inArray[16 + inpos] & 127) << 16)
| ((inArray[17 + inpos] & 127) << 23)
| ((inArray[18 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[18 + inpos] & 127) >>> (7 - 5))
| ((inArray[19 + inpos] & 127) << 5)
| ((inArray[20 + inpos] & 127) << 12)
| ((inArray[21 + inpos] & 127) << 19)
| ((inArray[22 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[22 + inpos] & 127) >>> (7 - 1))
| ((inArray[23 + inpos] & 127) << 1)
| ((inArray[24 + inpos] & 127) << 8)
| ((inArray[25 + inpos] & 127) << 15)
| ((inArray[26 + inpos] & 127) << 22)
| ((inArray[27 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[27 + inpos] & 127) >>> (7 - 4))
| ((inArray[28 + inpos] & 127) << 4)
| ((inArray[29 + inpos] & 127) << 11)
| ((inArray[30 + inpos] & 127) << 18)
| ((inArray[31 + inpos]) << 25);
}

function fastpack8(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 255)
| ((inArray[1 + inpos] & 255) << 8)
| ((inArray[2 + inpos] & 255) << 16)
| ((inArray[3 + inpos]) << 24);
  outArray[1 + outpos] = (inArray[4 + inpos] & 255)
| ((inArray[5 + inpos] & 255) << 8)
| ((inArray[6 + inpos] & 255) << 16)
| ((inArray[7 + inpos]) << 24);
  outArray[2 + outpos] = (inArray[8 + inpos] & 255)
| ((inArray[9 + inpos] & 255) << 8)
| ((inArray[10 + inpos] & 255) << 16)
| ((inArray[11 + inpos]) << 24);
  outArray[3 + outpos] = (inArray[12 + inpos] & 255)
| ((inArray[13 + inpos] & 255) << 8)
| ((inArray[14 + inpos] & 255) << 16)
| ((inArray[15 + inpos]) << 24);
  outArray[4 + outpos] = (inArray[16 + inpos] & 255)
| ((inArray[17 + inpos] & 255) << 8)
| ((inArray[18 + inpos] & 255) << 16)
| ((inArray[19 + inpos]) << 24);
  outArray[5 + outpos] = (inArray[20 + inpos] & 255)
| ((inArray[21 + inpos] & 255) << 8)
| ((inArray[22 + inpos] & 255) << 16)
| ((inArray[23 + inpos]) << 24);
  outArray[6 + outpos] = (inArray[24 + inpos] & 255)
| ((inArray[25 + inpos] & 255) << 8)
| ((inArray[26 + inpos] & 255) << 16)
| ((inArray[27 + inpos]) << 24);
  outArray[7 + outpos] = (inArray[28 + inpos] & 255)
| ((inArray[29 + inpos] & 255) << 8)
| ((inArray[30 + inpos] & 255) << 16)
| ((inArray[31 + inpos]) << 24);
}

function fastpack9(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = (inArray[inpos] & 511)
| ((inArray[1 + inpos] & 511) << 9)
| ((inArray[2 + inpos] & 511) << 18)
| ((inArray[3 + inpos]) << 27);
  outArray[1 + outpos] = ((inArray[3 + inpos] & 511) >>> (9 - 4))
| ((inArray[4 + inpos] & 511) << 4)
| ((inArray[5 + inpos] & 511) << 13)
| ((inArray[6 + inpos] & 511) << 22)
| ((inArray[7 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[7 + inpos] & 511) >>> (9 - 8))
| ((inArray[8 + inpos] & 511) << 8)
| ((inArray[9 + inpos] & 511) << 17)
| ((inArray[10 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[10 + inpos] & 511) >>> (9 - 3))
| ((inArray[11 + inpos] & 511) << 3)
| ((inArray[12 + inpos] & 511) << 12)
| ((inArray[13 + inpos] & 511) << 21)
| ((inArray[14 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[14 + inpos] & 511) >>> (9 - 7))
| ((inArray[15 + inpos] & 511) << 7)
| ((inArray[16 + inpos] & 511) << 16)
| ((inArray[17 + inpos]) << 25);
  outArray[5 + outpos] = ((inArray[17 + inpos] & 511) >>> (9 - 2))
| ((inArray[18 + inpos] & 511) << 2)
| ((inArray[19 + inpos] & 511) << 11)
| ((inArray[20 + inpos] & 511) << 20)
| ((inArray[21 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[21 + inpos] & 511) >>> (9 - 6))
| ((inArray[22 + inpos] & 511) << 6)
| ((inArray[23 + inpos] & 511) << 15)
| ((inArray[24 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[24 + inpos] & 511) >>> (9 - 1))
| ((inArray[25 + inpos] & 511) << 1)
| ((inArray[26 + inpos] & 511) << 10)
| ((inArray[27 + inpos] & 511) << 19)
| ((inArray[28 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[28 + inpos] & 511) >>> (9 - 5))
| ((inArray[29 + inpos] & 511) << 5)
| ((inArray[30 + inpos] & 511) << 14)
| ((inArray[31 + inpos]) << 23);
}

/**
 * Unpack 32 numberegers
 *
 * @param inArray
 *                source array
 * @param inpos
 *                position in source array
 * @param outArray
 *                output array
 * @param outpos
 *                position in output array
 * @param bit
 *                number of bits to use per numbereger
 */
export function fastpackwithoutmask(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number, bit: number) {
  switch (bit) {
    case 0:
      fastpackwithoutmask0(inArray, inpos, outArray, outpos);
      break;
    case 1:
      fastpackwithoutmask1(inArray, inpos, outArray, outpos);
      break;
    case 2:
      fastpackwithoutmask2(inArray, inpos, outArray, outpos);
      break;
    case 3:
      fastpackwithoutmask3(inArray, inpos, outArray, outpos);
      break;
    case 4:
      fastpackwithoutmask4(inArray, inpos, outArray, outpos);
      break;
    case 5:
      fastpackwithoutmask5(inArray, inpos, outArray, outpos);
      break;
    case 6:
      fastpackwithoutmask6(inArray, inpos, outArray, outpos);
      break;
    case 7:
      fastpackwithoutmask7(inArray, inpos, outArray, outpos);
      break;
    case 8:
      fastpackwithoutmask8(inArray, inpos, outArray, outpos);
      break;
    case 9:
      fastpackwithoutmask9(inArray, inpos, outArray, outpos);
      break;
    case 10:
      fastpackwithoutmask10(inArray, inpos, outArray, outpos);
      break;
    case 11:
      fastpackwithoutmask11(inArray, inpos, outArray, outpos);
      break;
    case 12:
      fastpackwithoutmask12(inArray, inpos, outArray, outpos);
      break;
    case 13:
      fastpackwithoutmask13(inArray, inpos, outArray, outpos);
      break;
    case 14:
      fastpackwithoutmask14(inArray, inpos, outArray, outpos);
      break;
    case 15:
      fastpackwithoutmask15(inArray, inpos, outArray, outpos);
      break;
    case 16:
      fastpackwithoutmask16(inArray, inpos, outArray, outpos);
      break;
    case 17:
      fastpackwithoutmask17(inArray, inpos, outArray, outpos);
      break;
    case 18:
      fastpackwithoutmask18(inArray, inpos, outArray, outpos);
      break;
    case 19:
      fastpackwithoutmask19(inArray, inpos, outArray, outpos);
      break;
    case 20:
      fastpackwithoutmask20(inArray, inpos, outArray, outpos);
      break;
    case 21:
      fastpackwithoutmask21(inArray, inpos, outArray, outpos);
      break;
    case 22:
      fastpackwithoutmask22(inArray, inpos, outArray, outpos);
      break;
    case 23:
      fastpackwithoutmask23(inArray, inpos, outArray, outpos);
      break;
    case 24:
      fastpackwithoutmask24(inArray, inpos, outArray, outpos);
      break;
    case 25:
      fastpackwithoutmask25(inArray, inpos, outArray, outpos);
      break;
    case 26:
      fastpackwithoutmask26(inArray, inpos, outArray, outpos);
      break;
    case 27:
      fastpackwithoutmask27(inArray, inpos, outArray, outpos);
      break;
    case 28:
      fastpackwithoutmask28(inArray, inpos, outArray, outpos);
      break;
    case 29:
      fastpackwithoutmask29(inArray, inpos, outArray, outpos);
      break;
    case 30:
      fastpackwithoutmask30(inArray, inpos, outArray, outpos);
      break;
    case 31:
      fastpackwithoutmask31(inArray, inpos, outArray, outpos);
      break;
    case 32:
      fastpackwithoutmask32(inArray, inpos, outArray, outpos);
      break;
    default:
      throw new Error("Unsupported bit width.");
  }
}

function fastpackwithoutmask0(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  // nothing
}

function fastpackwithoutmask1(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 1)
| ((inArray[2 + inpos]) << 2) | ((inArray[3 + inpos]) << 3)
| ((inArray[4 + inpos]) << 4) | ((inArray[5 + inpos]) << 5)
| ((inArray[6 + inpos]) << 6) | ((inArray[7 + inpos]) << 7)
| ((inArray[8 + inpos]) << 8) | ((inArray[9 + inpos]) << 9)
| ((inArray[10 + inpos]) << 10) | ((inArray[11 + inpos]) << 11)
| ((inArray[12 + inpos]) << 12) | ((inArray[13 + inpos]) << 13)
| ((inArray[14 + inpos]) << 14) | ((inArray[15 + inpos]) << 15)
| ((inArray[16 + inpos]) << 16) | ((inArray[17 + inpos]) << 17)
| ((inArray[18 + inpos]) << 18) | ((inArray[19 + inpos]) << 19)
| ((inArray[20 + inpos]) << 20) | ((inArray[21 + inpos]) << 21)
| ((inArray[22 + inpos]) << 22) | ((inArray[23 + inpos]) << 23)
| ((inArray[24 + inpos]) << 24) | ((inArray[25 + inpos]) << 25)
| ((inArray[26 + inpos]) << 26) | ((inArray[27 + inpos]) << 27)
| ((inArray[28 + inpos]) << 28) | ((inArray[29 + inpos]) << 29)
| ((inArray[30 + inpos]) << 30) | ((inArray[31 + inpos]) << 31);
}

function fastpackwithoutmask10(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 10)
| ((inArray[2 + inpos]) << 20) | ((inArray[3 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[3 + inpos]) >>> (10 - 8))
| ((inArray[4 + inpos]) << 8) | ((inArray[5 + inpos]) << 18)
| ((inArray[6 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[6 + inpos]) >>> (10 - 6))
| ((inArray[7 + inpos]) << 6) | ((inArray[8 + inpos]) << 16)
| ((inArray[9 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[9 + inpos]) >>> (10 - 4))
| ((inArray[10 + inpos]) << 4) | ((inArray[11 + inpos]) << 14)
| ((inArray[12 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[12 + inpos]) >>> (10 - 2))
| ((inArray[13 + inpos]) << 2) | ((inArray[14 + inpos]) << 12)
| ((inArray[15 + inpos]) << 22);
  outArray[5 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 10)
| ((inArray[18 + inpos]) << 20) | ((inArray[19 + inpos]) << 30);
  outArray[6 + outpos] = ((inArray[19 + inpos]) >>> (10 - 8))
| ((inArray[20 + inpos]) << 8) | ((inArray[21 + inpos]) << 18)
| ((inArray[22 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[22 + inpos]) >>> (10 - 6))
| ((inArray[23 + inpos]) << 6) | ((inArray[24 + inpos]) << 16)
| ((inArray[25 + inpos]) << 26);
  outArray[8 + outpos] = ((inArray[25 + inpos]) >>> (10 - 4))
| ((inArray[26 + inpos]) << 4) | ((inArray[27 + inpos]) << 14)
| ((inArray[28 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[28 + inpos]) >>> (10 - 2))
| ((inArray[29 + inpos]) << 2) | ((inArray[30 + inpos]) << 12)
| ((inArray[31 + inpos]) << 22);
}

function fastpackwithoutmask11(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 11)
| ((inArray[2 + inpos]) << 22);
  outArray[1 + outpos] = ((inArray[2 + inpos]) >>> (11 - 1))
| ((inArray[3 + inpos]) << 1) | ((inArray[4 + inpos]) << 12)
| ((inArray[5 + inpos]) << 23);
  outArray[2 + outpos] = ((inArray[5 + inpos]) >>> (11 - 2))
| ((inArray[6 + inpos]) << 2) | ((inArray[7 + inpos]) << 13)
| ((inArray[8 + inpos]) << 24);
  outArray[3 + outpos] = ((inArray[8 + inpos]) >>> (11 - 3))
| ((inArray[9 + inpos]) << 3) | ((inArray[10 + inpos]) << 14)
| ((inArray[11 + inpos]) << 25);
  outArray[4 + outpos] = ((inArray[11 + inpos]) >>> (11 - 4))
| ((inArray[12 + inpos]) << 4) | ((inArray[13 + inpos]) << 15)
| ((inArray[14 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[14 + inpos]) >>> (11 - 5))
| ((inArray[15 + inpos]) << 5) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 27);
  outArray[6 + outpos] = ((inArray[17 + inpos]) >>> (11 - 6))
| ((inArray[18 + inpos]) << 6) | ((inArray[19 + inpos]) << 17)
| ((inArray[20 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[20 + inpos]) >>> (11 - 7))
| ((inArray[21 + inpos]) << 7) | ((inArray[22 + inpos]) << 18)
| ((inArray[23 + inpos]) << 29);
  outArray[8 + outpos] = ((inArray[23 + inpos]) >>> (11 - 8))
| ((inArray[24 + inpos]) << 8) | ((inArray[25 + inpos]) << 19)
| ((inArray[26 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[26 + inpos]) >>> (11 - 9))
| ((inArray[27 + inpos]) << 9) | ((inArray[28 + inpos]) << 20)
| ((inArray[29 + inpos]) << 31);
  outArray[10 + outpos] = ((inArray[29 + inpos]) >>> (11 - 10))
| ((inArray[30 + inpos]) << 10) | ((inArray[31 + inpos]) << 21);
}

function fastpackwithoutmask12(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 12)
| ((inArray[2 + inpos]) << 24);
  outArray[1 + outpos] = ((inArray[2 + inpos]) >>> (12 - 4))
| ((inArray[3 + inpos]) << 4) | ((inArray[4 + inpos]) << 16)
| ((inArray[5 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[5 + inpos]) >>> (12 - 8))
| ((inArray[6 + inpos]) << 8) | ((inArray[7 + inpos]) << 20);
  outArray[3 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 12)
| ((inArray[10 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[10 + inpos]) >>> (12 - 4))
| ((inArray[11 + inpos]) << 4) | ((inArray[12 + inpos]) << 16)
| ((inArray[13 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[13 + inpos]) >>> (12 - 8))
| ((inArray[14 + inpos]) << 8) | ((inArray[15 + inpos]) << 20);
  outArray[6 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 12)
| ((inArray[18 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[18 + inpos]) >>> (12 - 4))
| ((inArray[19 + inpos]) << 4) | ((inArray[20 + inpos]) << 16)
| ((inArray[21 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[21 + inpos]) >>> (12 - 8))
| ((inArray[22 + inpos]) << 8) | ((inArray[23 + inpos]) << 20);
  outArray[9 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 12)
| ((inArray[26 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[26 + inpos]) >>> (12 - 4))
| ((inArray[27 + inpos]) << 4) | ((inArray[28 + inpos]) << 16)
| ((inArray[29 + inpos]) << 28);
  outArray[11 + outpos] = ((inArray[29 + inpos]) >>> (12 - 8))
| ((inArray[30 + inpos]) << 8) | ((inArray[31 + inpos]) << 20);
}

function fastpackwithoutmask13(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 13)
| ((inArray[2 + inpos]) << 26);
  outArray[1 + outpos] = ((inArray[2 + inpos]) >>> (13 - 7))
| ((inArray[3 + inpos]) << 7) | ((inArray[4 + inpos]) << 20);
  outArray[2 + outpos] = ((inArray[4 + inpos]) >>> (13 - 1))
| ((inArray[5 + inpos]) << 1) | ((inArray[6 + inpos]) << 14)
| ((inArray[7 + inpos]) << 27);
  outArray[3 + outpos] = ((inArray[7 + inpos]) >>> (13 - 8))
| ((inArray[8 + inpos]) << 8) | ((inArray[9 + inpos]) << 21);
  outArray[4 + outpos] = ((inArray[9 + inpos]) >>> (13 - 2))
| ((inArray[10 + inpos]) << 2) | ((inArray[11 + inpos]) << 15)
| ((inArray[12 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[12 + inpos]) >>> (13 - 9))
| ((inArray[13 + inpos]) << 9) | ((inArray[14 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[14 + inpos]) >>> (13 - 3))
| ((inArray[15 + inpos]) << 3) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 29);
  outArray[7 + outpos] = ((inArray[17 + inpos]) >>> (13 - 10))
| ((inArray[18 + inpos]) << 10) | ((inArray[19 + inpos]) << 23);
  outArray[8 + outpos] = ((inArray[19 + inpos]) >>> (13 - 4))
| ((inArray[20 + inpos]) << 4) | ((inArray[21 + inpos]) << 17)
| ((inArray[22 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[22 + inpos]) >>> (13 - 11))
| ((inArray[23 + inpos]) << 11) | ((inArray[24 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[24 + inpos]) >>> (13 - 5))
| ((inArray[25 + inpos]) << 5) | ((inArray[26 + inpos]) << 18)
| ((inArray[27 + inpos]) << 31);
  outArray[11 + outpos] = ((inArray[27 + inpos]) >>> (13 - 12))
| ((inArray[28 + inpos]) << 12) | ((inArray[29 + inpos]) << 25);
  outArray[12 + outpos] = ((inArray[29 + inpos]) >>> (13 - 6))
| ((inArray[30 + inpos]) << 6) | ((inArray[31 + inpos]) << 19);
}

function fastpackwithoutmask14(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 14)
| ((inArray[2 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[2 + inpos]) >>> (14 - 10))
| ((inArray[3 + inpos]) << 10) | ((inArray[4 + inpos]) << 24);
  outArray[2 + outpos] = ((inArray[4 + inpos]) >>> (14 - 6))
| ((inArray[5 + inpos]) << 6) | ((inArray[6 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[6 + inpos]) >>> (14 - 2))
| ((inArray[7 + inpos]) << 2) | ((inArray[8 + inpos]) << 16)
| ((inArray[9 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[9 + inpos]) >>> (14 - 12))
| ((inArray[10 + inpos]) << 12) | ((inArray[11 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[11 + inpos]) >>> (14 - 8))
| ((inArray[12 + inpos]) << 8) | ((inArray[13 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[13 + inpos]) >>> (14 - 4))
| ((inArray[14 + inpos]) << 4) | ((inArray[15 + inpos]) << 18);
  outArray[7 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 14)
| ((inArray[18 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[18 + inpos]) >>> (14 - 10))
| ((inArray[19 + inpos]) << 10) | ((inArray[20 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[20 + inpos]) >>> (14 - 6))
| ((inArray[21 + inpos]) << 6) | ((inArray[22 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[22 + inpos]) >>> (14 - 2))
| ((inArray[23 + inpos]) << 2) | ((inArray[24 + inpos]) << 16)
| ((inArray[25 + inpos]) << 30);
  outArray[11 + outpos] = ((inArray[25 + inpos]) >>> (14 - 12))
| ((inArray[26 + inpos]) << 12) | ((inArray[27 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[27 + inpos]) >>> (14 - 8))
| ((inArray[28 + inpos]) << 8) | ((inArray[29 + inpos]) << 22);
  outArray[13 + outpos] = ((inArray[29 + inpos]) >>> (14 - 4))
| ((inArray[30 + inpos]) << 4) | ((inArray[31 + inpos]) << 18);
}

function fastpackwithoutmask15(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 15)
| ((inArray[2 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[2 + inpos]) >>> (15 - 13))
| ((inArray[3 + inpos]) << 13) | ((inArray[4 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[4 + inpos]) >>> (15 - 11))
| ((inArray[5 + inpos]) << 11) | ((inArray[6 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[6 + inpos]) >>> (15 - 9))
| ((inArray[7 + inpos]) << 9) | ((inArray[8 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[8 + inpos]) >>> (15 - 7))
| ((inArray[9 + inpos]) << 7) | ((inArray[10 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[10 + inpos]) >>> (15 - 5))
| ((inArray[11 + inpos]) << 5) | ((inArray[12 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[12 + inpos]) >>> (15 - 3))
| ((inArray[13 + inpos]) << 3) | ((inArray[14 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[14 + inpos]) >>> (15 - 1))
| ((inArray[15 + inpos]) << 1) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 31);
  outArray[8 + outpos] = ((inArray[17 + inpos]) >>> (15 - 14))
| ((inArray[18 + inpos]) << 14) | ((inArray[19 + inpos]) << 29);
  outArray[9 + outpos] = ((inArray[19 + inpos]) >>> (15 - 12))
| ((inArray[20 + inpos]) << 12) | ((inArray[21 + inpos]) << 27);
  outArray[10 + outpos] = ((inArray[21 + inpos]) >>> (15 - 10))
| ((inArray[22 + inpos]) << 10) | ((inArray[23 + inpos]) << 25);
  outArray[11 + outpos] = ((inArray[23 + inpos]) >>> (15 - 8))
| ((inArray[24 + inpos]) << 8) | ((inArray[25 + inpos]) << 23);
  outArray[12 + outpos] = ((inArray[25 + inpos]) >>> (15 - 6))
| ((inArray[26 + inpos]) << 6) | ((inArray[27 + inpos]) << 21);
  outArray[13 + outpos] = ((inArray[27 + inpos]) >>> (15 - 4))
| ((inArray[28 + inpos]) << 4) | ((inArray[29 + inpos]) << 19);
  outArray[14 + outpos] = ((inArray[29 + inpos]) >>> (15 - 2))
| ((inArray[30 + inpos]) << 2) | ((inArray[31 + inpos]) << 17);
}

function fastpackwithoutmask16(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 16);
  outArray[1 + outpos] = inArray[2 + inpos] | ((inArray[3 + inpos]) << 16);
  outArray[2 + outpos] = inArray[4 + inpos] | ((inArray[5 + inpos]) << 16);
  outArray[3 + outpos] = inArray[6 + inpos] | ((inArray[7 + inpos]) << 16);
  outArray[4 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 16);
  outArray[5 + outpos] = inArray[10 + inpos] | ((inArray[11 + inpos]) << 16);
  outArray[6 + outpos] = inArray[12 + inpos] | ((inArray[13 + inpos]) << 16);
  outArray[7 + outpos] = inArray[14 + inpos] | ((inArray[15 + inpos]) << 16);
  outArray[8 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 16);
  outArray[9 + outpos] = inArray[18 + inpos] | ((inArray[19 + inpos]) << 16);
  outArray[10 + outpos] = inArray[20 + inpos] | ((inArray[21 + inpos]) << 16);
  outArray[11 + outpos] = inArray[22 + inpos] | ((inArray[23 + inpos]) << 16);
  outArray[12 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 16);
  outArray[13 + outpos] = inArray[26 + inpos] | ((inArray[27 + inpos]) << 16);
  outArray[14 + outpos] = inArray[28 + inpos] | ((inArray[29 + inpos]) << 16);
  outArray[15 + outpos] = inArray[30 + inpos] | ((inArray[31 + inpos]) << 16);
}

function fastpackwithoutmask17(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 17);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (17 - 2))
| ((inArray[2 + inpos]) << 2) | ((inArray[3 + inpos]) << 19);
  outArray[2 + outpos] = ((inArray[3 + inpos]) >>> (17 - 4))
| ((inArray[4 + inpos]) << 4) | ((inArray[5 + inpos]) << 21);
  outArray[3 + outpos] = ((inArray[5 + inpos]) >>> (17 - 6))
| ((inArray[6 + inpos]) << 6) | ((inArray[7 + inpos]) << 23);
  outArray[4 + outpos] = ((inArray[7 + inpos]) >>> (17 - 8))
| ((inArray[8 + inpos]) << 8) | ((inArray[9 + inpos]) << 25);
  outArray[5 + outpos] = ((inArray[9 + inpos]) >>> (17 - 10))
| ((inArray[10 + inpos]) << 10) | ((inArray[11 + inpos]) << 27);
  outArray[6 + outpos] = ((inArray[11 + inpos]) >>> (17 - 12))
| ((inArray[12 + inpos]) << 12) | ((inArray[13 + inpos]) << 29);
  outArray[7 + outpos] = ((inArray[13 + inpos]) >>> (17 - 14))
| ((inArray[14 + inpos]) << 14) | ((inArray[15 + inpos]) << 31);
  outArray[8 + outpos] = ((inArray[15 + inpos]) >>> (17 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[9 + outpos] = ((inArray[16 + inpos]) >>> (17 - 1))
| ((inArray[17 + inpos]) << 1) | ((inArray[18 + inpos]) << 18);
  outArray[10 + outpos] = ((inArray[18 + inpos]) >>> (17 - 3))
| ((inArray[19 + inpos]) << 3) | ((inArray[20 + inpos]) << 20);
  outArray[11 + outpos] = ((inArray[20 + inpos]) >>> (17 - 5))
| ((inArray[21 + inpos]) << 5) | ((inArray[22 + inpos]) << 22);
  outArray[12 + outpos] = ((inArray[22 + inpos]) >>> (17 - 7))
| ((inArray[23 + inpos]) << 7) | ((inArray[24 + inpos]) << 24);
  outArray[13 + outpos] = ((inArray[24 + inpos]) >>> (17 - 9))
| ((inArray[25 + inpos]) << 9) | ((inArray[26 + inpos]) << 26);
  outArray[14 + outpos] = ((inArray[26 + inpos]) >>> (17 - 11))
| ((inArray[27 + inpos]) << 11) | ((inArray[28 + inpos]) << 28);
  outArray[15 + outpos] = ((inArray[28 + inpos]) >>> (17 - 13))
| ((inArray[29 + inpos]) << 13) | ((inArray[30 + inpos]) << 30);
  outArray[16 + outpos] = ((inArray[30 + inpos]) >>> (17 - 15))
| ((inArray[31 + inpos]) << 15);
}

function fastpackwithoutmask18(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 18);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (18 - 4))
| ((inArray[2 + inpos]) << 4) | ((inArray[3 + inpos]) << 22);
  outArray[2 + outpos] = ((inArray[3 + inpos]) >>> (18 - 8))
| ((inArray[4 + inpos]) << 8) | ((inArray[5 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[5 + inpos]) >>> (18 - 12))
| ((inArray[6 + inpos]) << 12) | ((inArray[7 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[7 + inpos]) >>> (18 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[5 + outpos] = ((inArray[8 + inpos]) >>> (18 - 2))
| ((inArray[9 + inpos]) << 2) | ((inArray[10 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[10 + inpos]) >>> (18 - 6))
| ((inArray[11 + inpos]) << 6) | ((inArray[12 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[12 + inpos]) >>> (18 - 10))
| ((inArray[13 + inpos]) << 10) | ((inArray[14 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[14 + inpos]) >>> (18 - 14))
| ((inArray[15 + inpos]) << 14);
  outArray[9 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 18);
  outArray[10 + outpos] = ((inArray[17 + inpos]) >>> (18 - 4))
| ((inArray[18 + inpos]) << 4) | ((inArray[19 + inpos]) << 22);
  outArray[11 + outpos] = ((inArray[19 + inpos]) >>> (18 - 8))
| ((inArray[20 + inpos]) << 8) | ((inArray[21 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[21 + inpos]) >>> (18 - 12))
| ((inArray[22 + inpos]) << 12) | ((inArray[23 + inpos]) << 30);
  outArray[13 + outpos] = ((inArray[23 + inpos]) >>> (18 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[24 + inpos]) >>> (18 - 2))
| ((inArray[25 + inpos]) << 2) | ((inArray[26 + inpos]) << 20);
  outArray[15 + outpos] = ((inArray[26 + inpos]) >>> (18 - 6))
| ((inArray[27 + inpos]) << 6) | ((inArray[28 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[28 + inpos]) >>> (18 - 10))
| ((inArray[29 + inpos]) << 10) | ((inArray[30 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[30 + inpos]) >>> (18 - 14))
| ((inArray[31 + inpos]) << 14);
}

function fastpackwithoutmask19(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 19);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (19 - 6))
| ((inArray[2 + inpos]) << 6) | ((inArray[3 + inpos]) << 25);
  outArray[2 + outpos] = ((inArray[3 + inpos]) >>> (19 - 12))
| ((inArray[4 + inpos]) << 12) | ((inArray[5 + inpos]) << 31);
  outArray[3 + outpos] = ((inArray[5 + inpos]) >>> (19 - 18))
| ((inArray[6 + inpos]) << 18);
  outArray[4 + outpos] = ((inArray[6 + inpos]) >>> (19 - 5))
| ((inArray[7 + inpos]) << 5) | ((inArray[8 + inpos]) << 24);
  outArray[5 + outpos] = ((inArray[8 + inpos]) >>> (19 - 11))
| ((inArray[9 + inpos]) << 11) | ((inArray[10 + inpos]) << 30);
  outArray[6 + outpos] = ((inArray[10 + inpos]) >>> (19 - 17))
| ((inArray[11 + inpos]) << 17);
  outArray[7 + outpos] = ((inArray[11 + inpos]) >>> (19 - 4))
| ((inArray[12 + inpos]) << 4) | ((inArray[13 + inpos]) << 23);
  outArray[8 + outpos] = ((inArray[13 + inpos]) >>> (19 - 10))
| ((inArray[14 + inpos]) << 10) | ((inArray[15 + inpos]) << 29);
  outArray[9 + outpos] = ((inArray[15 + inpos]) >>> (19 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[10 + outpos] = ((inArray[16 + inpos]) >>> (19 - 3))
| ((inArray[17 + inpos]) << 3) | ((inArray[18 + inpos]) << 22);
  outArray[11 + outpos] = ((inArray[18 + inpos]) >>> (19 - 9))
| ((inArray[19 + inpos]) << 9) | ((inArray[20 + inpos]) << 28);
  outArray[12 + outpos] = ((inArray[20 + inpos]) >>> (19 - 15))
| ((inArray[21 + inpos]) << 15);
  outArray[13 + outpos] = ((inArray[21 + inpos]) >>> (19 - 2))
| ((inArray[22 + inpos]) << 2) | ((inArray[23 + inpos]) << 21);
  outArray[14 + outpos] = ((inArray[23 + inpos]) >>> (19 - 8))
| ((inArray[24 + inpos]) << 8) | ((inArray[25 + inpos]) << 27);
  outArray[15 + outpos] = ((inArray[25 + inpos]) >>> (19 - 14))
| ((inArray[26 + inpos]) << 14);
  outArray[16 + outpos] = ((inArray[26 + inpos]) >>> (19 - 1))
| ((inArray[27 + inpos]) << 1) | ((inArray[28 + inpos]) << 20);
  outArray[17 + outpos] = ((inArray[28 + inpos]) >>> (19 - 7))
| ((inArray[29 + inpos]) << 7) | ((inArray[30 + inpos]) << 26);
  outArray[18 + outpos] = ((inArray[30 + inpos]) >>> (19 - 13))
| ((inArray[31 + inpos]) << 13);
}

function fastpackwithoutmask2(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 2)
| ((inArray[2 + inpos]) << 4) | ((inArray[3 + inpos]) << 6)
| ((inArray[4 + inpos]) << 8) | ((inArray[5 + inpos]) << 10)
| ((inArray[6 + inpos]) << 12) | ((inArray[7 + inpos]) << 14)
| ((inArray[8 + inpos]) << 16) | ((inArray[9 + inpos]) << 18)
| ((inArray[10 + inpos]) << 20) | ((inArray[11 + inpos]) << 22)
| ((inArray[12 + inpos]) << 24) | ((inArray[13 + inpos]) << 26)
| ((inArray[14 + inpos]) << 28) | ((inArray[15 + inpos]) << 30);
  outArray[1 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 2)
| ((inArray[18 + inpos]) << 4) | ((inArray[19 + inpos]) << 6)
| ((inArray[20 + inpos]) << 8) | ((inArray[21 + inpos]) << 10)
| ((inArray[22 + inpos]) << 12) | ((inArray[23 + inpos]) << 14)
| ((inArray[24 + inpos]) << 16) | ((inArray[25 + inpos]) << 18)
| ((inArray[26 + inpos]) << 20) | ((inArray[27 + inpos]) << 22)
| ((inArray[28 + inpos]) << 24) | ((inArray[29 + inpos]) << 26)
| ((inArray[30 + inpos]) << 28) | ((inArray[31 + inpos]) << 30);
}

function fastpackwithoutmask20(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 20);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (20 - 8))
| ((inArray[2 + inpos]) << 8) | ((inArray[3 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[3 + inpos]) >>> (20 - 16))
| ((inArray[4 + inpos]) << 16);
  outArray[3 + outpos] = ((inArray[4 + inpos]) >>> (20 - 4))
| ((inArray[5 + inpos]) << 4) | ((inArray[6 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[6 + inpos]) >>> (20 - 12))
| ((inArray[7 + inpos]) << 12);
  outArray[5 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[9 + inpos]) >>> (20 - 8))
| ((inArray[10 + inpos]) << 8) | ((inArray[11 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[11 + inpos]) >>> (20 - 16))
| ((inArray[12 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[12 + inpos]) >>> (20 - 4))
| ((inArray[13 + inpos]) << 4) | ((inArray[14 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[14 + inpos]) >>> (20 - 12))
| ((inArray[15 + inpos]) << 12);
  outArray[10 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 20);
  outArray[11 + outpos] = ((inArray[17 + inpos]) >>> (20 - 8))
| ((inArray[18 + inpos]) << 8) | ((inArray[19 + inpos]) << 28);
  outArray[12 + outpos] = ((inArray[19 + inpos]) >>> (20 - 16))
| ((inArray[20 + inpos]) << 16);
  outArray[13 + outpos] = ((inArray[20 + inpos]) >>> (20 - 4))
| ((inArray[21 + inpos]) << 4) | ((inArray[22 + inpos]) << 24);
  outArray[14 + outpos] = ((inArray[22 + inpos]) >>> (20 - 12))
| ((inArray[23 + inpos]) << 12);
  outArray[15 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 20);
  outArray[16 + outpos] = ((inArray[25 + inpos]) >>> (20 - 8))
| ((inArray[26 + inpos]) << 8) | ((inArray[27 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[27 + inpos]) >>> (20 - 16))
| ((inArray[28 + inpos]) << 16);
  outArray[18 + outpos] = ((inArray[28 + inpos]) >>> (20 - 4))
| ((inArray[29 + inpos]) << 4) | ((inArray[30 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[30 + inpos]) >>> (20 - 12))
| ((inArray[31 + inpos]) << 12);
}

function fastpackwithoutmask21(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 21);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (21 - 10))
| ((inArray[2 + inpos]) << 10) | ((inArray[3 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[3 + inpos]) >>> (21 - 20))
| ((inArray[4 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[4 + inpos]) >>> (21 - 9))
| ((inArray[5 + inpos]) << 9) | ((inArray[6 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[6 + inpos]) >>> (21 - 19))
| ((inArray[7 + inpos]) << 19);
  outArray[5 + outpos] = ((inArray[7 + inpos]) >>> (21 - 8))
| ((inArray[8 + inpos]) << 8) | ((inArray[9 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[9 + inpos]) >>> (21 - 18))
| ((inArray[10 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[10 + inpos]) >>> (21 - 7))
| ((inArray[11 + inpos]) << 7) | ((inArray[12 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[12 + inpos]) >>> (21 - 17))
| ((inArray[13 + inpos]) << 17);
  outArray[9 + outpos] = ((inArray[13 + inpos]) >>> (21 - 6))
| ((inArray[14 + inpos]) << 6) | ((inArray[15 + inpos]) << 27);
  outArray[10 + outpos] = ((inArray[15 + inpos]) >>> (21 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[16 + inpos]) >>> (21 - 5))
| ((inArray[17 + inpos]) << 5) | ((inArray[18 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[18 + inpos]) >>> (21 - 15))
| ((inArray[19 + inpos]) << 15);
  outArray[13 + outpos] = ((inArray[19 + inpos]) >>> (21 - 4))
| ((inArray[20 + inpos]) << 4) | ((inArray[21 + inpos]) << 25);
  outArray[14 + outpos] = ((inArray[21 + inpos]) >>> (21 - 14))
| ((inArray[22 + inpos]) << 14);
  outArray[15 + outpos] = ((inArray[22 + inpos]) >>> (21 - 3))
| ((inArray[23 + inpos]) << 3) | ((inArray[24 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[24 + inpos]) >>> (21 - 13))
| ((inArray[25 + inpos]) << 13);
  outArray[17 + outpos] = ((inArray[25 + inpos]) >>> (21 - 2))
| ((inArray[26 + inpos]) << 2) | ((inArray[27 + inpos]) << 23);
  outArray[18 + outpos] = ((inArray[27 + inpos]) >>> (21 - 12))
| ((inArray[28 + inpos]) << 12);
  outArray[19 + outpos] = ((inArray[28 + inpos]) >>> (21 - 1))
| ((inArray[29 + inpos]) << 1) | ((inArray[30 + inpos]) << 22);
  outArray[20 + outpos] = ((inArray[30 + inpos]) >>> (21 - 11))
| ((inArray[31 + inpos]) << 11);
}

function fastpackwithoutmask22(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 22);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (22 - 12))
| ((inArray[2 + inpos]) << 12);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (22 - 2))
| ((inArray[3 + inpos]) << 2) | ((inArray[4 + inpos]) << 24);
  outArray[3 + outpos] = ((inArray[4 + inpos]) >>> (22 - 14))
| ((inArray[5 + inpos]) << 14);
  outArray[4 + outpos] = ((inArray[5 + inpos]) >>> (22 - 4))
| ((inArray[6 + inpos]) << 4) | ((inArray[7 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[7 + inpos]) >>> (22 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[6 + outpos] = ((inArray[8 + inpos]) >>> (22 - 6))
| ((inArray[9 + inpos]) << 6) | ((inArray[10 + inpos]) << 28);
  outArray[7 + outpos] = ((inArray[10 + inpos]) >>> (22 - 18))
| ((inArray[11 + inpos]) << 18);
  outArray[8 + outpos] = ((inArray[11 + inpos]) >>> (22 - 8))
| ((inArray[12 + inpos]) << 8) | ((inArray[13 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[13 + inpos]) >>> (22 - 20))
| ((inArray[14 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[14 + inpos]) >>> (22 - 10))
| ((inArray[15 + inpos]) << 10);
  outArray[11 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 22);
  outArray[12 + outpos] = ((inArray[17 + inpos]) >>> (22 - 12))
| ((inArray[18 + inpos]) << 12);
  outArray[13 + outpos] = ((inArray[18 + inpos]) >>> (22 - 2))
| ((inArray[19 + inpos]) << 2) | ((inArray[20 + inpos]) << 24);
  outArray[14 + outpos] = ((inArray[20 + inpos]) >>> (22 - 14))
| ((inArray[21 + inpos]) << 14);
  outArray[15 + outpos] = ((inArray[21 + inpos]) >>> (22 - 4))
| ((inArray[22 + inpos]) << 4) | ((inArray[23 + inpos]) << 26);
  outArray[16 + outpos] = ((inArray[23 + inpos]) >>> (22 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[17 + outpos] = ((inArray[24 + inpos]) >>> (22 - 6))
| ((inArray[25 + inpos]) << 6) | ((inArray[26 + inpos]) << 28);
  outArray[18 + outpos] = ((inArray[26 + inpos]) >>> (22 - 18))
| ((inArray[27 + inpos]) << 18);
  outArray[19 + outpos] = ((inArray[27 + inpos]) >>> (22 - 8))
| ((inArray[28 + inpos]) << 8) | ((inArray[29 + inpos]) << 30);
  outArray[20 + outpos] = ((inArray[29 + inpos]) >>> (22 - 20))
| ((inArray[30 + inpos]) << 20);
  outArray[21 + outpos] = ((inArray[30 + inpos]) >>> (22 - 10))
| ((inArray[31 + inpos]) << 10);
}

function fastpackwithoutmask23(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 23);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (23 - 14))
| ((inArray[2 + inpos]) << 14);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (23 - 5))
| ((inArray[3 + inpos]) << 5) | ((inArray[4 + inpos]) << 28);
  outArray[3 + outpos] = ((inArray[4 + inpos]) >>> (23 - 19))
| ((inArray[5 + inpos]) << 19);
  outArray[4 + outpos] = ((inArray[5 + inpos]) >>> (23 - 10))
| ((inArray[6 + inpos]) << 10);
  outArray[5 + outpos] = ((inArray[6 + inpos]) >>> (23 - 1))
| ((inArray[7 + inpos]) << 1) | ((inArray[8 + inpos]) << 24);
  outArray[6 + outpos] = ((inArray[8 + inpos]) >>> (23 - 15))
| ((inArray[9 + inpos]) << 15);
  outArray[7 + outpos] = ((inArray[9 + inpos]) >>> (23 - 6))
| ((inArray[10 + inpos]) << 6) | ((inArray[11 + inpos]) << 29);
  outArray[8 + outpos] = ((inArray[11 + inpos]) >>> (23 - 20))
| ((inArray[12 + inpos]) << 20);
  outArray[9 + outpos] = ((inArray[12 + inpos]) >>> (23 - 11))
| ((inArray[13 + inpos]) << 11);
  outArray[10 + outpos] = ((inArray[13 + inpos]) >>> (23 - 2))
| ((inArray[14 + inpos]) << 2) | ((inArray[15 + inpos]) << 25);
  outArray[11 + outpos] = ((inArray[15 + inpos]) >>> (23 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[12 + outpos] = ((inArray[16 + inpos]) >>> (23 - 7))
| ((inArray[17 + inpos]) << 7) | ((inArray[18 + inpos]) << 30);
  outArray[13 + outpos] = ((inArray[18 + inpos]) >>> (23 - 21))
| ((inArray[19 + inpos]) << 21);
  outArray[14 + outpos] = ((inArray[19 + inpos]) >>> (23 - 12))
| ((inArray[20 + inpos]) << 12);
  outArray[15 + outpos] = ((inArray[20 + inpos]) >>> (23 - 3))
| ((inArray[21 + inpos]) << 3) | ((inArray[22 + inpos]) << 26);
  outArray[16 + outpos] = ((inArray[22 + inpos]) >>> (23 - 17))
| ((inArray[23 + inpos]) << 17);
  outArray[17 + outpos] = ((inArray[23 + inpos]) >>> (23 - 8))
| ((inArray[24 + inpos]) << 8) | ((inArray[25 + inpos]) << 31);
  outArray[18 + outpos] = ((inArray[25 + inpos]) >>> (23 - 22))
| ((inArray[26 + inpos]) << 22);
  outArray[19 + outpos] = ((inArray[26 + inpos]) >>> (23 - 13))
| ((inArray[27 + inpos]) << 13);
  outArray[20 + outpos] = ((inArray[27 + inpos]) >>> (23 - 4))
| ((inArray[28 + inpos]) << 4) | ((inArray[29 + inpos]) << 27);
  outArray[21 + outpos] = ((inArray[29 + inpos]) >>> (23 - 18))
| ((inArray[30 + inpos]) << 18);
  outArray[22 + outpos] = ((inArray[30 + inpos]) >>> (23 - 9))
| ((inArray[31 + inpos]) << 9);
}

function fastpackwithoutmask24(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 24);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (24 - 16))
| ((inArray[2 + inpos]) << 16);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (24 - 8))
| ((inArray[3 + inpos]) << 8);
  outArray[3 + outpos] = inArray[4 + inpos] | ((inArray[5 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[5 + inpos]) >>> (24 - 16))
| ((inArray[6 + inpos]) << 16);
  outArray[5 + outpos] = ((inArray[6 + inpos]) >>> (24 - 8))
| ((inArray[7 + inpos]) << 8);
  outArray[6 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[9 + inpos]) >>> (24 - 16))
| ((inArray[10 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[10 + inpos]) >>> (24 - 8))
| ((inArray[11 + inpos]) << 8);
  outArray[9 + outpos] = inArray[12 + inpos] | ((inArray[13 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[13 + inpos]) >>> (24 - 16))
| ((inArray[14 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[14 + inpos]) >>> (24 - 8))
| ((inArray[15 + inpos]) << 8);
  outArray[12 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 24);
  outArray[13 + outpos] = ((inArray[17 + inpos]) >>> (24 - 16))
| ((inArray[18 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[18 + inpos]) >>> (24 - 8))
| ((inArray[19 + inpos]) << 8);
  outArray[15 + outpos] = inArray[20 + inpos] | ((inArray[21 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[21 + inpos]) >>> (24 - 16))
| ((inArray[22 + inpos]) << 16);
  outArray[17 + outpos] = ((inArray[22 + inpos]) >>> (24 - 8))
| ((inArray[23 + inpos]) << 8);
  outArray[18 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[25 + inpos]) >>> (24 - 16))
| ((inArray[26 + inpos]) << 16);
  outArray[20 + outpos] = ((inArray[26 + inpos]) >>> (24 - 8))
| ((inArray[27 + inpos]) << 8);
  outArray[21 + outpos] = inArray[28 + inpos] | ((inArray[29 + inpos]) << 24);
  outArray[22 + outpos] = ((inArray[29 + inpos]) >>> (24 - 16))
| ((inArray[30 + inpos]) << 16);
  outArray[23 + outpos] = ((inArray[30 + inpos]) >>> (24 - 8))
| ((inArray[31 + inpos]) << 8);
}

function fastpackwithoutmask25(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 25);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (25 - 18))
| ((inArray[2 + inpos]) << 18);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (25 - 11))
| ((inArray[3 + inpos]) << 11);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (25 - 4))
| ((inArray[4 + inpos]) << 4) | ((inArray[5 + inpos]) << 29);
  outArray[4 + outpos] = ((inArray[5 + inpos]) >>> (25 - 22))
| ((inArray[6 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[6 + inpos]) >>> (25 - 15))
| ((inArray[7 + inpos]) << 15);
  outArray[6 + outpos] = ((inArray[7 + inpos]) >>> (25 - 8))
| ((inArray[8 + inpos]) << 8);
  outArray[7 + outpos] = ((inArray[8 + inpos]) >>> (25 - 1))
| ((inArray[9 + inpos]) << 1) | ((inArray[10 + inpos]) << 26);
  outArray[8 + outpos] = ((inArray[10 + inpos]) >>> (25 - 19))
| ((inArray[11 + inpos]) << 19);
  outArray[9 + outpos] = ((inArray[11 + inpos]) >>> (25 - 12))
| ((inArray[12 + inpos]) << 12);
  outArray[10 + outpos] = ((inArray[12 + inpos]) >>> (25 - 5))
| ((inArray[13 + inpos]) << 5) | ((inArray[14 + inpos]) << 30);
  outArray[11 + outpos] = ((inArray[14 + inpos]) >>> (25 - 23))
| ((inArray[15 + inpos]) << 23);
  outArray[12 + outpos] = ((inArray[15 + inpos]) >>> (25 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[13 + outpos] = ((inArray[16 + inpos]) >>> (25 - 9))
| ((inArray[17 + inpos]) << 9);
  outArray[14 + outpos] = ((inArray[17 + inpos]) >>> (25 - 2))
| ((inArray[18 + inpos]) << 2) | ((inArray[19 + inpos]) << 27);
  outArray[15 + outpos] = ((inArray[19 + inpos]) >>> (25 - 20))
| ((inArray[20 + inpos]) << 20);
  outArray[16 + outpos] = ((inArray[20 + inpos]) >>> (25 - 13))
| ((inArray[21 + inpos]) << 13);
  outArray[17 + outpos] = ((inArray[21 + inpos]) >>> (25 - 6))
| ((inArray[22 + inpos]) << 6) | ((inArray[23 + inpos]) << 31);
  outArray[18 + outpos] = ((inArray[23 + inpos]) >>> (25 - 24))
| ((inArray[24 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[24 + inpos]) >>> (25 - 17))
| ((inArray[25 + inpos]) << 17);
  outArray[20 + outpos] = ((inArray[25 + inpos]) >>> (25 - 10))
| ((inArray[26 + inpos]) << 10);
  outArray[21 + outpos] = ((inArray[26 + inpos]) >>> (25 - 3))
| ((inArray[27 + inpos]) << 3) | ((inArray[28 + inpos]) << 28);
  outArray[22 + outpos] = ((inArray[28 + inpos]) >>> (25 - 21))
| ((inArray[29 + inpos]) << 21);
  outArray[23 + outpos] = ((inArray[29 + inpos]) >>> (25 - 14))
| ((inArray[30 + inpos]) << 14);
  outArray[24 + outpos] = ((inArray[30 + inpos]) >>> (25 - 7))
| ((inArray[31 + inpos]) << 7);
}

function fastpackwithoutmask26(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 26);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (26 - 20))
| ((inArray[2 + inpos]) << 20);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (26 - 14))
| ((inArray[3 + inpos]) << 14);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (26 - 8))
| ((inArray[4 + inpos]) << 8);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (26 - 2))
| ((inArray[5 + inpos]) << 2) | ((inArray[6 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[6 + inpos]) >>> (26 - 22))
| ((inArray[7 + inpos]) << 22);
  outArray[6 + outpos] = ((inArray[7 + inpos]) >>> (26 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[7 + outpos] = ((inArray[8 + inpos]) >>> (26 - 10))
| ((inArray[9 + inpos]) << 10);
  outArray[8 + outpos] = ((inArray[9 + inpos]) >>> (26 - 4))
| ((inArray[10 + inpos]) << 4) | ((inArray[11 + inpos]) << 30);
  outArray[9 + outpos] = ((inArray[11 + inpos]) >>> (26 - 24))
| ((inArray[12 + inpos]) << 24);
  outArray[10 + outpos] = ((inArray[12 + inpos]) >>> (26 - 18))
| ((inArray[13 + inpos]) << 18);
  outArray[11 + outpos] = ((inArray[13 + inpos]) >>> (26 - 12))
| ((inArray[14 + inpos]) << 12);
  outArray[12 + outpos] = ((inArray[14 + inpos]) >>> (26 - 6))
| ((inArray[15 + inpos]) << 6);
  outArray[13 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 26);
  outArray[14 + outpos] = ((inArray[17 + inpos]) >>> (26 - 20))
| ((inArray[18 + inpos]) << 20);
  outArray[15 + outpos] = ((inArray[18 + inpos]) >>> (26 - 14))
| ((inArray[19 + inpos]) << 14);
  outArray[16 + outpos] = ((inArray[19 + inpos]) >>> (26 - 8))
| ((inArray[20 + inpos]) << 8);
  outArray[17 + outpos] = ((inArray[20 + inpos]) >>> (26 - 2))
| ((inArray[21 + inpos]) << 2) | ((inArray[22 + inpos]) << 28);
  outArray[18 + outpos] = ((inArray[22 + inpos]) >>> (26 - 22))
| ((inArray[23 + inpos]) << 22);
  outArray[19 + outpos] = ((inArray[23 + inpos]) >>> (26 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[20 + outpos] = ((inArray[24 + inpos]) >>> (26 - 10))
| ((inArray[25 + inpos]) << 10);
  outArray[21 + outpos] = ((inArray[25 + inpos]) >>> (26 - 4))
| ((inArray[26 + inpos]) << 4) | ((inArray[27 + inpos]) << 30);
  outArray[22 + outpos] = ((inArray[27 + inpos]) >>> (26 - 24))
| ((inArray[28 + inpos]) << 24);
  outArray[23 + outpos] = ((inArray[28 + inpos]) >>> (26 - 18))
| ((inArray[29 + inpos]) << 18);
  outArray[24 + outpos] = ((inArray[29 + inpos]) >>> (26 - 12))
| ((inArray[30 + inpos]) << 12);
  outArray[25 + outpos] = ((inArray[30 + inpos]) >>> (26 - 6))
| ((inArray[31 + inpos]) << 6);
}

function fastpackwithoutmask27(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 27);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (27 - 22))
| ((inArray[2 + inpos]) << 22);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (27 - 17))
| ((inArray[3 + inpos]) << 17);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (27 - 12))
| ((inArray[4 + inpos]) << 12);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (27 - 7))
| ((inArray[5 + inpos]) << 7);
  outArray[5 + outpos] = ((inArray[5 + inpos]) >>> (27 - 2))
| ((inArray[6 + inpos]) << 2) | ((inArray[7 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[7 + inpos]) >>> (27 - 24))
| ((inArray[8 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[8 + inpos]) >>> (27 - 19))
| ((inArray[9 + inpos]) << 19);
  outArray[8 + outpos] = ((inArray[9 + inpos]) >>> (27 - 14))
| ((inArray[10 + inpos]) << 14);
  outArray[9 + outpos] = ((inArray[10 + inpos]) >>> (27 - 9))
| ((inArray[11 + inpos]) << 9);
  outArray[10 + outpos] = ((inArray[11 + inpos]) >>> (27 - 4))
| ((inArray[12 + inpos]) << 4) | ((inArray[13 + inpos]) << 31);
  outArray[11 + outpos] = ((inArray[13 + inpos]) >>> (27 - 26))
| ((inArray[14 + inpos]) << 26);
  outArray[12 + outpos] = ((inArray[14 + inpos]) >>> (27 - 21))
| ((inArray[15 + inpos]) << 21);
  outArray[13 + outpos] = ((inArray[15 + inpos]) >>> (27 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[14 + outpos] = ((inArray[16 + inpos]) >>> (27 - 11))
| ((inArray[17 + inpos]) << 11);
  outArray[15 + outpos] = ((inArray[17 + inpos]) >>> (27 - 6))
| ((inArray[18 + inpos]) << 6);
  outArray[16 + outpos] = ((inArray[18 + inpos]) >>> (27 - 1))
| ((inArray[19 + inpos]) << 1) | ((inArray[20 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[20 + inpos]) >>> (27 - 23))
| ((inArray[21 + inpos]) << 23);
  outArray[18 + outpos] = ((inArray[21 + inpos]) >>> (27 - 18))
| ((inArray[22 + inpos]) << 18);
  outArray[19 + outpos] = ((inArray[22 + inpos]) >>> (27 - 13))
| ((inArray[23 + inpos]) << 13);
  outArray[20 + outpos] = ((inArray[23 + inpos]) >>> (27 - 8))
| ((inArray[24 + inpos]) << 8);
  outArray[21 + outpos] = ((inArray[24 + inpos]) >>> (27 - 3))
| ((inArray[25 + inpos]) << 3) | ((inArray[26 + inpos]) << 30);
  outArray[22 + outpos] = ((inArray[26 + inpos]) >>> (27 - 25))
| ((inArray[27 + inpos]) << 25);
  outArray[23 + outpos] = ((inArray[27 + inpos]) >>> (27 - 20))
| ((inArray[28 + inpos]) << 20);
  outArray[24 + outpos] = ((inArray[28 + inpos]) >>> (27 - 15))
| ((inArray[29 + inpos]) << 15);
  outArray[25 + outpos] = ((inArray[29 + inpos]) >>> (27 - 10))
| ((inArray[30 + inpos]) << 10);
  outArray[26 + outpos] = ((inArray[30 + inpos]) >>> (27 - 5))
| ((inArray[31 + inpos]) << 5);
}

function fastpackwithoutmask28(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (28 - 24))
| ((inArray[2 + inpos]) << 24);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (28 - 20))
| ((inArray[3 + inpos]) << 20);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (28 - 16))
| ((inArray[4 + inpos]) << 16);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (28 - 12))
| ((inArray[5 + inpos]) << 12);
  outArray[5 + outpos] = ((inArray[5 + inpos]) >>> (28 - 8))
| ((inArray[6 + inpos]) << 8);
  outArray[6 + outpos] = ((inArray[6 + inpos]) >>> (28 - 4))
| ((inArray[7 + inpos]) << 4);
  outArray[7 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[9 + inpos]) >>> (28 - 24))
| ((inArray[10 + inpos]) << 24);
  outArray[9 + outpos] = ((inArray[10 + inpos]) >>> (28 - 20))
| ((inArray[11 + inpos]) << 20);
  outArray[10 + outpos] = ((inArray[11 + inpos]) >>> (28 - 16))
| ((inArray[12 + inpos]) << 16);
  outArray[11 + outpos] = ((inArray[12 + inpos]) >>> (28 - 12))
| ((inArray[13 + inpos]) << 12);
  outArray[12 + outpos] = ((inArray[13 + inpos]) >>> (28 - 8))
| ((inArray[14 + inpos]) << 8);
  outArray[13 + outpos] = ((inArray[14 + inpos]) >>> (28 - 4))
| ((inArray[15 + inpos]) << 4);
  outArray[14 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 28);
  outArray[15 + outpos] = ((inArray[17 + inpos]) >>> (28 - 24))
| ((inArray[18 + inpos]) << 24);
  outArray[16 + outpos] = ((inArray[18 + inpos]) >>> (28 - 20))
| ((inArray[19 + inpos]) << 20);
  outArray[17 + outpos] = ((inArray[19 + inpos]) >>> (28 - 16))
| ((inArray[20 + inpos]) << 16);
  outArray[18 + outpos] = ((inArray[20 + inpos]) >>> (28 - 12))
| ((inArray[21 + inpos]) << 12);
  outArray[19 + outpos] = ((inArray[21 + inpos]) >>> (28 - 8))
| ((inArray[22 + inpos]) << 8);
  outArray[20 + outpos] = ((inArray[22 + inpos]) >>> (28 - 4))
| ((inArray[23 + inpos]) << 4);
  outArray[21 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 28);
  outArray[22 + outpos] = ((inArray[25 + inpos]) >>> (28 - 24))
| ((inArray[26 + inpos]) << 24);
  outArray[23 + outpos] = ((inArray[26 + inpos]) >>> (28 - 20))
| ((inArray[27 + inpos]) << 20);
  outArray[24 + outpos] = ((inArray[27 + inpos]) >>> (28 - 16))
| ((inArray[28 + inpos]) << 16);
  outArray[25 + outpos] = ((inArray[28 + inpos]) >>> (28 - 12))
| ((inArray[29 + inpos]) << 12);
  outArray[26 + outpos] = ((inArray[29 + inpos]) >>> (28 - 8))
| ((inArray[30 + inpos]) << 8);
  outArray[27 + outpos] = ((inArray[30 + inpos]) >>> (28 - 4))
| ((inArray[31 + inpos]) << 4);
}

function fastpackwithoutmask29(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 29);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (29 - 26))
| ((inArray[2 + inpos]) << 26);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (29 - 23))
| ((inArray[3 + inpos]) << 23);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (29 - 20))
| ((inArray[4 + inpos]) << 20);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (29 - 17))
| ((inArray[5 + inpos]) << 17);
  outArray[5 + outpos] = ((inArray[5 + inpos]) >>> (29 - 14))
| ((inArray[6 + inpos]) << 14);
  outArray[6 + outpos] = ((inArray[6 + inpos]) >>> (29 - 11))
| ((inArray[7 + inpos]) << 11);
  outArray[7 + outpos] = ((inArray[7 + inpos]) >>> (29 - 8))
| ((inArray[8 + inpos]) << 8);
  outArray[8 + outpos] = ((inArray[8 + inpos]) >>> (29 - 5))
| ((inArray[9 + inpos]) << 5);
  outArray[9 + outpos] = ((inArray[9 + inpos]) >>> (29 - 2))
| ((inArray[10 + inpos]) << 2) | ((inArray[11 + inpos]) << 31);
  outArray[10 + outpos] = ((inArray[11 + inpos]) >>> (29 - 28))
| ((inArray[12 + inpos]) << 28);
  outArray[11 + outpos] = ((inArray[12 + inpos]) >>> (29 - 25))
| ((inArray[13 + inpos]) << 25);
  outArray[12 + outpos] = ((inArray[13 + inpos]) >>> (29 - 22))
| ((inArray[14 + inpos]) << 22);
  outArray[13 + outpos] = ((inArray[14 + inpos]) >>> (29 - 19))
| ((inArray[15 + inpos]) << 19);
  outArray[14 + outpos] = ((inArray[15 + inpos]) >>> (29 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[15 + outpos] = ((inArray[16 + inpos]) >>> (29 - 13))
| ((inArray[17 + inpos]) << 13);
  outArray[16 + outpos] = ((inArray[17 + inpos]) >>> (29 - 10))
| ((inArray[18 + inpos]) << 10);
  outArray[17 + outpos] = ((inArray[18 + inpos]) >>> (29 - 7))
| ((inArray[19 + inpos]) << 7);
  outArray[18 + outpos] = ((inArray[19 + inpos]) >>> (29 - 4))
| ((inArray[20 + inpos]) << 4);
  outArray[19 + outpos] = ((inArray[20 + inpos]) >>> (29 - 1))
| ((inArray[21 + inpos]) << 1) | ((inArray[22 + inpos]) << 30);
  outArray[20 + outpos] = ((inArray[22 + inpos]) >>> (29 - 27))
| ((inArray[23 + inpos]) << 27);
  outArray[21 + outpos] = ((inArray[23 + inpos]) >>> (29 - 24))
| ((inArray[24 + inpos]) << 24);
  outArray[22 + outpos] = ((inArray[24 + inpos]) >>> (29 - 21))
| ((inArray[25 + inpos]) << 21);
  outArray[23 + outpos] = ((inArray[25 + inpos]) >>> (29 - 18))
| ((inArray[26 + inpos]) << 18);
  outArray[24 + outpos] = ((inArray[26 + inpos]) >>> (29 - 15))
| ((inArray[27 + inpos]) << 15);
  outArray[25 + outpos] = ((inArray[27 + inpos]) >>> (29 - 12))
| ((inArray[28 + inpos]) << 12);
  outArray[26 + outpos] = ((inArray[28 + inpos]) >>> (29 - 9))
| ((inArray[29 + inpos]) << 9);
  outArray[27 + outpos] = ((inArray[29 + inpos]) >>> (29 - 6))
| ((inArray[30 + inpos]) << 6);
  outArray[28 + outpos] = ((inArray[30 + inpos]) >>> (29 - 3))
| ((inArray[31 + inpos]) << 3);
}

function fastpackwithoutmask3(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 3)
| ((inArray[2 + inpos]) << 6) | ((inArray[3 + inpos]) << 9)
| ((inArray[4 + inpos]) << 12) | ((inArray[5 + inpos]) << 15)
| ((inArray[6 + inpos]) << 18) | ((inArray[7 + inpos]) << 21)
| ((inArray[8 + inpos]) << 24) | ((inArray[9 + inpos]) << 27)
| ((inArray[10 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[10 + inpos]) >>> (3 - 1))
| ((inArray[11 + inpos]) << 1) | ((inArray[12 + inpos]) << 4)
| ((inArray[13 + inpos]) << 7) | ((inArray[14 + inpos]) << 10)
| ((inArray[15 + inpos]) << 13) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 19) | ((inArray[18 + inpos]) << 22)
| ((inArray[19 + inpos]) << 25) | ((inArray[20 + inpos]) << 28)
| ((inArray[21 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[21 + inpos]) >>> (3 - 2))
| ((inArray[22 + inpos]) << 2) | ((inArray[23 + inpos]) << 5)
| ((inArray[24 + inpos]) << 8) | ((inArray[25 + inpos]) << 11)
| ((inArray[26 + inpos]) << 14) | ((inArray[27 + inpos]) << 17)
| ((inArray[28 + inpos]) << 20) | ((inArray[29 + inpos]) << 23)
| ((inArray[30 + inpos]) << 26) | ((inArray[31 + inpos]) << 29);
}

function fastpackwithoutmask30(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (30 - 28))
| ((inArray[2 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (30 - 26))
| ((inArray[3 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (30 - 24))
| ((inArray[4 + inpos]) << 24);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (30 - 22))
| ((inArray[5 + inpos]) << 22);
  outArray[5 + outpos] = ((inArray[5 + inpos]) >>> (30 - 20))
| ((inArray[6 + inpos]) << 20);
  outArray[6 + outpos] = ((inArray[6 + inpos]) >>> (30 - 18))
| ((inArray[7 + inpos]) << 18);
  outArray[7 + outpos] = ((inArray[7 + inpos]) >>> (30 - 16))
| ((inArray[8 + inpos]) << 16);
  outArray[8 + outpos] = ((inArray[8 + inpos]) >>> (30 - 14))
| ((inArray[9 + inpos]) << 14);
  outArray[9 + outpos] = ((inArray[9 + inpos]) >>> (30 - 12))
| ((inArray[10 + inpos]) << 12);
  outArray[10 + outpos] = ((inArray[10 + inpos]) >>> (30 - 10))
| ((inArray[11 + inpos]) << 10);
  outArray[11 + outpos] = ((inArray[11 + inpos]) >>> (30 - 8))
| ((inArray[12 + inpos]) << 8);
  outArray[12 + outpos] = ((inArray[12 + inpos]) >>> (30 - 6))
| ((inArray[13 + inpos]) << 6);
  outArray[13 + outpos] = ((inArray[13 + inpos]) >>> (30 - 4))
| ((inArray[14 + inpos]) << 4);
  outArray[14 + outpos] = ((inArray[14 + inpos]) >>> (30 - 2))
| ((inArray[15 + inpos]) << 2);
  outArray[15 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 30);
  outArray[16 + outpos] = ((inArray[17 + inpos]) >>> (30 - 28))
| ((inArray[18 + inpos]) << 28);
  outArray[17 + outpos] = ((inArray[18 + inpos]) >>> (30 - 26))
| ((inArray[19 + inpos]) << 26);
  outArray[18 + outpos] = ((inArray[19 + inpos]) >>> (30 - 24))
| ((inArray[20 + inpos]) << 24);
  outArray[19 + outpos] = ((inArray[20 + inpos]) >>> (30 - 22))
| ((inArray[21 + inpos]) << 22);
  outArray[20 + outpos] = ((inArray[21 + inpos]) >>> (30 - 20))
| ((inArray[22 + inpos]) << 20);
  outArray[21 + outpos] = ((inArray[22 + inpos]) >>> (30 - 18))
| ((inArray[23 + inpos]) << 18);
  outArray[22 + outpos] = ((inArray[23 + inpos]) >>> (30 - 16))
| ((inArray[24 + inpos]) << 16);
  outArray[23 + outpos] = ((inArray[24 + inpos]) >>> (30 - 14))
| ((inArray[25 + inpos]) << 14);
  outArray[24 + outpos] = ((inArray[25 + inpos]) >>> (30 - 12))
| ((inArray[26 + inpos]) << 12);
  outArray[25 + outpos] = ((inArray[26 + inpos]) >>> (30 - 10))
| ((inArray[27 + inpos]) << 10);
  outArray[26 + outpos] = ((inArray[27 + inpos]) >>> (30 - 8))
| ((inArray[28 + inpos]) << 8);
  outArray[27 + outpos] = ((inArray[28 + inpos]) >>> (30 - 6))
| ((inArray[29 + inpos]) << 6);
  outArray[28 + outpos] = ((inArray[29 + inpos]) >>> (30 - 4))
| ((inArray[30 + inpos]) << 4);
  outArray[29 + outpos] = ((inArray[30 + inpos]) >>> (30 - 2))
| ((inArray[31 + inpos]) << 2);
}

function fastpackwithoutmask31(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 31);
  outArray[1 + outpos] = ((inArray[1 + inpos]) >>> (31 - 30))
| ((inArray[2 + inpos]) << 30);
  outArray[2 + outpos] = ((inArray[2 + inpos]) >>> (31 - 29))
| ((inArray[3 + inpos]) << 29);
  outArray[3 + outpos] = ((inArray[3 + inpos]) >>> (31 - 28))
| ((inArray[4 + inpos]) << 28);
  outArray[4 + outpos] = ((inArray[4 + inpos]) >>> (31 - 27))
| ((inArray[5 + inpos]) << 27);
  outArray[5 + outpos] = ((inArray[5 + inpos]) >>> (31 - 26))
| ((inArray[6 + inpos]) << 26);
  outArray[6 + outpos] = ((inArray[6 + inpos]) >>> (31 - 25))
| ((inArray[7 + inpos]) << 25);
  outArray[7 + outpos] = ((inArray[7 + inpos]) >>> (31 - 24))
| ((inArray[8 + inpos]) << 24);
  outArray[8 + outpos] = ((inArray[8 + inpos]) >>> (31 - 23))
| ((inArray[9 + inpos]) << 23);
  outArray[9 + outpos] = ((inArray[9 + inpos]) >>> (31 - 22))
| ((inArray[10 + inpos]) << 22);
  outArray[10 + outpos] = ((inArray[10 + inpos]) >>> (31 - 21))
| ((inArray[11 + inpos]) << 21);
  outArray[11 + outpos] = ((inArray[11 + inpos]) >>> (31 - 20))
| ((inArray[12 + inpos]) << 20);
  outArray[12 + outpos] = ((inArray[12 + inpos]) >>> (31 - 19))
| ((inArray[13 + inpos]) << 19);
  outArray[13 + outpos] = ((inArray[13 + inpos]) >>> (31 - 18))
| ((inArray[14 + inpos]) << 18);
  outArray[14 + outpos] = ((inArray[14 + inpos]) >>> (31 - 17))
| ((inArray[15 + inpos]) << 17);
  outArray[15 + outpos] = ((inArray[15 + inpos]) >>> (31 - 16))
| ((inArray[16 + inpos]) << 16);
  outArray[16 + outpos] = ((inArray[16 + inpos]) >>> (31 - 15))
| ((inArray[17 + inpos]) << 15);
  outArray[17 + outpos] = ((inArray[17 + inpos]) >>> (31 - 14))
| ((inArray[18 + inpos]) << 14);
  outArray[18 + outpos] = ((inArray[18 + inpos]) >>> (31 - 13))
| ((inArray[19 + inpos]) << 13);
  outArray[19 + outpos] = ((inArray[19 + inpos]) >>> (31 - 12))
| ((inArray[20 + inpos]) << 12);
  outArray[20 + outpos] = ((inArray[20 + inpos]) >>> (31 - 11))
| ((inArray[21 + inpos]) << 11);
  outArray[21 + outpos] = ((inArray[21 + inpos]) >>> (31 - 10))
| ((inArray[22 + inpos]) << 10);
  outArray[22 + outpos] = ((inArray[22 + inpos]) >>> (31 - 9))
| ((inArray[23 + inpos]) << 9);
  outArray[23 + outpos] = ((inArray[23 + inpos]) >>> (31 - 8))
| ((inArray[24 + inpos]) << 8);
  outArray[24 + outpos] = ((inArray[24 + inpos]) >>> (31 - 7))
| ((inArray[25 + inpos]) << 7);
  outArray[25 + outpos] = ((inArray[25 + inpos]) >>> (31 - 6))
| ((inArray[26 + inpos]) << 6);
  outArray[26 + outpos] = ((inArray[26 + inpos]) >>> (31 - 5))
| ((inArray[27 + inpos]) << 5);
  outArray[27 + outpos] = ((inArray[27 + inpos]) >>> (31 - 4))
| ((inArray[28 + inpos]) << 4);
  outArray[28 + outpos] = ((inArray[28 + inpos]) >>> (31 - 3))
| ((inArray[29 + inpos]) << 3);
  outArray[29 + outpos] = ((inArray[29 + inpos]) >>> (31 - 2))
| ((inArray[30 + inpos]) << 2);
  outArray[30 + outpos] = ((inArray[30 + inpos]) >>> (31 - 1))
| ((inArray[31 + inpos]) << 1);
}

function fastpackwithoutmask32(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  arraycopy(inArray, inpos, outArray, outpos, 32);
}

function fastpackwithoutmask4(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 4)
| ((inArray[2 + inpos]) << 8) | ((inArray[3 + inpos]) << 12)
| ((inArray[4 + inpos]) << 16) | ((inArray[5 + inpos]) << 20)
| ((inArray[6 + inpos]) << 24) | ((inArray[7 + inpos]) << 28);
  outArray[1 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 4)
| ((inArray[10 + inpos]) << 8) | ((inArray[11 + inpos]) << 12)
| ((inArray[12 + inpos]) << 16) | ((inArray[13 + inpos]) << 20)
| ((inArray[14 + inpos]) << 24) | ((inArray[15 + inpos]) << 28);
  outArray[2 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 4)
| ((inArray[18 + inpos]) << 8) | ((inArray[19 + inpos]) << 12)
| ((inArray[20 + inpos]) << 16) | ((inArray[21 + inpos]) << 20)
| ((inArray[22 + inpos]) << 24) | ((inArray[23 + inpos]) << 28);
  outArray[3 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 4)
| ((inArray[26 + inpos]) << 8) | ((inArray[27 + inpos]) << 12)
| ((inArray[28 + inpos]) << 16) | ((inArray[29 + inpos]) << 20)
| ((inArray[30 + inpos]) << 24) | ((inArray[31 + inpos]) << 28);
}

function fastpackwithoutmask5(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 5)
| ((inArray[2 + inpos]) << 10) | ((inArray[3 + inpos]) << 15)
| ((inArray[4 + inpos]) << 20) | ((inArray[5 + inpos]) << 25)
| ((inArray[6 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[6 + inpos]) >>> (5 - 3))
| ((inArray[7 + inpos]) << 3) | ((inArray[8 + inpos]) << 8)
| ((inArray[9 + inpos]) << 13) | ((inArray[10 + inpos]) << 18)
| ((inArray[11 + inpos]) << 23) | ((inArray[12 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[12 + inpos]) >>> (5 - 1))
| ((inArray[13 + inpos]) << 1) | ((inArray[14 + inpos]) << 6)
| ((inArray[15 + inpos]) << 11) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 21) | ((inArray[18 + inpos]) << 26)
| ((inArray[19 + inpos]) << 31);
  outArray[3 + outpos] = ((inArray[19 + inpos]) >>> (5 - 4))
| ((inArray[20 + inpos]) << 4) | ((inArray[21 + inpos]) << 9)
| ((inArray[22 + inpos]) << 14) | ((inArray[23 + inpos]) << 19)
| ((inArray[24 + inpos]) << 24) | ((inArray[25 + inpos]) << 29);
  outArray[4 + outpos] = ((inArray[25 + inpos]) >>> (5 - 2))
| ((inArray[26 + inpos]) << 2) | ((inArray[27 + inpos]) << 7)
| ((inArray[28 + inpos]) << 12) | ((inArray[29 + inpos]) << 17)
| ((inArray[30 + inpos]) << 22) | ((inArray[31 + inpos]) << 27);
}

function fastpackwithoutmask6(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 6)
| ((inArray[2 + inpos]) << 12) | ((inArray[3 + inpos]) << 18)
| ((inArray[4 + inpos]) << 24) | ((inArray[5 + inpos]) << 30);
  outArray[1 + outpos] = ((inArray[5 + inpos]) >>> (6 - 4))
| ((inArray[6 + inpos]) << 4) | ((inArray[7 + inpos]) << 10)
| ((inArray[8 + inpos]) << 16) | ((inArray[9 + inpos]) << 22)
| ((inArray[10 + inpos]) << 28);
  outArray[2 + outpos] = ((inArray[10 + inpos]) >>> (6 - 2))
| ((inArray[11 + inpos]) << 2) | ((inArray[12 + inpos]) << 8)
| ((inArray[13 + inpos]) << 14) | ((inArray[14 + inpos]) << 20)
| ((inArray[15 + inpos]) << 26);
  outArray[3 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 6)
| ((inArray[18 + inpos]) << 12) | ((inArray[19 + inpos]) << 18)
| ((inArray[20 + inpos]) << 24) | ((inArray[21 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[21 + inpos]) >>> (6 - 4))
| ((inArray[22 + inpos]) << 4) | ((inArray[23 + inpos]) << 10)
| ((inArray[24 + inpos]) << 16) | ((inArray[25 + inpos]) << 22)
| ((inArray[26 + inpos]) << 28);
  outArray[5 + outpos] = ((inArray[26 + inpos]) >>> (6 - 2))
| ((inArray[27 + inpos]) << 2) | ((inArray[28 + inpos]) << 8)
| ((inArray[29 + inpos]) << 14) | ((inArray[30 + inpos]) << 20)
| ((inArray[31 + inpos]) << 26);
}

function fastpackwithoutmask7(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 7)
| ((inArray[2 + inpos]) << 14) | ((inArray[3 + inpos]) << 21)
| ((inArray[4 + inpos]) << 28);
  outArray[1 + outpos] = ((inArray[4 + inpos]) >>> (7 - 3))
| ((inArray[5 + inpos]) << 3) | ((inArray[6 + inpos]) << 10)
| ((inArray[7 + inpos]) << 17) | ((inArray[8 + inpos]) << 24)
| ((inArray[9 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[9 + inpos]) >>> (7 - 6))
| ((inArray[10 + inpos]) << 6) | ((inArray[11 + inpos]) << 13)
| ((inArray[12 + inpos]) << 20) | ((inArray[13 + inpos]) << 27);
  outArray[3 + outpos] = ((inArray[13 + inpos]) >>> (7 - 2))
| ((inArray[14 + inpos]) << 2) | ((inArray[15 + inpos]) << 9)
| ((inArray[16 + inpos]) << 16) | ((inArray[17 + inpos]) << 23)
| ((inArray[18 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[18 + inpos]) >>> (7 - 5))
| ((inArray[19 + inpos]) << 5) | ((inArray[20 + inpos]) << 12)
| ((inArray[21 + inpos]) << 19) | ((inArray[22 + inpos]) << 26);
  outArray[5 + outpos] = ((inArray[22 + inpos]) >>> (7 - 1))
| ((inArray[23 + inpos]) << 1) | ((inArray[24 + inpos]) << 8)
| ((inArray[25 + inpos]) << 15) | ((inArray[26 + inpos]) << 22)
| ((inArray[27 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[27 + inpos]) >>> (7 - 4))
| ((inArray[28 + inpos]) << 4) | ((inArray[29 + inpos]) << 11)
| ((inArray[30 + inpos]) << 18) | ((inArray[31 + inpos]) << 25);
}

function fastpackwithoutmask8(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 8)
| ((inArray[2 + inpos]) << 16) | ((inArray[3 + inpos]) << 24);
  outArray[1 + outpos] = inArray[4 + inpos] | ((inArray[5 + inpos]) << 8)
| ((inArray[6 + inpos]) << 16) | ((inArray[7 + inpos]) << 24);
  outArray[2 + outpos] = inArray[8 + inpos] | ((inArray[9 + inpos]) << 8)
| ((inArray[10 + inpos]) << 16) | ((inArray[11 + inpos]) << 24);
  outArray[3 + outpos] = inArray[12 + inpos] | ((inArray[13 + inpos]) << 8)
| ((inArray[14 + inpos]) << 16) | ((inArray[15 + inpos]) << 24);
  outArray[4 + outpos] = inArray[16 + inpos] | ((inArray[17 + inpos]) << 8)
| ((inArray[18 + inpos]) << 16) | ((inArray[19 + inpos]) << 24);
  outArray[5 + outpos] = inArray[20 + inpos] | ((inArray[21 + inpos]) << 8)
| ((inArray[22 + inpos]) << 16) | ((inArray[23 + inpos]) << 24);
  outArray[6 + outpos] = inArray[24 + inpos] | ((inArray[25 + inpos]) << 8)
| ((inArray[26 + inpos]) << 16) | ((inArray[27 + inpos]) << 24);
  outArray[7 + outpos] = inArray[28 + inpos] | ((inArray[29 + inpos]) << 8)
| ((inArray[30 + inpos]) << 16) | ((inArray[31 + inpos]) << 24);
}

function fastpackwithoutmask9(inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number) {
  outArray[outpos] = inArray[inpos] | ((inArray[1 + inpos]) << 9)
| ((inArray[2 + inpos]) << 18) | ((inArray[3 + inpos]) << 27);
  outArray[1 + outpos] = ((inArray[3 + inpos]) >>> (9 - 4))
| ((inArray[4 + inpos]) << 4) | ((inArray[5 + inpos]) << 13)
| ((inArray[6 + inpos]) << 22) | ((inArray[7 + inpos]) << 31);
  outArray[2 + outpos] = ((inArray[7 + inpos]) >>> (9 - 8))
| ((inArray[8 + inpos]) << 8) | ((inArray[9 + inpos]) << 17)
| ((inArray[10 + inpos]) << 26);
  outArray[3 + outpos] = ((inArray[10 + inpos]) >>> (9 - 3))
| ((inArray[11 + inpos]) << 3) | ((inArray[12 + inpos]) << 12)
| ((inArray[13 + inpos]) << 21) | ((inArray[14 + inpos]) << 30);
  outArray[4 + outpos] = ((inArray[14 + inpos]) >>> (9 - 7))
| ((inArray[15 + inpos]) << 7) | ((inArray[16 + inpos]) << 16)
| ((inArray[17 + inpos]) << 25);
  outArray[5 + outpos] = ((inArray[17 + inpos]) >>> (9 - 2))
| ((inArray[18 + inpos]) << 2) | ((inArray[19 + inpos]) << 11)
| ((inArray[20 + inpos]) << 20) | ((inArray[21 + inpos]) << 29);
  outArray[6 + outpos] = ((inArray[21 + inpos]) >>> (9 - 6))
| ((inArray[22 + inpos]) << 6) | ((inArray[23 + inpos]) << 15)
| ((inArray[24 + inpos]) << 24);
  outArray[7 + outpos] = ((inArray[24 + inpos]) >>> (9 - 1))
| ((inArray[25 + inpos]) << 1) | ((inArray[26 + inpos]) << 10)
| ((inArray[27 + inpos]) << 19) | ((inArray[28 + inpos]) << 28);
  outArray[8 + outpos] = ((inArray[28 + inpos]) >>> (9 - 5))
| ((inArray[29 + inpos]) << 5) | ((inArray[30 + inpos]) << 14)
| ((inArray[31 + inpos]) << 23);
}

/**
 * Pack the 32 numberegers
 *
 * @param inArray
 *                source array
 * @param inpos
 *                starting ponumber in the source array
 * @param outArray
 *                output array
 * @param outpos
 *                starting ponumber in the output array
 * @param bit
 *                how many bits to use per numbereger
 */
export function fastunpack(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number, bit: number }) {
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

function fastunpack0(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray.fill(0, model.outpos, model.outpos + 32);
}

function fastunpack1(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 1);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 1) & 1);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 2) & 1);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 3) & 1);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 4) & 1);
  model.outArray[5 + model.outpos] = ((model.inArray[model.inpos] >>> 5) & 1);
  model.outArray[6 + model.outpos] = ((model.inArray[model.inpos] >>> 6) & 1);
  model.outArray[7 + model.outpos] = ((model.inArray[model.inpos] >>> 7) & 1);
  model.outArray[8 + model.outpos] = ((model.inArray[model.inpos] >>> 8) & 1);
  model.outArray[9 + model.outpos] = ((model.inArray[model.inpos] >>> 9) & 1);
  model.outArray[10 + model.outpos] = ((model.inArray[model.inpos] >>> 10) & 1);
  model.outArray[11 + model.outpos] = ((model.inArray[model.inpos] >>> 11) & 1);
  model.outArray[12 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 1);
  model.outArray[13 + model.outpos] = ((model.inArray[model.inpos] >>> 13) & 1);
  model.outArray[14 + model.outpos] = ((model.inArray[model.inpos] >>> 14) & 1);
  model.outArray[15 + model.outpos] = ((model.inArray[model.inpos] >>> 15) & 1);
  model.outArray[16 + model.outpos] = ((model.inArray[model.inpos] >>> 16) & 1);
  model.outArray[17 + model.outpos] = ((model.inArray[model.inpos] >>> 17) & 1);
  model.outArray[18 + model.outpos] = ((model.inArray[model.inpos] >>> 18) & 1);
  model.outArray[19 + model.outpos] = ((model.inArray[model.inpos] >>> 19) & 1);
  model.outArray[20 + model.outpos] = ((model.inArray[model.inpos] >>> 20) & 1);
  model.outArray[21 + model.outpos] = ((model.inArray[model.inpos] >>> 21) & 1);
  model.outArray[22 + model.outpos] = ((model.inArray[model.inpos] >>> 22) & 1);
  model.outArray[23 + model.outpos] = ((model.inArray[model.inpos] >>> 23) & 1);
  model.outArray[24 + model.outpos] = ((model.inArray[model.inpos] >>> 24) & 1);
  model.outArray[25 + model.outpos] = ((model.inArray[model.inpos] >>> 25) & 1);
  model.outArray[26 + model.outpos] = ((model.inArray[model.inpos] >>> 26) & 1);
  model.outArray[27 + model.outpos] = ((model.inArray[model.inpos] >>> 27) & 1);
  model.outArray[28 + model.outpos] = ((model.inArray[model.inpos] >>> 28) & 1);
  model.outArray[29 + model.outpos] = ((model.inArray[model.inpos] >>> 29) & 1);
  model.outArray[30 + model.outpos] = ((model.inArray[model.inpos] >>> 30) & 1);
  model.outArray[31 + model.outpos] = (model.inArray[model.inpos] >>> 31);
}

function fastunpack10(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 1023);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 10) & 1023);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 20) & 1023);
  model.outArray[3 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 255) << (10 - 8));
  model.outArray[4 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 1023);
  model.outArray[5 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 18) & 1023);
  model.outArray[6 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 63) << (10 - 6));
  model.outArray[7 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 6) & 1023);
  model.outArray[8 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 16) & 1023);
  model.outArray[9 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26)
| ((model.inArray[3 + model.inpos] & 15) << (10 - 4));
  model.outArray[10 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 4) & 1023);
  model.outArray[11 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 14) & 1023);
  model.outArray[12 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 3) << (10 - 2));
  model.outArray[13 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 2) & 1023);
  model.outArray[14 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 12) & 1023);
  model.outArray[15 + model.outpos] = (model.inArray[4 + model.inpos] >>> 22);
  model.outArray[16 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 0) & 1023);
  model.outArray[17 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 10) & 1023);
  model.outArray[18 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 20) & 1023);
  model.outArray[19 + model.outpos] = (model.inArray[5 + model.inpos] >>> 30)
| ((model.inArray[6 + model.inpos] & 255) << (10 - 8));
  model.outArray[20 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 8) & 1023);
  model.outArray[21 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 18) & 1023);
  model.outArray[22 + model.outpos] = (model.inArray[6 + model.inpos] >>> 28)
| ((model.inArray[7 + model.inpos] & 63) << (10 - 6));
  model.outArray[23 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 6) & 1023);
  model.outArray[24 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 16) & 1023);
  model.outArray[25 + model.outpos] = (model.inArray[7 + model.inpos] >>> 26)
| ((model.inArray[8 + model.inpos] & 15) << (10 - 4));
  model.outArray[26 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 4) & 1023);
  model.outArray[27 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 14) & 1023);
  model.outArray[28 + model.outpos] = (model.inArray[8 + model.inpos] >>> 24)
| ((model.inArray[9 + model.inpos] & 3) << (10 - 2));
  model.outArray[29 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 2) & 1023);
  model.outArray[30 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 12) & 1023);
  model.outArray[31 + model.outpos] = (model.inArray[9 + model.inpos] >>> 22);
}

function fastunpack11(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 2047);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 11) & 2047);
  model.outArray[2 + model.outpos] = (model.inArray[model.inpos] >>> 22)
| ((model.inArray[1 + model.inpos] & 1) << (11 - 1));
  model.outArray[3 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 1) & 2047);
  model.outArray[4 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 12) & 2047);
  model.outArray[5 + model.outpos] = (model.inArray[1 + model.inpos] >>> 23)
| ((model.inArray[2 + model.inpos] & 3) << (11 - 2));
  model.outArray[6 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 2) & 2047);
  model.outArray[7 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 13) & 2047);
  model.outArray[8 + model.outpos] = (model.inArray[2 + model.inpos] >>> 24)
| ((model.inArray[3 + model.inpos] & 7) << (11 - 3));
  model.outArray[9 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 3) & 2047);
  model.outArray[10 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 14) & 2047);
  model.outArray[11 + model.outpos] = (model.inArray[3 + model.inpos] >>> 25)
| ((model.inArray[4 + model.inpos] & 15) << (11 - 4));
  model.outArray[12 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 4) & 2047);
  model.outArray[13 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 15) & 2047);
  model.outArray[14 + model.outpos] = (model.inArray[4 + model.inpos] >>> 26)
| ((model.inArray[5 + model.inpos] & 31) << (11 - 5));
  model.outArray[15 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 5) & 2047);
  model.outArray[16 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 16) & 2047);
  model.outArray[17 + model.outpos] = (model.inArray[5 + model.inpos] >>> 27)
| ((model.inArray[6 + model.inpos] & 63) << (11 - 6));
  model.outArray[18 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 6) & 2047);
  model.outArray[19 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 17) & 2047);
  model.outArray[20 + model.outpos] = (model.inArray[6 + model.inpos] >>> 28)
| ((model.inArray[7 + model.inpos] & 127) << (11 - 7));
  model.outArray[21 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 7) & 2047);
  model.outArray[22 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 18) & 2047);
  model.outArray[23 + model.outpos] = (model.inArray[7 + model.inpos] >>> 29)
| ((model.inArray[8 + model.inpos] & 255) << (11 - 8));
  model.outArray[24 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 8) & 2047);
  model.outArray[25 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 19) & 2047);
  model.outArray[26 + model.outpos] = (model.inArray[8 + model.inpos] >>> 30)
| ((model.inArray[9 + model.inpos] & 511) << (11 - 9));
  model.outArray[27 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 9) & 2047);
  model.outArray[28 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 20) & 2047);
  model.outArray[29 + model.outpos] = (model.inArray[9 + model.inpos] >>> 31)
| ((model.inArray[10 + model.inpos] & 1023) << (11 - 10));
  model.outArray[30 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 10) & 2047);
  model.outArray[31 + model.outpos] = (model.inArray[10 + model.inpos] >>> 21);
}

function fastunpack12(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 4095);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 4095);
  model.outArray[2 + model.outpos] = (model.inArray[model.inpos] >>> 24)
| ((model.inArray[1 + model.inpos] & 15) << (12 - 4));
  model.outArray[3 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 4095);
  model.outArray[4 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 4095);
  model.outArray[5 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 255) << (12 - 8));
  model.outArray[6 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 4095);
  model.outArray[7 + model.outpos] = (model.inArray[2 + model.inpos] >>> 20);
  model.outArray[8 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 4095);
  model.outArray[9 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 12) & 4095);
  model.outArray[10 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 15) << (12 - 4));
  model.outArray[11 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 4) & 4095);
  model.outArray[12 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 16) & 4095);
  model.outArray[13 + model.outpos] = (model.inArray[4 + model.inpos] >>> 28)
| ((model.inArray[5 + model.inpos] & 255) << (12 - 8));
  model.outArray[14 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 4095);
  model.outArray[15 + model.outpos] = (model.inArray[5 + model.inpos] >>> 20);
  model.outArray[16 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 0) & 4095);
  model.outArray[17 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 12) & 4095);
  model.outArray[18 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24)
| ((model.inArray[7 + model.inpos] & 15) << (12 - 4));
  model.outArray[19 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 4) & 4095);
  model.outArray[20 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 16) & 4095);
  model.outArray[21 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 255) << (12 - 8));
  model.outArray[22 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 8) & 4095);
  model.outArray[23 + model.outpos] = (model.inArray[8 + model.inpos] >>> 20);
  model.outArray[24 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 0) & 4095);
  model.outArray[25 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 12) & 4095);
  model.outArray[26 + model.outpos] = (model.inArray[9 + model.inpos] >>> 24)
| ((model.inArray[10 + model.inpos] & 15) << (12 - 4));
  model.outArray[27 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 4) & 4095);
  model.outArray[28 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 16) & 4095);
  model.outArray[29 + model.outpos] = (model.inArray[10 + model.inpos] >>> 28)
| ((model.inArray[11 + model.inpos] & 255) << (12 - 8));
  model.outArray[30 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 8) & 4095);
  model.outArray[31 + model.outpos] = (model.inArray[11 + model.inpos] >>> 20);
}

function fastunpack13(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 8191);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 13) & 8191);
  model.outArray[2 + model.outpos] = (model.inArray[model.inpos] >>> 26)
| ((model.inArray[1 + model.inpos] & 127) << (13 - 7));
  model.outArray[3 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 7) & 8191);
  model.outArray[4 + model.outpos] = (model.inArray[1 + model.inpos] >>> 20)
| ((model.inArray[2 + model.inpos] & 1) << (13 - 1));
  model.outArray[5 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 1) & 8191);
  model.outArray[6 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 14) & 8191);
  model.outArray[7 + model.outpos] = (model.inArray[2 + model.inpos] >>> 27)
| ((model.inArray[3 + model.inpos] & 255) << (13 - 8));
  model.outArray[8 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 8) & 8191);
  model.outArray[9 + model.outpos] = (model.inArray[3 + model.inpos] >>> 21)
| ((model.inArray[4 + model.inpos] & 3) << (13 - 2));
  model.outArray[10 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 2) & 8191);
  model.outArray[11 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 15) & 8191);
  model.outArray[12 + model.outpos] = (model.inArray[4 + model.inpos] >>> 28)
| ((model.inArray[5 + model.inpos] & 511) << (13 - 9));
  model.outArray[13 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 9) & 8191);
  model.outArray[14 + model.outpos] = (model.inArray[5 + model.inpos] >>> 22)
| ((model.inArray[6 + model.inpos] & 7) << (13 - 3));
  model.outArray[15 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 3) & 8191);
  model.outArray[16 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 16) & 8191);
  model.outArray[17 + model.outpos] = (model.inArray[6 + model.inpos] >>> 29)
| ((model.inArray[7 + model.inpos] & 1023) << (13 - 10));
  model.outArray[18 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 10) & 8191);
  model.outArray[19 + model.outpos] = (model.inArray[7 + model.inpos] >>> 23)
| ((model.inArray[8 + model.inpos] & 15) << (13 - 4));
  model.outArray[20 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 4) & 8191);
  model.outArray[21 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 17) & 8191);
  model.outArray[22 + model.outpos] = (model.inArray[8 + model.inpos] >>> 30)
| ((model.inArray[9 + model.inpos] & 2047) << (13 - 11));
  model.outArray[23 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 11) & 8191);
  model.outArray[24 + model.outpos] = (model.inArray[9 + model.inpos] >>> 24)
| ((model.inArray[10 + model.inpos] & 31) << (13 - 5));
  model.outArray[25 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 5) & 8191);
  model.outArray[26 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 18) & 8191);
  model.outArray[27 + model.outpos] = (model.inArray[10 + model.inpos] >>> 31)
| ((model.inArray[11 + model.inpos] & 4095) << (13 - 12));
  model.outArray[28 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 12) & 8191);
  model.outArray[29 + model.outpos] = (model.inArray[11 + model.inpos] >>> 25)
| ((model.inArray[12 + model.inpos] & 63) << (13 - 6));
  model.outArray[30 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 6) & 8191);
  model.outArray[31 + model.outpos] = (model.inArray[12 + model.inpos] >>> 19);
}

function fastunpack14(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 16383);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 14) & 16383);
  model.outArray[2 + model.outpos] = (model.inArray[model.inpos] >>> 28)
| ((model.inArray[1 + model.inpos] & 1023) << (14 - 10));
  model.outArray[3 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 16383);
  model.outArray[4 + model.outpos] = (model.inArray[1 + model.inpos] >>> 24)
| ((model.inArray[2 + model.inpos] & 63) << (14 - 6));
  model.outArray[5 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 6) & 16383);
  model.outArray[6 + model.outpos] = (model.inArray[2 + model.inpos] >>> 20)
| ((model.inArray[3 + model.inpos] & 3) << (14 - 2));
  model.outArray[7 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 2) & 16383);
  model.outArray[8 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 16) & 16383);
  model.outArray[9 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 4095) << (14 - 12));
  model.outArray[10 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 12) & 16383);
  model.outArray[11 + model.outpos] = (model.inArray[4 + model.inpos] >>> 26)
| ((model.inArray[5 + model.inpos] & 255) << (14 - 8));
  model.outArray[12 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 16383);
  model.outArray[13 + model.outpos] = (model.inArray[5 + model.inpos] >>> 22)
| ((model.inArray[6 + model.inpos] & 15) << (14 - 4));
  model.outArray[14 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 4) & 16383);
  model.outArray[15 + model.outpos] = (model.inArray[6 + model.inpos] >>> 18);
  model.outArray[16 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 0) & 16383);
  model.outArray[17 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 14) & 16383);
  model.outArray[18 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 1023) << (14 - 10));
  model.outArray[19 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 10) & 16383);
  model.outArray[20 + model.outpos] = (model.inArray[8 + model.inpos] >>> 24)
| ((model.inArray[9 + model.inpos] & 63) << (14 - 6));
  model.outArray[21 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 6) & 16383);
  model.outArray[22 + model.outpos] = (model.inArray[9 + model.inpos] >>> 20)
| ((model.inArray[10 + model.inpos] & 3) << (14 - 2));
  model.outArray[23 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 2) & 16383);
  model.outArray[24 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 16) & 16383);
  model.outArray[25 + model.outpos] = (model.inArray[10 + model.inpos] >>> 30)
| ((model.inArray[11 + model.inpos] & 4095) << (14 - 12));
  model.outArray[26 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 12) & 16383);
  model.outArray[27 + model.outpos] = (model.inArray[11 + model.inpos] >>> 26)
| ((model.inArray[12 + model.inpos] & 255) << (14 - 8));
  model.outArray[28 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 8) & 16383);
  model.outArray[29 + model.outpos] = (model.inArray[12 + model.inpos] >>> 22)
| ((model.inArray[13 + model.inpos] & 15) << (14 - 4));
  model.outArray[30 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 4) & 16383);
  model.outArray[31 + model.outpos] = (model.inArray[13 + model.inpos] >>> 18);
}

function fastunpack15(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 32767);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 15) & 32767);
  model.outArray[2 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 8191) << (15 - 13));
  model.outArray[3 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 13) & 32767);
  model.outArray[4 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 2047) << (15 - 11));
  model.outArray[5 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 11) & 32767);
  model.outArray[6 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26)
| ((model.inArray[3 + model.inpos] & 511) << (15 - 9));
  model.outArray[7 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 9) & 32767);
  model.outArray[8 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 127) << (15 - 7));
  model.outArray[9 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 7) & 32767);
  model.outArray[10 + model.outpos] = (model.inArray[4 + model.inpos] >>> 22)
| ((model.inArray[5 + model.inpos] & 31) << (15 - 5));
  model.outArray[11 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 5) & 32767);
  model.outArray[12 + model.outpos] = (model.inArray[5 + model.inpos] >>> 20)
| ((model.inArray[6 + model.inpos] & 7) << (15 - 3));
  model.outArray[13 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 3) & 32767);
  model.outArray[14 + model.outpos] = (model.inArray[6 + model.inpos] >>> 18)
| ((model.inArray[7 + model.inpos] & 1) << (15 - 1));
  model.outArray[15 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 1) & 32767);
  model.outArray[16 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 16) & 32767);
  model.outArray[17 + model.outpos] = (model.inArray[7 + model.inpos] >>> 31)
| ((model.inArray[8 + model.inpos] & 16383) << (15 - 14));
  model.outArray[18 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 14) & 32767);
  model.outArray[19 + model.outpos] = (model.inArray[8 + model.inpos] >>> 29)
| ((model.inArray[9 + model.inpos] & 4095) << (15 - 12));
  model.outArray[20 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 12) & 32767);
  model.outArray[21 + model.outpos] = (model.inArray[9 + model.inpos] >>> 27)
| ((model.inArray[10 + model.inpos] & 1023) << (15 - 10));
  model.outArray[22 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 10) & 32767);
  model.outArray[23 + model.outpos] = (model.inArray[10 + model.inpos] >>> 25)
| ((model.inArray[11 + model.inpos] & 255) << (15 - 8));
  model.outArray[24 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 8) & 32767);
  model.outArray[25 + model.outpos] = (model.inArray[11 + model.inpos] >>> 23)
| ((model.inArray[12 + model.inpos] & 63) << (15 - 6));
  model.outArray[26 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 6) & 32767);
  model.outArray[27 + model.outpos] = (model.inArray[12 + model.inpos] >>> 21)
| ((model.inArray[13 + model.inpos] & 15) << (15 - 4));
  model.outArray[28 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 4) & 32767);
  model.outArray[29 + model.outpos] = (model.inArray[13 + model.inpos] >>> 19)
| ((model.inArray[14 + model.inpos] & 3) << (15 - 2));
  model.outArray[30 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 2) & 32767);
  model.outArray[31 + model.outpos] = (model.inArray[14 + model.inpos] >>> 17);
}

function fastunpack16(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 65535);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 16);
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 0) & 65535);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 16);
  model.outArray[4 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 0) & 65535);
  model.outArray[5 + model.outpos] = (model.inArray[2 + model.inpos] >>> 16);
  model.outArray[6 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 65535);
  model.outArray[7 + model.outpos] = (model.inArray[3 + model.inpos] >>> 16);
  model.outArray[8 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 0) & 65535);
  model.outArray[9 + model.outpos] = (model.inArray[4 + model.inpos] >>> 16);
  model.outArray[10 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 0) & 65535);
  model.outArray[11 + model.outpos] = (model.inArray[5 + model.inpos] >>> 16);
  model.outArray[12 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 0) & 65535);
  model.outArray[13 + model.outpos] = (model.inArray[6 + model.inpos] >>> 16);
  model.outArray[14 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 0) & 65535);
  model.outArray[15 + model.outpos] = (model.inArray[7 + model.inpos] >>> 16);
  model.outArray[16 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 0) & 65535);
  model.outArray[17 + model.outpos] = (model.inArray[8 + model.inpos] >>> 16);
  model.outArray[18 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 0) & 65535);
  model.outArray[19 + model.outpos] = (model.inArray[9 + model.inpos] >>> 16);
  model.outArray[20 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 0) & 65535);
  model.outArray[21 + model.outpos] = (model.inArray[10 + model.inpos] >>> 16);
  model.outArray[22 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 0) & 65535);
  model.outArray[23 + model.outpos] = (model.inArray[11 + model.inpos] >>> 16);
  model.outArray[24 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 0) & 65535);
  model.outArray[25 + model.outpos] = (model.inArray[12 + model.inpos] >>> 16);
  model.outArray[26 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 0) & 65535);
  model.outArray[27 + model.outpos] = (model.inArray[13 + model.inpos] >>> 16);
  model.outArray[28 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 0) & 65535);
  model.outArray[29 + model.outpos] = (model.inArray[14 + model.inpos] >>> 16);
  model.outArray[30 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 0) & 65535);
  model.outArray[31 + model.outpos] = (model.inArray[15 + model.inpos] >>> 16);
}

function fastunpack17(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 131071);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 17)
| ((model.inArray[1 + model.inpos] & 3) << (17 - 2));
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 2) & 131071);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 19)
| ((model.inArray[2 + model.inpos] & 15) << (17 - 4));
  model.outArray[4 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 4) & 131071);
  model.outArray[5 + model.outpos] = (model.inArray[2 + model.inpos] >>> 21)
| ((model.inArray[3 + model.inpos] & 63) << (17 - 6));
  model.outArray[6 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 6) & 131071);
  model.outArray[7 + model.outpos] = (model.inArray[3 + model.inpos] >>> 23)
| ((model.inArray[4 + model.inpos] & 255) << (17 - 8));
  model.outArray[8 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 8) & 131071);
  model.outArray[9 + model.outpos] = (model.inArray[4 + model.inpos] >>> 25)
| ((model.inArray[5 + model.inpos] & 1023) << (17 - 10));
  model.outArray[10 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 10) & 131071);
  model.outArray[11 + model.outpos] = (model.inArray[5 + model.inpos] >>> 27)
| ((model.inArray[6 + model.inpos] & 4095) << (17 - 12));
  model.outArray[12 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 12) & 131071);
  model.outArray[13 + model.outpos] = (model.inArray[6 + model.inpos] >>> 29)
| ((model.inArray[7 + model.inpos] & 16383) << (17 - 14));
  model.outArray[14 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 14) & 131071);
  model.outArray[15 + model.outpos] = (model.inArray[7 + model.inpos] >>> 31)
| ((model.inArray[8 + model.inpos] & 65535) << (17 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[8 + model.inpos] >>> 16)
| ((model.inArray[9 + model.inpos] & 1) << (17 - 1));
  model.outArray[17 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 1) & 131071);
  model.outArray[18 + model.outpos] = (model.inArray[9 + model.inpos] >>> 18)
| ((model.inArray[10 + model.inpos] & 7) << (17 - 3));
  model.outArray[19 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 3) & 131071);
  model.outArray[20 + model.outpos] = (model.inArray[10 + model.inpos] >>> 20)
| ((model.inArray[11 + model.inpos] & 31) << (17 - 5));
  model.outArray[21 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 5) & 131071);
  model.outArray[22 + model.outpos] = (model.inArray[11 + model.inpos] >>> 22)
| ((model.inArray[12 + model.inpos] & 127) << (17 - 7));
  model.outArray[23 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 7) & 131071);
  model.outArray[24 + model.outpos] = (model.inArray[12 + model.inpos] >>> 24)
| ((model.inArray[13 + model.inpos] & 511) << (17 - 9));
  model.outArray[25 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 9) & 131071);
  model.outArray[26 + model.outpos] = (model.inArray[13 + model.inpos] >>> 26)
| ((model.inArray[14 + model.inpos] & 2047) << (17 - 11));
  model.outArray[27 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 11) & 131071);
  model.outArray[28 + model.outpos] = (model.inArray[14 + model.inpos] >>> 28)
| ((model.inArray[15 + model.inpos] & 8191) << (17 - 13));
  model.outArray[29 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 13) & 131071);
  model.outArray[30 + model.outpos] = (model.inArray[15 + model.inpos] >>> 30)
| ((model.inArray[16 + model.inpos] & 32767) << (17 - 15));
  model.outArray[31 + model.outpos] = (model.inArray[16 + model.inpos] >>> 15);
}

function fastunpack18(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 262143);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 18)
| ((model.inArray[1 + model.inpos] & 15) << (18 - 4));
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 262143);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 22)
| ((model.inArray[2 + model.inpos] & 255) << (18 - 8));
  model.outArray[4 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 262143);
  model.outArray[5 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26)
| ((model.inArray[3 + model.inpos] & 4095) << (18 - 12));
  model.outArray[6 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 12) & 262143);
  model.outArray[7 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 65535) << (18 - 16));
  model.outArray[8 + model.outpos] = (model.inArray[4 + model.inpos] >>> 16)
| ((model.inArray[5 + model.inpos] & 3) << (18 - 2));
  model.outArray[9 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 2) & 262143);
  model.outArray[10 + model.outpos] = (model.inArray[5 + model.inpos] >>> 20)
| ((model.inArray[6 + model.inpos] & 63) << (18 - 6));
  model.outArray[11 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 6) & 262143);
  model.outArray[12 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24)
| ((model.inArray[7 + model.inpos] & 1023) << (18 - 10));
  model.outArray[13 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 10) & 262143);
  model.outArray[14 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 16383) << (18 - 14));
  model.outArray[15 + model.outpos] = (model.inArray[8 + model.inpos] >>> 14);
  model.outArray[16 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 0) & 262143);
  model.outArray[17 + model.outpos] = (model.inArray[9 + model.inpos] >>> 18)
| ((model.inArray[10 + model.inpos] & 15) << (18 - 4));
  model.outArray[18 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 4) & 262143);
  model.outArray[19 + model.outpos] = (model.inArray[10 + model.inpos] >>> 22)
| ((model.inArray[11 + model.inpos] & 255) << (18 - 8));
  model.outArray[20 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 8) & 262143);
  model.outArray[21 + model.outpos] = (model.inArray[11 + model.inpos] >>> 26)
| ((model.inArray[12 + model.inpos] & 4095) << (18 - 12));
  model.outArray[22 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 12) & 262143);
  model.outArray[23 + model.outpos] = (model.inArray[12 + model.inpos] >>> 30)
| ((model.inArray[13 + model.inpos] & 65535) << (18 - 16));
  model.outArray[24 + model.outpos] = (model.inArray[13 + model.inpos] >>> 16)
| ((model.inArray[14 + model.inpos] & 3) << (18 - 2));
  model.outArray[25 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 2) & 262143);
  model.outArray[26 + model.outpos] = (model.inArray[14 + model.inpos] >>> 20)
| ((model.inArray[15 + model.inpos] & 63) << (18 - 6));
  model.outArray[27 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 6) & 262143);
  model.outArray[28 + model.outpos] = (model.inArray[15 + model.inpos] >>> 24)
| ((model.inArray[16 + model.inpos] & 1023) << (18 - 10));
  model.outArray[29 + model.outpos] = ((model.inArray[16 + model.inpos] >>> 10) & 262143);
  model.outArray[30 + model.outpos] = (model.inArray[16 + model.inpos] >>> 28)
| ((model.inArray[17 + model.inpos] & 16383) << (18 - 14));
  model.outArray[31 + model.outpos] = (model.inArray[17 + model.inpos] >>> 14);
}

function fastunpack19(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 524287);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 19)
| ((model.inArray[1 + model.inpos] & 63) << (19 - 6));
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 6) & 524287);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 25)
| ((model.inArray[2 + model.inpos] & 4095) << (19 - 12));
  model.outArray[4 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 12) & 524287);
  model.outArray[5 + model.outpos] = (model.inArray[2 + model.inpos] >>> 31)
| ((model.inArray[3 + model.inpos] & 262143) << (19 - 18));
  model.outArray[6 + model.outpos] = (model.inArray[3 + model.inpos] >>> 18)
| ((model.inArray[4 + model.inpos] & 31) << (19 - 5));
  model.outArray[7 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 5) & 524287);
  model.outArray[8 + model.outpos] = (model.inArray[4 + model.inpos] >>> 24)
| ((model.inArray[5 + model.inpos] & 2047) << (19 - 11));
  model.outArray[9 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 11) & 524287);
  model.outArray[10 + model.outpos] = (model.inArray[5 + model.inpos] >>> 30)
| ((model.inArray[6 + model.inpos] & 131071) << (19 - 17));
  model.outArray[11 + model.outpos] = (model.inArray[6 + model.inpos] >>> 17)
| ((model.inArray[7 + model.inpos] & 15) << (19 - 4));
  model.outArray[12 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 4) & 524287);
  model.outArray[13 + model.outpos] = (model.inArray[7 + model.inpos] >>> 23)
| ((model.inArray[8 + model.inpos] & 1023) << (19 - 10));
  model.outArray[14 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 10) & 524287);
  model.outArray[15 + model.outpos] = (model.inArray[8 + model.inpos] >>> 29)
| ((model.inArray[9 + model.inpos] & 65535) << (19 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[9 + model.inpos] >>> 16)
| ((model.inArray[10 + model.inpos] & 7) << (19 - 3));
  model.outArray[17 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 3) & 524287);
  model.outArray[18 + model.outpos] = (model.inArray[10 + model.inpos] >>> 22)
| ((model.inArray[11 + model.inpos] & 511) << (19 - 9));
  model.outArray[19 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 9) & 524287);
  model.outArray[20 + model.outpos] = (model.inArray[11 + model.inpos] >>> 28)
| ((model.inArray[12 + model.inpos] & 32767) << (19 - 15));
  model.outArray[21 + model.outpos] = (model.inArray[12 + model.inpos] >>> 15)
| ((model.inArray[13 + model.inpos] & 3) << (19 - 2));
  model.outArray[22 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 2) & 524287);
  model.outArray[23 + model.outpos] = (model.inArray[13 + model.inpos] >>> 21)
| ((model.inArray[14 + model.inpos] & 255) << (19 - 8));
  model.outArray[24 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 8) & 524287);
  model.outArray[25 + model.outpos] = (model.inArray[14 + model.inpos] >>> 27)
| ((model.inArray[15 + model.inpos] & 16383) << (19 - 14));
  model.outArray[26 + model.outpos] = (model.inArray[15 + model.inpos] >>> 14)
| ((model.inArray[16 + model.inpos] & 1) << (19 - 1));
  model.outArray[27 + model.outpos] = ((model.inArray[16 + model.inpos] >>> 1) & 524287);
  model.outArray[28 + model.outpos] = (model.inArray[16 + model.inpos] >>> 20)
| ((model.inArray[17 + model.inpos] & 127) << (19 - 7));
  model.outArray[29 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 7) & 524287);
  model.outArray[30 + model.outpos] = (model.inArray[17 + model.inpos] >>> 26)
| ((model.inArray[18 + model.inpos] & 8191) << (19 - 13));
  model.outArray[31 + model.outpos] = (model.inArray[18 + model.inpos] >>> 13);
}

function fastunpack2(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 3);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 2) & 3);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 4) & 3);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 6) & 3);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 8) & 3);
  model.outArray[5 + model.outpos] = ((model.inArray[model.inpos] >>> 10) & 3);
  model.outArray[6 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 3);
  model.outArray[7 + model.outpos] = ((model.inArray[model.inpos] >>> 14) & 3);
  model.outArray[8 + model.outpos] = ((model.inArray[model.inpos] >>> 16) & 3);
  model.outArray[9 + model.outpos] = ((model.inArray[model.inpos] >>> 18) & 3);
  model.outArray[10 + model.outpos] = ((model.inArray[model.inpos] >>> 20) & 3);
  model.outArray[11 + model.outpos] = ((model.inArray[model.inpos] >>> 22) & 3);
  model.outArray[12 + model.outpos] = ((model.inArray[model.inpos] >>> 24) & 3);
  model.outArray[13 + model.outpos] = ((model.inArray[model.inpos] >>> 26) & 3);
  model.outArray[14 + model.outpos] = ((model.inArray[model.inpos] >>> 28) & 3);
  model.outArray[15 + model.outpos] = (model.inArray[model.inpos] >>> 30);
  model.outArray[16 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 0) & 3);
  model.outArray[17 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 2) & 3);
  model.outArray[18 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 3);
  model.outArray[19 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 6) & 3);
  model.outArray[20 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 3);
  model.outArray[21 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 3);
  model.outArray[22 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 12) & 3);
  model.outArray[23 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 14) & 3);
  model.outArray[24 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 3);
  model.outArray[25 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 18) & 3);
  model.outArray[26 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 20) & 3);
  model.outArray[27 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 22) & 3);
  model.outArray[28 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 24) & 3);
  model.outArray[29 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 26) & 3);
  model.outArray[30 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 28) & 3);
  model.outArray[31 + model.outpos] = (model.inArray[1 + model.inpos] >>> 30);
}

function fastunpack20(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 1048575);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 20)
| ((model.inArray[1 + model.inpos] & 255) << (20 - 8));
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 1048575);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 65535) << (20 - 16));
  model.outArray[4 + model.outpos] = (model.inArray[2 + model.inpos] >>> 16)
| ((model.inArray[3 + model.inpos] & 15) << (20 - 4));
  model.outArray[5 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 4) & 1048575);
  model.outArray[6 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 4095) << (20 - 12));
  model.outArray[7 + model.outpos] = (model.inArray[4 + model.inpos] >>> 12);
  model.outArray[8 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 0) & 1048575);
  model.outArray[9 + model.outpos] = (model.inArray[5 + model.inpos] >>> 20)
| ((model.inArray[6 + model.inpos] & 255) << (20 - 8));
  model.outArray[10 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 8) & 1048575);
  model.outArray[11 + model.outpos] = (model.inArray[6 + model.inpos] >>> 28)
| ((model.inArray[7 + model.inpos] & 65535) << (20 - 16));
  model.outArray[12 + model.outpos] = (model.inArray[7 + model.inpos] >>> 16)
| ((model.inArray[8 + model.inpos] & 15) << (20 - 4));
  model.outArray[13 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 4) & 1048575);
  model.outArray[14 + model.outpos] = (model.inArray[8 + model.inpos] >>> 24)
| ((model.inArray[9 + model.inpos] & 4095) << (20 - 12));
  model.outArray[15 + model.outpos] = (model.inArray[9 + model.inpos] >>> 12);
  model.outArray[16 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 0) & 1048575);
  model.outArray[17 + model.outpos] = (model.inArray[10 + model.inpos] >>> 20)
| ((model.inArray[11 + model.inpos] & 255) << (20 - 8));
  model.outArray[18 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 8) & 1048575);
  model.outArray[19 + model.outpos] = (model.inArray[11 + model.inpos] >>> 28)
| ((model.inArray[12 + model.inpos] & 65535) << (20 - 16));
  model.outArray[20 + model.outpos] = (model.inArray[12 + model.inpos] >>> 16)
| ((model.inArray[13 + model.inpos] & 15) << (20 - 4));
  model.outArray[21 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 4) & 1048575);
  model.outArray[22 + model.outpos] = (model.inArray[13 + model.inpos] >>> 24)
| ((model.inArray[14 + model.inpos] & 4095) << (20 - 12));
  model.outArray[23 + model.outpos] = (model.inArray[14 + model.inpos] >>> 12);
  model.outArray[24 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 0) & 1048575);
  model.outArray[25 + model.outpos] = (model.inArray[15 + model.inpos] >>> 20)
| ((model.inArray[16 + model.inpos] & 255) << (20 - 8));
  model.outArray[26 + model.outpos] = ((model.inArray[16 + model.inpos] >>> 8) & 1048575);
  model.outArray[27 + model.outpos] = (model.inArray[16 + model.inpos] >>> 28)
| ((model.inArray[17 + model.inpos] & 65535) << (20 - 16));
  model.outArray[28 + model.outpos] = (model.inArray[17 + model.inpos] >>> 16)
| ((model.inArray[18 + model.inpos] & 15) << (20 - 4));
  model.outArray[29 + model.outpos] = ((model.inArray[18 + model.inpos] >>> 4) & 1048575);
  model.outArray[30 + model.outpos] = (model.inArray[18 + model.inpos] >>> 24)
| ((model.inArray[19 + model.inpos] & 4095) << (20 - 12));
  model.outArray[31 + model.outpos] = (model.inArray[19 + model.inpos] >>> 12);
}

function fastunpack21(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 2097151);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 21)
| ((model.inArray[1 + model.inpos] & 1023) << (21 - 10));
  model.outArray[2 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 2097151);
  model.outArray[3 + model.outpos] = (model.inArray[1 + model.inpos] >>> 31)
| ((model.inArray[2 + model.inpos] & 1048575) << (21 - 20));
  model.outArray[4 + model.outpos] = (model.inArray[2 + model.inpos] >>> 20)
| ((model.inArray[3 + model.inpos] & 511) << (21 - 9));
  model.outArray[5 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 9) & 2097151);
  model.outArray[6 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 524287) << (21 - 19));
  model.outArray[7 + model.outpos] = (model.inArray[4 + model.inpos] >>> 19)
| ((model.inArray[5 + model.inpos] & 255) << (21 - 8));
  model.outArray[8 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 2097151);
  model.outArray[9 + model.outpos] = (model.inArray[5 + model.inpos] >>> 29)
| ((model.inArray[6 + model.inpos] & 262143) << (21 - 18));
  model.outArray[10 + model.outpos] = (model.inArray[6 + model.inpos] >>> 18)
| ((model.inArray[7 + model.inpos] & 127) << (21 - 7));
  model.outArray[11 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 7) & 2097151);
  model.outArray[12 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 131071) << (21 - 17));
  model.outArray[13 + model.outpos] = (model.inArray[8 + model.inpos] >>> 17)
| ((model.inArray[9 + model.inpos] & 63) << (21 - 6));
  model.outArray[14 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 6) & 2097151);
  model.outArray[15 + model.outpos] = (model.inArray[9 + model.inpos] >>> 27)
| ((model.inArray[10 + model.inpos] & 65535) << (21 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[10 + model.inpos] >>> 16)
| ((model.inArray[11 + model.inpos] & 31) << (21 - 5));
  model.outArray[17 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 5) & 2097151);
  model.outArray[18 + model.outpos] = (model.inArray[11 + model.inpos] >>> 26)
| ((model.inArray[12 + model.inpos] & 32767) << (21 - 15));
  model.outArray[19 + model.outpos] = (model.inArray[12 + model.inpos] >>> 15)
| ((model.inArray[13 + model.inpos] & 15) << (21 - 4));
  model.outArray[20 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 4) & 2097151);
  model.outArray[21 + model.outpos] = (model.inArray[13 + model.inpos] >>> 25)
| ((model.inArray[14 + model.inpos] & 16383) << (21 - 14));
  model.outArray[22 + model.outpos] = (model.inArray[14 + model.inpos] >>> 14)
| ((model.inArray[15 + model.inpos] & 7) << (21 - 3));
  model.outArray[23 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 3) & 2097151);
  model.outArray[24 + model.outpos] = (model.inArray[15 + model.inpos] >>> 24)
| ((model.inArray[16 + model.inpos] & 8191) << (21 - 13));
  model.outArray[25 + model.outpos] = (model.inArray[16 + model.inpos] >>> 13)
| ((model.inArray[17 + model.inpos] & 3) << (21 - 2));
  model.outArray[26 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 2) & 2097151);
  model.outArray[27 + model.outpos] = (model.inArray[17 + model.inpos] >>> 23)
| ((model.inArray[18 + model.inpos] & 4095) << (21 - 12));
  model.outArray[28 + model.outpos] = (model.inArray[18 + model.inpos] >>> 12)
| ((model.inArray[19 + model.inpos] & 1) << (21 - 1));
  model.outArray[29 + model.outpos] = ((model.inArray[19 + model.inpos] >>> 1) & 2097151);
  model.outArray[30 + model.outpos] = (model.inArray[19 + model.inpos] >>> 22)
| ((model.inArray[20 + model.inpos] & 2047) << (21 - 11));
  model.outArray[31 + model.outpos] = (model.inArray[20 + model.inpos] >>> 11);
}

function fastunpack22(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 4194303);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 22)
| ((model.inArray[1 + model.inpos] & 4095) << (22 - 12));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 12)
| ((model.inArray[2 + model.inpos] & 3) << (22 - 2));
  model.outArray[3 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 2) & 4194303);
  model.outArray[4 + model.outpos] = (model.inArray[2 + model.inpos] >>> 24)
| ((model.inArray[3 + model.inpos] & 16383) << (22 - 14));
  model.outArray[5 + model.outpos] = (model.inArray[3 + model.inpos] >>> 14)
| ((model.inArray[4 + model.inpos] & 15) << (22 - 4));
  model.outArray[6 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 4) & 4194303);
  model.outArray[7 + model.outpos] = (model.inArray[4 + model.inpos] >>> 26)
| ((model.inArray[5 + model.inpos] & 65535) << (22 - 16));
  model.outArray[8 + model.outpos] = (model.inArray[5 + model.inpos] >>> 16)
| ((model.inArray[6 + model.inpos] & 63) << (22 - 6));
  model.outArray[9 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 6) & 4194303);
  model.outArray[10 + model.outpos] = (model.inArray[6 + model.inpos] >>> 28)
| ((model.inArray[7 + model.inpos] & 262143) << (22 - 18));
  model.outArray[11 + model.outpos] = (model.inArray[7 + model.inpos] >>> 18)
| ((model.inArray[8 + model.inpos] & 255) << (22 - 8));
  model.outArray[12 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 8) & 4194303);
  model.outArray[13 + model.outpos] = (model.inArray[8 + model.inpos] >>> 30)
| ((model.inArray[9 + model.inpos] & 1048575) << (22 - 20));
  model.outArray[14 + model.outpos] = (model.inArray[9 + model.inpos] >>> 20)
| ((model.inArray[10 + model.inpos] & 1023) << (22 - 10));
  model.outArray[15 + model.outpos] = (model.inArray[10 + model.inpos] >>> 10);
  model.outArray[16 + model.outpos] = ((model.inArray[11 + model.inpos] >>> 0) & 4194303);
  model.outArray[17 + model.outpos] = (model.inArray[11 + model.inpos] >>> 22)
| ((model.inArray[12 + model.inpos] & 4095) << (22 - 12));
  model.outArray[18 + model.outpos] = (model.inArray[12 + model.inpos] >>> 12)
| ((model.inArray[13 + model.inpos] & 3) << (22 - 2));
  model.outArray[19 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 2) & 4194303);
  model.outArray[20 + model.outpos] = (model.inArray[13 + model.inpos] >>> 24)
| ((model.inArray[14 + model.inpos] & 16383) << (22 - 14));
  model.outArray[21 + model.outpos] = (model.inArray[14 + model.inpos] >>> 14)
| ((model.inArray[15 + model.inpos] & 15) << (22 - 4));
  model.outArray[22 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 4) & 4194303);
  model.outArray[23 + model.outpos] = (model.inArray[15 + model.inpos] >>> 26)
| ((model.inArray[16 + model.inpos] & 65535) << (22 - 16));
  model.outArray[24 + model.outpos] = (model.inArray[16 + model.inpos] >>> 16)
| ((model.inArray[17 + model.inpos] & 63) << (22 - 6));
  model.outArray[25 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 6) & 4194303);
  model.outArray[26 + model.outpos] = (model.inArray[17 + model.inpos] >>> 28)
| ((model.inArray[18 + model.inpos] & 262143) << (22 - 18));
  model.outArray[27 + model.outpos] = (model.inArray[18 + model.inpos] >>> 18)
| ((model.inArray[19 + model.inpos] & 255) << (22 - 8));
  model.outArray[28 + model.outpos] = ((model.inArray[19 + model.inpos] >>> 8) & 4194303);
  model.outArray[29 + model.outpos] = (model.inArray[19 + model.inpos] >>> 30)
| ((model.inArray[20 + model.inpos] & 1048575) << (22 - 20));
  model.outArray[30 + model.outpos] = (model.inArray[20 + model.inpos] >>> 20)
| ((model.inArray[21 + model.inpos] & 1023) << (22 - 10));
  model.outArray[31 + model.outpos] = (model.inArray[21 + model.inpos] >>> 10);
}

function fastunpack23(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 8388607);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 23)
| ((model.inArray[1 + model.inpos] & 16383) << (23 - 14));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 14)
| ((model.inArray[2 + model.inpos] & 31) << (23 - 5));
  model.outArray[3 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 5) & 8388607);
  model.outArray[4 + model.outpos] = (model.inArray[2 + model.inpos] >>> 28)
| ((model.inArray[3 + model.inpos] & 524287) << (23 - 19));
  model.outArray[5 + model.outpos] = (model.inArray[3 + model.inpos] >>> 19)
| ((model.inArray[4 + model.inpos] & 1023) << (23 - 10));
  model.outArray[6 + model.outpos] = (model.inArray[4 + model.inpos] >>> 10)
| ((model.inArray[5 + model.inpos] & 1) << (23 - 1));
  model.outArray[7 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 1) & 8388607);
  model.outArray[8 + model.outpos] = (model.inArray[5 + model.inpos] >>> 24)
| ((model.inArray[6 + model.inpos] & 32767) << (23 - 15));
  model.outArray[9 + model.outpos] = (model.inArray[6 + model.inpos] >>> 15)
| ((model.inArray[7 + model.inpos] & 63) << (23 - 6));
  model.outArray[10 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 6) & 8388607);
  model.outArray[11 + model.outpos] = (model.inArray[7 + model.inpos] >>> 29)
| ((model.inArray[8 + model.inpos] & 1048575) << (23 - 20));
  model.outArray[12 + model.outpos] = (model.inArray[8 + model.inpos] >>> 20)
| ((model.inArray[9 + model.inpos] & 2047) << (23 - 11));
  model.outArray[13 + model.outpos] = (model.inArray[9 + model.inpos] >>> 11)
| ((model.inArray[10 + model.inpos] & 3) << (23 - 2));
  model.outArray[14 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 2) & 8388607);
  model.outArray[15 + model.outpos] = (model.inArray[10 + model.inpos] >>> 25)
| ((model.inArray[11 + model.inpos] & 65535) << (23 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[11 + model.inpos] >>> 16)
| ((model.inArray[12 + model.inpos] & 127) << (23 - 7));
  model.outArray[17 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 7) & 8388607);
  model.outArray[18 + model.outpos] = (model.inArray[12 + model.inpos] >>> 30)
| ((model.inArray[13 + model.inpos] & 2097151) << (23 - 21));
  model.outArray[19 + model.outpos] = (model.inArray[13 + model.inpos] >>> 21)
| ((model.inArray[14 + model.inpos] & 4095) << (23 - 12));
  model.outArray[20 + model.outpos] = (model.inArray[14 + model.inpos] >>> 12)
| ((model.inArray[15 + model.inpos] & 7) << (23 - 3));
  model.outArray[21 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 3) & 8388607);
  model.outArray[22 + model.outpos] = (model.inArray[15 + model.inpos] >>> 26)
| ((model.inArray[16 + model.inpos] & 131071) << (23 - 17));
  model.outArray[23 + model.outpos] = (model.inArray[16 + model.inpos] >>> 17)
| ((model.inArray[17 + model.inpos] & 255) << (23 - 8));
  model.outArray[24 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 8) & 8388607);
  model.outArray[25 + model.outpos] = (model.inArray[17 + model.inpos] >>> 31)
| ((model.inArray[18 + model.inpos] & 4194303) << (23 - 22));
  model.outArray[26 + model.outpos] = (model.inArray[18 + model.inpos] >>> 22)
| ((model.inArray[19 + model.inpos] & 8191) << (23 - 13));
  model.outArray[27 + model.outpos] = (model.inArray[19 + model.inpos] >>> 13)
| ((model.inArray[20 + model.inpos] & 15) << (23 - 4));
  model.outArray[28 + model.outpos] = ((model.inArray[20 + model.inpos] >>> 4) & 8388607);
  model.outArray[29 + model.outpos] = (model.inArray[20 + model.inpos] >>> 27)
| ((model.inArray[21 + model.inpos] & 262143) << (23 - 18));
  model.outArray[30 + model.outpos] = (model.inArray[21 + model.inpos] >>> 18)
| ((model.inArray[22 + model.inpos] & 511) << (23 - 9));
  model.outArray[31 + model.outpos] = (model.inArray[22 + model.inpos] >>> 9);
}

function fastunpack24(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 16777215);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 24)
| ((model.inArray[1 + model.inpos] & 65535) << (24 - 16));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 16)
| ((model.inArray[2 + model.inpos] & 255) << (24 - 8));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 8);
  model.outArray[4 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 16777215);
  model.outArray[5 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 65535) << (24 - 16));
  model.outArray[6 + model.outpos] = (model.inArray[4 + model.inpos] >>> 16)
| ((model.inArray[5 + model.inpos] & 255) << (24 - 8));
  model.outArray[7 + model.outpos] = (model.inArray[5 + model.inpos] >>> 8);
  model.outArray[8 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 0) & 16777215);
  model.outArray[9 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24)
| ((model.inArray[7 + model.inpos] & 65535) << (24 - 16));
  model.outArray[10 + model.outpos] = (model.inArray[7 + model.inpos] >>> 16)
| ((model.inArray[8 + model.inpos] & 255) << (24 - 8));
  model.outArray[11 + model.outpos] = (model.inArray[8 + model.inpos] >>> 8);
  model.outArray[12 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 0) & 16777215);
  model.outArray[13 + model.outpos] = (model.inArray[9 + model.inpos] >>> 24)
| ((model.inArray[10 + model.inpos] & 65535) << (24 - 16));
  model.outArray[14 + model.outpos] = (model.inArray[10 + model.inpos] >>> 16)
| ((model.inArray[11 + model.inpos] & 255) << (24 - 8));
  model.outArray[15 + model.outpos] = (model.inArray[11 + model.inpos] >>> 8);
  model.outArray[16 + model.outpos] = ((model.inArray[12 + model.inpos] >>> 0) & 16777215);
  model.outArray[17 + model.outpos] = (model.inArray[12 + model.inpos] >>> 24)
| ((model.inArray[13 + model.inpos] & 65535) << (24 - 16));
  model.outArray[18 + model.outpos] = (model.inArray[13 + model.inpos] >>> 16)
| ((model.inArray[14 + model.inpos] & 255) << (24 - 8));
  model.outArray[19 + model.outpos] = (model.inArray[14 + model.inpos] >>> 8);
  model.outArray[20 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 0) & 16777215);
  model.outArray[21 + model.outpos] = (model.inArray[15 + model.inpos] >>> 24)
| ((model.inArray[16 + model.inpos] & 65535) << (24 - 16));
  model.outArray[22 + model.outpos] = (model.inArray[16 + model.inpos] >>> 16)
| ((model.inArray[17 + model.inpos] & 255) << (24 - 8));
  model.outArray[23 + model.outpos] = (model.inArray[17 + model.inpos] >>> 8);
  model.outArray[24 + model.outpos] = ((model.inArray[18 + model.inpos] >>> 0) & 16777215);
  model.outArray[25 + model.outpos] = (model.inArray[18 + model.inpos] >>> 24)
| ((model.inArray[19 + model.inpos] & 65535) << (24 - 16));
  model.outArray[26 + model.outpos] = (model.inArray[19 + model.inpos] >>> 16)
| ((model.inArray[20 + model.inpos] & 255) << (24 - 8));
  model.outArray[27 + model.outpos] = (model.inArray[20 + model.inpos] >>> 8);
  model.outArray[28 + model.outpos] = ((model.inArray[21 + model.inpos] >>> 0) & 16777215);
  model.outArray[29 + model.outpos] = (model.inArray[21 + model.inpos] >>> 24)
| ((model.inArray[22 + model.inpos] & 65535) << (24 - 16));
  model.outArray[30 + model.outpos] = (model.inArray[22 + model.inpos] >>> 16)
| ((model.inArray[23 + model.inpos] & 255) << (24 - 8));
  model.outArray[31 + model.outpos] = (model.inArray[23 + model.inpos] >>> 8);
}

function fastunpack25(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 33554431);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 25)
| ((model.inArray[1 + model.inpos] & 262143) << (25 - 18));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 18)
| ((model.inArray[2 + model.inpos] & 2047) << (25 - 11));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 11)
| ((model.inArray[3 + model.inpos] & 15) << (25 - 4));
  model.outArray[4 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 4) & 33554431);
  model.outArray[5 + model.outpos] = (model.inArray[3 + model.inpos] >>> 29)
| ((model.inArray[4 + model.inpos] & 4194303) << (25 - 22));
  model.outArray[6 + model.outpos] = (model.inArray[4 + model.inpos] >>> 22)
| ((model.inArray[5 + model.inpos] & 32767) << (25 - 15));
  model.outArray[7 + model.outpos] = (model.inArray[5 + model.inpos] >>> 15)
| ((model.inArray[6 + model.inpos] & 255) << (25 - 8));
  model.outArray[8 + model.outpos] = (model.inArray[6 + model.inpos] >>> 8)
| ((model.inArray[7 + model.inpos] & 1) << (25 - 1));
  model.outArray[9 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 1) & 33554431);
  model.outArray[10 + model.outpos] = (model.inArray[7 + model.inpos] >>> 26)
| ((model.inArray[8 + model.inpos] & 524287) << (25 - 19));
  model.outArray[11 + model.outpos] = (model.inArray[8 + model.inpos] >>> 19)
| ((model.inArray[9 + model.inpos] & 4095) << (25 - 12));
  model.outArray[12 + model.outpos] = (model.inArray[9 + model.inpos] >>> 12)
| ((model.inArray[10 + model.inpos] & 31) << (25 - 5));
  model.outArray[13 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 5) & 33554431);
  model.outArray[14 + model.outpos] = (model.inArray[10 + model.inpos] >>> 30)
| ((model.inArray[11 + model.inpos] & 8388607) << (25 - 23));
  model.outArray[15 + model.outpos] = (model.inArray[11 + model.inpos] >>> 23)
| ((model.inArray[12 + model.inpos] & 65535) << (25 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[12 + model.inpos] >>> 16)
| ((model.inArray[13 + model.inpos] & 511) << (25 - 9));
  model.outArray[17 + model.outpos] = (model.inArray[13 + model.inpos] >>> 9)
| ((model.inArray[14 + model.inpos] & 3) << (25 - 2));
  model.outArray[18 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 2) & 33554431);
  model.outArray[19 + model.outpos] = (model.inArray[14 + model.inpos] >>> 27)
| ((model.inArray[15 + model.inpos] & 1048575) << (25 - 20));
  model.outArray[20 + model.outpos] = (model.inArray[15 + model.inpos] >>> 20)
| ((model.inArray[16 + model.inpos] & 8191) << (25 - 13));
  model.outArray[21 + model.outpos] = (model.inArray[16 + model.inpos] >>> 13)
| ((model.inArray[17 + model.inpos] & 63) << (25 - 6));
  model.outArray[22 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 6) & 33554431);
  model.outArray[23 + model.outpos] = (model.inArray[17 + model.inpos] >>> 31)
| ((model.inArray[18 + model.inpos] & 16777215) << (25 - 24));
  model.outArray[24 + model.outpos] = (model.inArray[18 + model.inpos] >>> 24)
| ((model.inArray[19 + model.inpos] & 131071) << (25 - 17));
  model.outArray[25 + model.outpos] = (model.inArray[19 + model.inpos] >>> 17)
| ((model.inArray[20 + model.inpos] & 1023) << (25 - 10));
  model.outArray[26 + model.outpos] = (model.inArray[20 + model.inpos] >>> 10)
| ((model.inArray[21 + model.inpos] & 7) << (25 - 3));
  model.outArray[27 + model.outpos] = ((model.inArray[21 + model.inpos] >>> 3) & 33554431);
  model.outArray[28 + model.outpos] = (model.inArray[21 + model.inpos] >>> 28)
| ((model.inArray[22 + model.inpos] & 2097151) << (25 - 21));
  model.outArray[29 + model.outpos] = (model.inArray[22 + model.inpos] >>> 21)
| ((model.inArray[23 + model.inpos] & 16383) << (25 - 14));
  model.outArray[30 + model.outpos] = (model.inArray[23 + model.inpos] >>> 14)
| ((model.inArray[24 + model.inpos] & 127) << (25 - 7));
  model.outArray[31 + model.outpos] = (model.inArray[24 + model.inpos] >>> 7);
}

function fastunpack26(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 67108863);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 26)
| ((model.inArray[1 + model.inpos] & 1048575) << (26 - 20));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 20)
| ((model.inArray[2 + model.inpos] & 16383) << (26 - 14));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 14)
| ((model.inArray[3 + model.inpos] & 255) << (26 - 8));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 8)
| ((model.inArray[4 + model.inpos] & 3) << (26 - 2));
  model.outArray[5 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 2) & 67108863);
  model.outArray[6 + model.outpos] = (model.inArray[4 + model.inpos] >>> 28)
| ((model.inArray[5 + model.inpos] & 4194303) << (26 - 22));
  model.outArray[7 + model.outpos] = (model.inArray[5 + model.inpos] >>> 22)
| ((model.inArray[6 + model.inpos] & 65535) << (26 - 16));
  model.outArray[8 + model.outpos] = (model.inArray[6 + model.inpos] >>> 16)
| ((model.inArray[7 + model.inpos] & 1023) << (26 - 10));
  model.outArray[9 + model.outpos] = (model.inArray[7 + model.inpos] >>> 10)
| ((model.inArray[8 + model.inpos] & 15) << (26 - 4));
  model.outArray[10 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 4) & 67108863);
  model.outArray[11 + model.outpos] = (model.inArray[8 + model.inpos] >>> 30)
| ((model.inArray[9 + model.inpos] & 16777215) << (26 - 24));
  model.outArray[12 + model.outpos] = (model.inArray[9 + model.inpos] >>> 24)
| ((model.inArray[10 + model.inpos] & 262143) << (26 - 18));
  model.outArray[13 + model.outpos] = (model.inArray[10 + model.inpos] >>> 18)
| ((model.inArray[11 + model.inpos] & 4095) << (26 - 12));
  model.outArray[14 + model.outpos] = (model.inArray[11 + model.inpos] >>> 12)
| ((model.inArray[12 + model.inpos] & 63) << (26 - 6));
  model.outArray[15 + model.outpos] = (model.inArray[12 + model.inpos] >>> 6);
  model.outArray[16 + model.outpos] = ((model.inArray[13 + model.inpos] >>> 0) & 67108863);
  model.outArray[17 + model.outpos] = (model.inArray[13 + model.inpos] >>> 26)
| ((model.inArray[14 + model.inpos] & 1048575) << (26 - 20));
  model.outArray[18 + model.outpos] = (model.inArray[14 + model.inpos] >>> 20)
| ((model.inArray[15 + model.inpos] & 16383) << (26 - 14));
  model.outArray[19 + model.outpos] = (model.inArray[15 + model.inpos] >>> 14)
| ((model.inArray[16 + model.inpos] & 255) << (26 - 8));
  model.outArray[20 + model.outpos] = (model.inArray[16 + model.inpos] >>> 8)
| ((model.inArray[17 + model.inpos] & 3) << (26 - 2));
  model.outArray[21 + model.outpos] = ((model.inArray[17 + model.inpos] >>> 2) & 67108863);
  model.outArray[22 + model.outpos] = (model.inArray[17 + model.inpos] >>> 28)
| ((model.inArray[18 + model.inpos] & 4194303) << (26 - 22));
  model.outArray[23 + model.outpos] = (model.inArray[18 + model.inpos] >>> 22)
| ((model.inArray[19 + model.inpos] & 65535) << (26 - 16));
  model.outArray[24 + model.outpos] = (model.inArray[19 + model.inpos] >>> 16)
| ((model.inArray[20 + model.inpos] & 1023) << (26 - 10));
  model.outArray[25 + model.outpos] = (model.inArray[20 + model.inpos] >>> 10)
| ((model.inArray[21 + model.inpos] & 15) << (26 - 4));
  model.outArray[26 + model.outpos] = ((model.inArray[21 + model.inpos] >>> 4) & 67108863);
  model.outArray[27 + model.outpos] = (model.inArray[21 + model.inpos] >>> 30)
| ((model.inArray[22 + model.inpos] & 16777215) << (26 - 24));
  model.outArray[28 + model.outpos] = (model.inArray[22 + model.inpos] >>> 24)
| ((model.inArray[23 + model.inpos] & 262143) << (26 - 18));
  model.outArray[29 + model.outpos] = (model.inArray[23 + model.inpos] >>> 18)
| ((model.inArray[24 + model.inpos] & 4095) << (26 - 12));
  model.outArray[30 + model.outpos] = (model.inArray[24 + model.inpos] >>> 12)
| ((model.inArray[25 + model.inpos] & 63) << (26 - 6));
  model.outArray[31 + model.outpos] = (model.inArray[25 + model.inpos] >>> 6);
}

function fastunpack27(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 134217727);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 27)
| ((model.inArray[1 + model.inpos] & 4194303) << (27 - 22));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 22)
| ((model.inArray[2 + model.inpos] & 131071) << (27 - 17));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 17)
| ((model.inArray[3 + model.inpos] & 4095) << (27 - 12));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 12)
| ((model.inArray[4 + model.inpos] & 127) << (27 - 7));
  model.outArray[5 + model.outpos] = (model.inArray[4 + model.inpos] >>> 7)
| ((model.inArray[5 + model.inpos] & 3) << (27 - 2));
  model.outArray[6 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 2) & 134217727);
  model.outArray[7 + model.outpos] = (model.inArray[5 + model.inpos] >>> 29)
| ((model.inArray[6 + model.inpos] & 16777215) << (27 - 24));
  model.outArray[8 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24)
| ((model.inArray[7 + model.inpos] & 524287) << (27 - 19));
  model.outArray[9 + model.outpos] = (model.inArray[7 + model.inpos] >>> 19)
| ((model.inArray[8 + model.inpos] & 16383) << (27 - 14));
  model.outArray[10 + model.outpos] = (model.inArray[8 + model.inpos] >>> 14)
| ((model.inArray[9 + model.inpos] & 511) << (27 - 9));
  model.outArray[11 + model.outpos] = (model.inArray[9 + model.inpos] >>> 9)
| ((model.inArray[10 + model.inpos] & 15) << (27 - 4));
  model.outArray[12 + model.outpos] = ((model.inArray[10 + model.inpos] >>> 4) & 134217727);
  model.outArray[13 + model.outpos] = (model.inArray[10 + model.inpos] >>> 31)
| ((model.inArray[11 + model.inpos] & 67108863) << (27 - 26));
  model.outArray[14 + model.outpos] = (model.inArray[11 + model.inpos] >>> 26)
| ((model.inArray[12 + model.inpos] & 2097151) << (27 - 21));
  model.outArray[15 + model.outpos] = (model.inArray[12 + model.inpos] >>> 21)
| ((model.inArray[13 + model.inpos] & 65535) << (27 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[13 + model.inpos] >>> 16)
| ((model.inArray[14 + model.inpos] & 2047) << (27 - 11));
  model.outArray[17 + model.outpos] = (model.inArray[14 + model.inpos] >>> 11)
| ((model.inArray[15 + model.inpos] & 63) << (27 - 6));
  model.outArray[18 + model.outpos] = (model.inArray[15 + model.inpos] >>> 6)
| ((model.inArray[16 + model.inpos] & 1) << (27 - 1));
  model.outArray[19 + model.outpos] = ((model.inArray[16 + model.inpos] >>> 1) & 134217727);
  model.outArray[20 + model.outpos] = (model.inArray[16 + model.inpos] >>> 28)
| ((model.inArray[17 + model.inpos] & 8388607) << (27 - 23));
  model.outArray[21 + model.outpos] = (model.inArray[17 + model.inpos] >>> 23)
| ((model.inArray[18 + model.inpos] & 262143) << (27 - 18));
  model.outArray[22 + model.outpos] = (model.inArray[18 + model.inpos] >>> 18)
| ((model.inArray[19 + model.inpos] & 8191) << (27 - 13));
  model.outArray[23 + model.outpos] = (model.inArray[19 + model.inpos] >>> 13)
| ((model.inArray[20 + model.inpos] & 255) << (27 - 8));
  model.outArray[24 + model.outpos] = (model.inArray[20 + model.inpos] >>> 8)
| ((model.inArray[21 + model.inpos] & 7) << (27 - 3));
  model.outArray[25 + model.outpos] = ((model.inArray[21 + model.inpos] >>> 3) & 134217727);
  model.outArray[26 + model.outpos] = (model.inArray[21 + model.inpos] >>> 30)
| ((model.inArray[22 + model.inpos] & 33554431) << (27 - 25));
  model.outArray[27 + model.outpos] = (model.inArray[22 + model.inpos] >>> 25)
| ((model.inArray[23 + model.inpos] & 1048575) << (27 - 20));
  model.outArray[28 + model.outpos] = (model.inArray[23 + model.inpos] >>> 20)
| ((model.inArray[24 + model.inpos] & 32767) << (27 - 15));
  model.outArray[29 + model.outpos] = (model.inArray[24 + model.inpos] >>> 15)
| ((model.inArray[25 + model.inpos] & 1023) << (27 - 10));
  model.outArray[30 + model.outpos] = (model.inArray[25 + model.inpos] >>> 10)
| ((model.inArray[26 + model.inpos] & 31) << (27 - 5));
  model.outArray[31 + model.outpos] = (model.inArray[26 + model.inpos] >>> 5);
}

function fastunpack28(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 268435455);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 28)
| ((model.inArray[1 + model.inpos] & 16777215) << (28 - 24));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 24)
| ((model.inArray[2 + model.inpos] & 1048575) << (28 - 20));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 20)
| ((model.inArray[3 + model.inpos] & 65535) << (28 - 16));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 16)
| ((model.inArray[4 + model.inpos] & 4095) << (28 - 12));
  model.outArray[5 + model.outpos] = (model.inArray[4 + model.inpos] >>> 12)
| ((model.inArray[5 + model.inpos] & 255) << (28 - 8));
  model.outArray[6 + model.outpos] = (model.inArray[5 + model.inpos] >>> 8)
| ((model.inArray[6 + model.inpos] & 15) << (28 - 4));
  model.outArray[7 + model.outpos] = (model.inArray[6 + model.inpos] >>> 4);
  model.outArray[8 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 0) & 268435455);
  model.outArray[9 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 16777215) << (28 - 24));
  model.outArray[10 + model.outpos] = (model.inArray[8 + model.inpos] >>> 24)
| ((model.inArray[9 + model.inpos] & 1048575) << (28 - 20));
  model.outArray[11 + model.outpos] = (model.inArray[9 + model.inpos] >>> 20)
| ((model.inArray[10 + model.inpos] & 65535) << (28 - 16));
  model.outArray[12 + model.outpos] = (model.inArray[10 + model.inpos] >>> 16)
| ((model.inArray[11 + model.inpos] & 4095) << (28 - 12));
  model.outArray[13 + model.outpos] = (model.inArray[11 + model.inpos] >>> 12)
| ((model.inArray[12 + model.inpos] & 255) << (28 - 8));
  model.outArray[14 + model.outpos] = (model.inArray[12 + model.inpos] >>> 8)
| ((model.inArray[13 + model.inpos] & 15) << (28 - 4));
  model.outArray[15 + model.outpos] = (model.inArray[13 + model.inpos] >>> 4);
  model.outArray[16 + model.outpos] = ((model.inArray[14 + model.inpos] >>> 0) & 268435455);
  model.outArray[17 + model.outpos] = (model.inArray[14 + model.inpos] >>> 28)
| ((model.inArray[15 + model.inpos] & 16777215) << (28 - 24));
  model.outArray[18 + model.outpos] = (model.inArray[15 + model.inpos] >>> 24)
| ((model.inArray[16 + model.inpos] & 1048575) << (28 - 20));
  model.outArray[19 + model.outpos] = (model.inArray[16 + model.inpos] >>> 20)
| ((model.inArray[17 + model.inpos] & 65535) << (28 - 16));
  model.outArray[20 + model.outpos] = (model.inArray[17 + model.inpos] >>> 16)
| ((model.inArray[18 + model.inpos] & 4095) << (28 - 12));
  model.outArray[21 + model.outpos] = (model.inArray[18 + model.inpos] >>> 12)
| ((model.inArray[19 + model.inpos] & 255) << (28 - 8));
  model.outArray[22 + model.outpos] = (model.inArray[19 + model.inpos] >>> 8)
| ((model.inArray[20 + model.inpos] & 15) << (28 - 4));
  model.outArray[23 + model.outpos] = (model.inArray[20 + model.inpos] >>> 4);
  model.outArray[24 + model.outpos] = ((model.inArray[21 + model.inpos] >>> 0) & 268435455);
  model.outArray[25 + model.outpos] = (model.inArray[21 + model.inpos] >>> 28)
| ((model.inArray[22 + model.inpos] & 16777215) << (28 - 24));
  model.outArray[26 + model.outpos] = (model.inArray[22 + model.inpos] >>> 24)
| ((model.inArray[23 + model.inpos] & 1048575) << (28 - 20));
  model.outArray[27 + model.outpos] = (model.inArray[23 + model.inpos] >>> 20)
| ((model.inArray[24 + model.inpos] & 65535) << (28 - 16));
  model.outArray[28 + model.outpos] = (model.inArray[24 + model.inpos] >>> 16)
| ((model.inArray[25 + model.inpos] & 4095) << (28 - 12));
  model.outArray[29 + model.outpos] = (model.inArray[25 + model.inpos] >>> 12)
| ((model.inArray[26 + model.inpos] & 255) << (28 - 8));
  model.outArray[30 + model.outpos] = (model.inArray[26 + model.inpos] >>> 8)
| ((model.inArray[27 + model.inpos] & 15) << (28 - 4));
  model.outArray[31 + model.outpos] = (model.inArray[27 + model.inpos] >>> 4);
}

function fastunpack29(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 536870911);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 29)
| ((model.inArray[1 + model.inpos] & 67108863) << (29 - 26));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 26)
| ((model.inArray[2 + model.inpos] & 8388607) << (29 - 23));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 23)
| ((model.inArray[3 + model.inpos] & 1048575) << (29 - 20));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 20)
| ((model.inArray[4 + model.inpos] & 131071) << (29 - 17));
  model.outArray[5 + model.outpos] = (model.inArray[4 + model.inpos] >>> 17)
| ((model.inArray[5 + model.inpos] & 16383) << (29 - 14));
  model.outArray[6 + model.outpos] = (model.inArray[5 + model.inpos] >>> 14)
| ((model.inArray[6 + model.inpos] & 2047) << (29 - 11));
  model.outArray[7 + model.outpos] = (model.inArray[6 + model.inpos] >>> 11)
| ((model.inArray[7 + model.inpos] & 255) << (29 - 8));
  model.outArray[8 + model.outpos] = (model.inArray[7 + model.inpos] >>> 8)
| ((model.inArray[8 + model.inpos] & 31) << (29 - 5));
  model.outArray[9 + model.outpos] = (model.inArray[8 + model.inpos] >>> 5)
| ((model.inArray[9 + model.inpos] & 3) << (29 - 2));
  model.outArray[10 + model.outpos] = ((model.inArray[9 + model.inpos] >>> 2) & 536870911);
  model.outArray[11 + model.outpos] = (model.inArray[9 + model.inpos] >>> 31)
| ((model.inArray[10 + model.inpos] & 268435455) << (29 - 28));
  model.outArray[12 + model.outpos] = (model.inArray[10 + model.inpos] >>> 28)
| ((model.inArray[11 + model.inpos] & 33554431) << (29 - 25));
  model.outArray[13 + model.outpos] = (model.inArray[11 + model.inpos] >>> 25)
| ((model.inArray[12 + model.inpos] & 4194303) << (29 - 22));
  model.outArray[14 + model.outpos] = (model.inArray[12 + model.inpos] >>> 22)
| ((model.inArray[13 + model.inpos] & 524287) << (29 - 19));
  model.outArray[15 + model.outpos] = (model.inArray[13 + model.inpos] >>> 19)
| ((model.inArray[14 + model.inpos] & 65535) << (29 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[14 + model.inpos] >>> 16)
| ((model.inArray[15 + model.inpos] & 8191) << (29 - 13));
  model.outArray[17 + model.outpos] = (model.inArray[15 + model.inpos] >>> 13)
| ((model.inArray[16 + model.inpos] & 1023) << (29 - 10));
  model.outArray[18 + model.outpos] = (model.inArray[16 + model.inpos] >>> 10)
| ((model.inArray[17 + model.inpos] & 127) << (29 - 7));
  model.outArray[19 + model.outpos] = (model.inArray[17 + model.inpos] >>> 7)
| ((model.inArray[18 + model.inpos] & 15) << (29 - 4));
  model.outArray[20 + model.outpos] = (model.inArray[18 + model.inpos] >>> 4)
| ((model.inArray[19 + model.inpos] & 1) << (29 - 1));
  model.outArray[21 + model.outpos] = ((model.inArray[19 + model.inpos] >>> 1) & 536870911);
  model.outArray[22 + model.outpos] = (model.inArray[19 + model.inpos] >>> 30)
| ((model.inArray[20 + model.inpos] & 134217727) << (29 - 27));
  model.outArray[23 + model.outpos] = (model.inArray[20 + model.inpos] >>> 27)
| ((model.inArray[21 + model.inpos] & 16777215) << (29 - 24));
  model.outArray[24 + model.outpos] = (model.inArray[21 + model.inpos] >>> 24)
| ((model.inArray[22 + model.inpos] & 2097151) << (29 - 21));
  model.outArray[25 + model.outpos] = (model.inArray[22 + model.inpos] >>> 21)
| ((model.inArray[23 + model.inpos] & 262143) << (29 - 18));
  model.outArray[26 + model.outpos] = (model.inArray[23 + model.inpos] >>> 18)
| ((model.inArray[24 + model.inpos] & 32767) << (29 - 15));
  model.outArray[27 + model.outpos] = (model.inArray[24 + model.inpos] >>> 15)
| ((model.inArray[25 + model.inpos] & 4095) << (29 - 12));
  model.outArray[28 + model.outpos] = (model.inArray[25 + model.inpos] >>> 12)
| ((model.inArray[26 + model.inpos] & 511) << (29 - 9));
  model.outArray[29 + model.outpos] = (model.inArray[26 + model.inpos] >>> 9)
| ((model.inArray[27 + model.inpos] & 63) << (29 - 6));
  model.outArray[30 + model.outpos] = (model.inArray[27 + model.inpos] >>> 6)
| ((model.inArray[28 + model.inpos] & 7) << (29 - 3));
  model.outArray[31 + model.outpos] = (model.inArray[28 + model.inpos] >>> 3);
}

function fastunpack3(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 7);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 3) & 7);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 6) & 7);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 9) & 7);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 7);
  model.outArray[5 + model.outpos] = ((model.inArray[model.inpos] >>> 15) & 7);
  model.outArray[6 + model.outpos] = ((model.inArray[model.inpos] >>> 18) & 7);
  model.outArray[7 + model.outpos] = ((model.inArray[model.inpos] >>> 21) & 7);
  model.outArray[8 + model.outpos] = ((model.inArray[model.inpos] >>> 24) & 7);
  model.outArray[9 + model.outpos] = ((model.inArray[model.inpos] >>> 27) & 7);
  model.outArray[10 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 1) << (3 - 1));
  model.outArray[11 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 1) & 7);
  model.outArray[12 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 7);
  model.outArray[13 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 7) & 7);
  model.outArray[14 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 7);
  model.outArray[15 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 13) & 7);
  model.outArray[16 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 7);
  model.outArray[17 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 19) & 7);
  model.outArray[18 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 22) & 7);
  model.outArray[19 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 25) & 7);
  model.outArray[20 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 28) & 7);
  model.outArray[21 + model.outpos] = (model.inArray[1 + model.inpos] >>> 31)
| ((model.inArray[2 + model.inpos] & 3) << (3 - 2));
  model.outArray[22 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 2) & 7);
  model.outArray[23 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 5) & 7);
  model.outArray[24 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 7);
  model.outArray[25 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 11) & 7);
  model.outArray[26 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 14) & 7);
  model.outArray[27 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 17) & 7);
  model.outArray[28 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 20) & 7);
  model.outArray[29 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 23) & 7);
  model.outArray[30 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 26) & 7);
  model.outArray[31 + model.outpos] = (model.inArray[2 + model.inpos] >>> 29);
}

function fastunpack30(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 1073741823);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 268435455) << (30 - 28));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 67108863) << (30 - 26));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26)
| ((model.inArray[3 + model.inpos] & 16777215) << (30 - 24));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24)
| ((model.inArray[4 + model.inpos] & 4194303) << (30 - 22));
  model.outArray[5 + model.outpos] = (model.inArray[4 + model.inpos] >>> 22)
| ((model.inArray[5 + model.inpos] & 1048575) << (30 - 20));
  model.outArray[6 + model.outpos] = (model.inArray[5 + model.inpos] >>> 20)
| ((model.inArray[6 + model.inpos] & 262143) << (30 - 18));
  model.outArray[7 + model.outpos] = (model.inArray[6 + model.inpos] >>> 18)
| ((model.inArray[7 + model.inpos] & 65535) << (30 - 16));
  model.outArray[8 + model.outpos] = (model.inArray[7 + model.inpos] >>> 16)
| ((model.inArray[8 + model.inpos] & 16383) << (30 - 14));
  model.outArray[9 + model.outpos] = (model.inArray[8 + model.inpos] >>> 14)
| ((model.inArray[9 + model.inpos] & 4095) << (30 - 12));
  model.outArray[10 + model.outpos] = (model.inArray[9 + model.inpos] >>> 12)
| ((model.inArray[10 + model.inpos] & 1023) << (30 - 10));
  model.outArray[11 + model.outpos] = (model.inArray[10 + model.inpos] >>> 10)
| ((model.inArray[11 + model.inpos] & 255) << (30 - 8));
  model.outArray[12 + model.outpos] = (model.inArray[11 + model.inpos] >>> 8)
| ((model.inArray[12 + model.inpos] & 63) << (30 - 6));
  model.outArray[13 + model.outpos] = (model.inArray[12 + model.inpos] >>> 6)
| ((model.inArray[13 + model.inpos] & 15) << (30 - 4));
  model.outArray[14 + model.outpos] = (model.inArray[13 + model.inpos] >>> 4)
| ((model.inArray[14 + model.inpos] & 3) << (30 - 2));
  model.outArray[15 + model.outpos] = (model.inArray[14 + model.inpos] >>> 2);
  model.outArray[16 + model.outpos] = ((model.inArray[15 + model.inpos] >>> 0) & 1073741823);
  model.outArray[17 + model.outpos] = (model.inArray[15 + model.inpos] >>> 30)
| ((model.inArray[16 + model.inpos] & 268435455) << (30 - 28));
  model.outArray[18 + model.outpos] = (model.inArray[16 + model.inpos] >>> 28)
| ((model.inArray[17 + model.inpos] & 67108863) << (30 - 26));
  model.outArray[19 + model.outpos] = (model.inArray[17 + model.inpos] >>> 26)
| ((model.inArray[18 + model.inpos] & 16777215) << (30 - 24));
  model.outArray[20 + model.outpos] = (model.inArray[18 + model.inpos] >>> 24)
| ((model.inArray[19 + model.inpos] & 4194303) << (30 - 22));
  model.outArray[21 + model.outpos] = (model.inArray[19 + model.inpos] >>> 22)
| ((model.inArray[20 + model.inpos] & 1048575) << (30 - 20));
  model.outArray[22 + model.outpos] = (model.inArray[20 + model.inpos] >>> 20)
| ((model.inArray[21 + model.inpos] & 262143) << (30 - 18));
  model.outArray[23 + model.outpos] = (model.inArray[21 + model.inpos] >>> 18)
| ((model.inArray[22 + model.inpos] & 65535) << (30 - 16));
  model.outArray[24 + model.outpos] = (model.inArray[22 + model.inpos] >>> 16)
| ((model.inArray[23 + model.inpos] & 16383) << (30 - 14));
  model.outArray[25 + model.outpos] = (model.inArray[23 + model.inpos] >>> 14)
| ((model.inArray[24 + model.inpos] & 4095) << (30 - 12));
  model.outArray[26 + model.outpos] = (model.inArray[24 + model.inpos] >>> 12)
| ((model.inArray[25 + model.inpos] & 1023) << (30 - 10));
  model.outArray[27 + model.outpos] = (model.inArray[25 + model.inpos] >>> 10)
| ((model.inArray[26 + model.inpos] & 255) << (30 - 8));
  model.outArray[28 + model.outpos] = (model.inArray[26 + model.inpos] >>> 8)
| ((model.inArray[27 + model.inpos] & 63) << (30 - 6));
  model.outArray[29 + model.outpos] = (model.inArray[27 + model.inpos] >>> 6)
| ((model.inArray[28 + model.inpos] & 15) << (30 - 4));
  model.outArray[30 + model.outpos] = (model.inArray[28 + model.inpos] >>> 4)
| ((model.inArray[29 + model.inpos] & 3) << (30 - 2));
  model.outArray[31 + model.outpos] = (model.inArray[29 + model.inpos] >>> 2);
}

function fastunpack31(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 2147483647);
  model.outArray[1 + model.outpos] = (model.inArray[model.inpos] >>> 31)
| ((model.inArray[1 + model.inpos] & 1073741823) << (31 - 30));
  model.outArray[2 + model.outpos] = (model.inArray[1 + model.inpos] >>> 30)
| ((model.inArray[2 + model.inpos] & 536870911) << (31 - 29));
  model.outArray[3 + model.outpos] = (model.inArray[2 + model.inpos] >>> 29)
| ((model.inArray[3 + model.inpos] & 268435455) << (31 - 28));
  model.outArray[4 + model.outpos] = (model.inArray[3 + model.inpos] >>> 28)
| ((model.inArray[4 + model.inpos] & 134217727) << (31 - 27));
  model.outArray[5 + model.outpos] = (model.inArray[4 + model.inpos] >>> 27)
| ((model.inArray[5 + model.inpos] & 67108863) << (31 - 26));
  model.outArray[6 + model.outpos] = (model.inArray[5 + model.inpos] >>> 26)
| ((model.inArray[6 + model.inpos] & 33554431) << (31 - 25));
  model.outArray[7 + model.outpos] = (model.inArray[6 + model.inpos] >>> 25)
| ((model.inArray[7 + model.inpos] & 16777215) << (31 - 24));
  model.outArray[8 + model.outpos] = (model.inArray[7 + model.inpos] >>> 24)
| ((model.inArray[8 + model.inpos] & 8388607) << (31 - 23));
  model.outArray[9 + model.outpos] = (model.inArray[8 + model.inpos] >>> 23)
| ((model.inArray[9 + model.inpos] & 4194303) << (31 - 22));
  model.outArray[10 + model.outpos] = (model.inArray[9 + model.inpos] >>> 22)
| ((model.inArray[10 + model.inpos] & 2097151) << (31 - 21));
  model.outArray[11 + model.outpos] = (model.inArray[10 + model.inpos] >>> 21)
| ((model.inArray[11 + model.inpos] & 1048575) << (31 - 20));
  model.outArray[12 + model.outpos] = (model.inArray[11 + model.inpos] >>> 20)
| ((model.inArray[12 + model.inpos] & 524287) << (31 - 19));
  model.outArray[13 + model.outpos] = (model.inArray[12 + model.inpos] >>> 19)
| ((model.inArray[13 + model.inpos] & 262143) << (31 - 18));
  model.outArray[14 + model.outpos] = (model.inArray[13 + model.inpos] >>> 18)
| ((model.inArray[14 + model.inpos] & 131071) << (31 - 17));
  model.outArray[15 + model.outpos] = (model.inArray[14 + model.inpos] >>> 17)
| ((model.inArray[15 + model.inpos] & 65535) << (31 - 16));
  model.outArray[16 + model.outpos] = (model.inArray[15 + model.inpos] >>> 16)
| ((model.inArray[16 + model.inpos] & 32767) << (31 - 15));
  model.outArray[17 + model.outpos] = (model.inArray[16 + model.inpos] >>> 15)
| ((model.inArray[17 + model.inpos] & 16383) << (31 - 14));
  model.outArray[18 + model.outpos] = (model.inArray[17 + model.inpos] >>> 14)
| ((model.inArray[18 + model.inpos] & 8191) << (31 - 13));
  model.outArray[19 + model.outpos] = (model.inArray[18 + model.inpos] >>> 13)
| ((model.inArray[19 + model.inpos] & 4095) << (31 - 12));
  model.outArray[20 + model.outpos] = (model.inArray[19 + model.inpos] >>> 12)
| ((model.inArray[20 + model.inpos] & 2047) << (31 - 11));
  model.outArray[21 + model.outpos] = (model.inArray[20 + model.inpos] >>> 11)
| ((model.inArray[21 + model.inpos] & 1023) << (31 - 10));
  model.outArray[22 + model.outpos] = (model.inArray[21 + model.inpos] >>> 10)
| ((model.inArray[22 + model.inpos] & 511) << (31 - 9));
  model.outArray[23 + model.outpos] = (model.inArray[22 + model.inpos] >>> 9)
| ((model.inArray[23 + model.inpos] & 255) << (31 - 8));
  model.outArray[24 + model.outpos] = (model.inArray[23 + model.inpos] >>> 8)
| ((model.inArray[24 + model.inpos] & 127) << (31 - 7));
  model.outArray[25 + model.outpos] = (model.inArray[24 + model.inpos] >>> 7)
| ((model.inArray[25 + model.inpos] & 63) << (31 - 6));
  model.outArray[26 + model.outpos] = (model.inArray[25 + model.inpos] >>> 6)
| ((model.inArray[26 + model.inpos] & 31) << (31 - 5));
  model.outArray[27 + model.outpos] = (model.inArray[26 + model.inpos] >>> 5)
| ((model.inArray[27 + model.inpos] & 15) << (31 - 4));
  model.outArray[28 + model.outpos] = (model.inArray[27 + model.inpos] >>> 4)
| ((model.inArray[28 + model.inpos] & 7) << (31 - 3));
  model.outArray[29 + model.outpos] = (model.inArray[28 + model.inpos] >>> 3)
| ((model.inArray[29 + model.inpos] & 3) << (31 - 2));
  model.outArray[30 + model.outpos] = (model.inArray[29 + model.inpos] >>> 2)
| ((model.inArray[30 + model.inpos] & 1) << (31 - 1));
  model.outArray[31 + model.outpos] = (model.inArray[30 + model.inpos] >>> 1);
}

function fastunpack32(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  for (let i=0; i < 32; i++)
    model.outArray[model.outpos + i] = model.inArray[model.inpos + i];
}

function fastunpack4(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 15);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 4) & 15);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 8) & 15);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 15);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 16) & 15);
  model.outArray[5 + model.outpos] = ((model.inArray[model.inpos] >>> 20) & 15);
  model.outArray[6 + model.outpos] = ((model.inArray[model.inpos] >>> 24) & 15);
  model.outArray[7 + model.outpos] = (model.inArray[model.inpos] >>> 28);
  model.outArray[8 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 0) & 15);
  model.outArray[9 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 15);
  model.outArray[10 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 15);
  model.outArray[11 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 12) & 15);
  model.outArray[12 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 15);
  model.outArray[13 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 20) & 15);
  model.outArray[14 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 24) & 15);
  model.outArray[15 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28);
  model.outArray[16 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 0) & 15);
  model.outArray[17 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 4) & 15);
  model.outArray[18 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 15);
  model.outArray[19 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 12) & 15);
  model.outArray[20 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 16) & 15);
  model.outArray[21 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 20) & 15);
  model.outArray[22 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 24) & 15);
  model.outArray[23 + model.outpos] = (model.inArray[2 + model.inpos] >>> 28);
  model.outArray[24 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 15);
  model.outArray[25 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 4) & 15);
  model.outArray[26 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 8) & 15);
  model.outArray[27 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 12) & 15);
  model.outArray[28 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 16) & 15);
  model.outArray[29 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 20) & 15);
  model.outArray[30 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 24) & 15);
  model.outArray[31 + model.outpos] = (model.inArray[3 + model.inpos] >>> 28);
}

function fastunpack5(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 31);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 5) & 31);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 10) & 31);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 15) & 31);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 20) & 31);
  model.outArray[5 + model.outpos] = ((model.inArray[model.inpos] >>> 25) & 31);
  model.outArray[6 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 7) << (5 - 3));
  model.outArray[7 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 3) & 31);
  model.outArray[8 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 31);
  model.outArray[9 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 13) & 31);
  model.outArray[10 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 18) & 31);
  model.outArray[11 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 23) & 31);
  model.outArray[12 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 1) << (5 - 1));
  model.outArray[13 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 1) & 31);
  model.outArray[14 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 6) & 31);
  model.outArray[15 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 11) & 31);
  model.outArray[16 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 16) & 31);
  model.outArray[17 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 21) & 31);
  model.outArray[18 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 26) & 31);
  model.outArray[19 + model.outpos] = (model.inArray[2 + model.inpos] >>> 31)
| ((model.inArray[3 + model.inpos] & 15) << (5 - 4));
  model.outArray[20 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 4) & 31);
  model.outArray[21 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 9) & 31);
  model.outArray[22 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 14) & 31);
  model.outArray[23 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 19) & 31);
  model.outArray[24 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 24) & 31);
  model.outArray[25 + model.outpos] = (model.inArray[3 + model.inpos] >>> 29)
| ((model.inArray[4 + model.inpos] & 3) << (5 - 2));
  model.outArray[26 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 2) & 31);
  model.outArray[27 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 7) & 31);
  model.outArray[28 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 12) & 31);
  model.outArray[29 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 17) & 31);
  model.outArray[30 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 22) & 31);
  model.outArray[31 + model.outpos] = (model.inArray[4 + model.inpos] >>> 27);
}

function fastunpack6(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 63);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 6) & 63);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 12) & 63);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 18) & 63);
  model.outArray[4 + model.outpos] = ((model.inArray[model.inpos] >>> 24) & 63);
  model.outArray[5 + model.outpos] = (model.inArray[model.inpos] >>> 30)
| ((model.inArray[1 + model.inpos] & 15) << (6 - 4));
  model.outArray[6 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 63);
  model.outArray[7 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 63);
  model.outArray[8 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 63);
  model.outArray[9 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 22) & 63);
  model.outArray[10 + model.outpos] = (model.inArray[1 + model.inpos] >>> 28)
| ((model.inArray[2 + model.inpos] & 3) << (6 - 2));
  model.outArray[11 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 2) & 63);
  model.outArray[12 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 63);
  model.outArray[13 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 14) & 63);
  model.outArray[14 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 20) & 63);
  model.outArray[15 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26);
  model.outArray[16 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 63);
  model.outArray[17 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 6) & 63);
  model.outArray[18 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 12) & 63);
  model.outArray[19 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 18) & 63);
  model.outArray[20 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 24) & 63);
  model.outArray[21 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 15) << (6 - 4));
  model.outArray[22 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 4) & 63);
  model.outArray[23 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 10) & 63);
  model.outArray[24 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 16) & 63);
  model.outArray[25 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 22) & 63);
  model.outArray[26 + model.outpos] = (model.inArray[4 + model.inpos] >>> 28)
| ((model.inArray[5 + model.inpos] & 3) << (6 - 2));
  model.outArray[27 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 2) & 63);
  model.outArray[28 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 63);
  model.outArray[29 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 14) & 63);
  model.outArray[30 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 20) & 63);
  model.outArray[31 + model.outpos] = (model.inArray[5 + model.inpos] >>> 26);
}

function fastunpack7(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 127);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 7) & 127);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 14) & 127);
  model.outArray[3 + model.outpos] = ((model.inArray[model.inpos] >>> 21) & 127);
  model.outArray[4 + model.outpos] = (model.inArray[model.inpos] >>> 28)
| ((model.inArray[1 + model.inpos] & 7) << (7 - 3));
  model.outArray[5 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 3) & 127);
  model.outArray[6 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 10) & 127);
  model.outArray[7 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 17) & 127);
  model.outArray[8 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 24) & 127);
  model.outArray[9 + model.outpos] = (model.inArray[1 + model.inpos] >>> 31)
| ((model.inArray[2 + model.inpos] & 63) << (7 - 6));
  model.outArray[10 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 6) & 127);
  model.outArray[11 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 13) & 127);
  model.outArray[12 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 20) & 127);
  model.outArray[13 + model.outpos] = (model.inArray[2 + model.inpos] >>> 27)
| ((model.inArray[3 + model.inpos] & 3) << (7 - 2));
  model.outArray[14 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 2) & 127);
  model.outArray[15 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 9) & 127);
  model.outArray[16 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 16) & 127);
  model.outArray[17 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 23) & 127);
  model.outArray[18 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 31) << (7 - 5));
  model.outArray[19 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 5) & 127);
  model.outArray[20 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 12) & 127);
  model.outArray[21 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 19) & 127);
  model.outArray[22 + model.outpos] = (model.inArray[4 + model.inpos] >>> 26)
| ((model.inArray[5 + model.inpos] & 1) << (7 - 1));
  model.outArray[23 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 1) & 127);
  model.outArray[24 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 127);
  model.outArray[25 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 15) & 127);
  model.outArray[26 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 22) & 127);
  model.outArray[27 + model.outpos] = (model.inArray[5 + model.inpos] >>> 29)
| ((model.inArray[6 + model.inpos] & 15) << (7 - 4));
  model.outArray[28 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 4) & 127);
  model.outArray[29 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 11) & 127);
  model.outArray[30 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 18) & 127);
  model.outArray[31 + model.outpos] = (model.inArray[6 + model.inpos] >>> 25);
}

function fastunpack8(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 255);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 8) & 255);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 16) & 255);
  model.outArray[3 + model.outpos] = (model.inArray[model.inpos] >>> 24);
  model.outArray[4 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 0) & 255);
  model.outArray[5 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 8) & 255);
  model.outArray[6 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 16) & 255);
  model.outArray[7 + model.outpos] = (model.inArray[1 + model.inpos] >>> 24);
  model.outArray[8 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 0) & 255);
  model.outArray[9 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 255);
  model.outArray[10 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 16) & 255);
  model.outArray[11 + model.outpos] = (model.inArray[2 + model.inpos] >>> 24);
  model.outArray[12 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 0) & 255);
  model.outArray[13 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 8) & 255);
  model.outArray[14 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 16) & 255);
  model.outArray[15 + model.outpos] = (model.inArray[3 + model.inpos] >>> 24);
  model.outArray[16 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 0) & 255);
  model.outArray[17 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 8) & 255);
  model.outArray[18 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 16) & 255);
  model.outArray[19 + model.outpos] = (model.inArray[4 + model.inpos] >>> 24);
  model.outArray[20 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 0) & 255);
  model.outArray[21 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 8) & 255);
  model.outArray[22 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 16) & 255);
  model.outArray[23 + model.outpos] = (model.inArray[5 + model.inpos] >>> 24);
  model.outArray[24 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 0) & 255);
  model.outArray[25 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 8) & 255);
  model.outArray[26 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 16) & 255);
  model.outArray[27 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24);
  model.outArray[28 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 0) & 255);
  model.outArray[29 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 8) & 255);
  model.outArray[30 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 16) & 255);
  model.outArray[31 + model.outpos] = (model.inArray[7 + model.inpos] >>> 24);
}

function fastunpack9(model: { inArray: Uint32Array, inpos: number, outArray: Uint32Array, outpos: number }) {
  model.outArray[model.outpos] = ((model.inArray[model.inpos] >>> 0) & 511);
  model.outArray[1 + model.outpos] = ((model.inArray[model.inpos] >>> 9) & 511);
  model.outArray[2 + model.outpos] = ((model.inArray[model.inpos] >>> 18) & 511);
  model.outArray[3 + model.outpos] = (model.inArray[model.inpos] >>> 27)
| ((model.inArray[1 + model.inpos] & 15) << (9 - 4));
  model.outArray[4 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 4) & 511);
  model.outArray[5 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 13) & 511);
  model.outArray[6 + model.outpos] = ((model.inArray[1 + model.inpos] >>> 22) & 511);
  model.outArray[7 + model.outpos] = (model.inArray[1 + model.inpos] >>> 31)
| ((model.inArray[2 + model.inpos] & 255) << (9 - 8));
  model.outArray[8 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 8) & 511);
  model.outArray[9 + model.outpos] = ((model.inArray[2 + model.inpos] >>> 17) & 511);
  model.outArray[10 + model.outpos] = (model.inArray[2 + model.inpos] >>> 26)
| ((model.inArray[3 + model.inpos] & 7) << (9 - 3));
  model.outArray[11 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 3) & 511);
  model.outArray[12 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 12) & 511);
  model.outArray[13 + model.outpos] = ((model.inArray[3 + model.inpos] >>> 21) & 511);
  model.outArray[14 + model.outpos] = (model.inArray[3 + model.inpos] >>> 30)
| ((model.inArray[4 + model.inpos] & 127) << (9 - 7));
  model.outArray[15 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 7) & 511);
  model.outArray[16 + model.outpos] = ((model.inArray[4 + model.inpos] >>> 16) & 511);
  model.outArray[17 + model.outpos] = (model.inArray[4 + model.inpos] >>> 25)
| ((model.inArray[5 + model.inpos] & 3) << (9 - 2));
  model.outArray[18 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 2) & 511);
  model.outArray[19 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 11) & 511);
  model.outArray[20 + model.outpos] = ((model.inArray[5 + model.inpos] >>> 20) & 511);
  model.outArray[21 + model.outpos] = (model.inArray[5 + model.inpos] >>> 29)
| ((model.inArray[6 + model.inpos] & 63) << (9 - 6));
  model.outArray[22 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 6) & 511);
  model.outArray[23 + model.outpos] = ((model.inArray[6 + model.inpos] >>> 15) & 511);
  model.outArray[24 + model.outpos] = (model.inArray[6 + model.inpos] >>> 24)
| ((model.inArray[7 + model.inpos] & 1) << (9 - 1));
  model.outArray[25 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 1) & 511);
  model.outArray[26 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 10) & 511);
  model.outArray[27 + model.outpos] = ((model.inArray[7 + model.inpos] >>> 19) & 511);
  model.outArray[28 + model.outpos] = (model.inArray[7 + model.inpos] >>> 28)
| ((model.inArray[8 + model.inpos] & 31) << (9 - 5));
  model.outArray[29 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 5) & 511);
  model.outArray[30 + model.outpos] = ((model.inArray[8 + model.inpos] >>> 14) & 511);
  model.outArray[31 + model.outpos] = (model.inArray[8 + model.inpos] >>> 23);
}
