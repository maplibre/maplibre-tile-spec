import { MASKS } from "./fastPforShared";

export function fastUnpack32_1(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    const in0 = inValues[inPos] >>> 0;
    for (let i = 0; i < 32; i++) {
        out[outPos + i] = (in0 >>> i) & 1;
    }
}

export function fastUnpack32_2(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    out[op++] = (in0 >>> 0) & 0x3;
    out[op++] = (in0 >>> 2) & 0x3;
    out[op++] = (in0 >>> 4) & 0x3;
    out[op++] = (in0 >>> 6) & 0x3;
    out[op++] = (in0 >>> 8) & 0x3;
    out[op++] = (in0 >>> 10) & 0x3;
    out[op++] = (in0 >>> 12) & 0x3;
    out[op++] = (in0 >>> 14) & 0x3;
    out[op++] = (in0 >>> 16) & 0x3;
    out[op++] = (in0 >>> 18) & 0x3;
    out[op++] = (in0 >>> 20) & 0x3;
    out[op++] = (in0 >>> 22) & 0x3;
    out[op++] = (in0 >>> 24) & 0x3;
    out[op++] = (in0 >>> 26) & 0x3;
    out[op++] = (in0 >>> 28) & 0x3;
    out[op++] = (in0 >>> 30) & 0x3;
    out[op++] = (in1 >>> 0) & 0x3;
    out[op++] = (in1 >>> 2) & 0x3;
    out[op++] = (in1 >>> 4) & 0x3;
    out[op++] = (in1 >>> 6) & 0x3;
    out[op++] = (in1 >>> 8) & 0x3;
    out[op++] = (in1 >>> 10) & 0x3;
    out[op++] = (in1 >>> 12) & 0x3;
    out[op++] = (in1 >>> 14) & 0x3;
    out[op++] = (in1 >>> 16) & 0x3;
    out[op++] = (in1 >>> 18) & 0x3;
    out[op++] = (in1 >>> 20) & 0x3;
    out[op++] = (in1 >>> 22) & 0x3;
    out[op++] = (in1 >>> 24) & 0x3;
    out[op++] = (in1 >>> 26) & 0x3;
    out[op++] = (in1 >>> 28) & 0x3;
    out[op++] = (in1 >>> 30) & 0x3;
}

export function fastUnpack32_3(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    out[op++] = (in0 >>> 0) & 0x7;
    out[op++] = (in0 >>> 3) & 0x7;
    out[op++] = (in0 >>> 6) & 0x7;
    out[op++] = (in0 >>> 9) & 0x7;
    out[op++] = (in0 >>> 12) & 0x7;
    out[op++] = (in0 >>> 15) & 0x7;
    out[op++] = (in0 >>> 18) & 0x7;
    out[op++] = (in0 >>> 21) & 0x7;
    out[op++] = (in0 >>> 24) & 0x7;
    out[op++] = (in0 >>> 27) & 0x7;
    out[op++] = ((in0 >>> 30) | ((in1 & 0x1) << 2)) & 0x7;
    out[op++] = (in1 >>> 1) & 0x7;
    out[op++] = (in1 >>> 4) & 0x7;
    out[op++] = (in1 >>> 7) & 0x7;
    out[op++] = (in1 >>> 10) & 0x7;
    out[op++] = (in1 >>> 13) & 0x7;
    out[op++] = (in1 >>> 16) & 0x7;
    out[op++] = (in1 >>> 19) & 0x7;
    out[op++] = (in1 >>> 22) & 0x7;
    out[op++] = (in1 >>> 25) & 0x7;
    out[op++] = (in1 >>> 28) & 0x7;
    out[op++] = ((in1 >>> 31) | ((in2 & 0x3) << 1)) & 0x7;
    out[op++] = (in2 >>> 2) & 0x7;
    out[op++] = (in2 >>> 5) & 0x7;
    out[op++] = (in2 >>> 8) & 0x7;
    out[op++] = (in2 >>> 11) & 0x7;
    out[op++] = (in2 >>> 14) & 0x7;
    out[op++] = (in2 >>> 17) & 0x7;
    out[op++] = (in2 >>> 20) & 0x7;
    out[op++] = (in2 >>> 23) & 0x7;
    out[op++] = (in2 >>> 26) & 0x7;
    out[op++] = (in2 >>> 29) & 0x7;
}

export function fastUnpack32_4(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    out[op++] = (in0 >>> 0) & 0xf;
    out[op++] = (in0 >>> 4) & 0xf;
    out[op++] = (in0 >>> 8) & 0xf;
    out[op++] = (in0 >>> 12) & 0xf;
    out[op++] = (in0 >>> 16) & 0xf;
    out[op++] = (in0 >>> 20) & 0xf;
    out[op++] = (in0 >>> 24) & 0xf;
    out[op++] = (in0 >>> 28) & 0xf;
    out[op++] = (in1 >>> 0) & 0xf;
    out[op++] = (in1 >>> 4) & 0xf;
    out[op++] = (in1 >>> 8) & 0xf;
    out[op++] = (in1 >>> 12) & 0xf;
    out[op++] = (in1 >>> 16) & 0xf;
    out[op++] = (in1 >>> 20) & 0xf;
    out[op++] = (in1 >>> 24) & 0xf;
    out[op++] = (in1 >>> 28) & 0xf;
    out[op++] = (in2 >>> 0) & 0xf;
    out[op++] = (in2 >>> 4) & 0xf;
    out[op++] = (in2 >>> 8) & 0xf;
    out[op++] = (in2 >>> 12) & 0xf;
    out[op++] = (in2 >>> 16) & 0xf;
    out[op++] = (in2 >>> 20) & 0xf;
    out[op++] = (in2 >>> 24) & 0xf;
    out[op++] = (in2 >>> 28) & 0xf;
    out[op++] = (in3 >>> 0) & 0xf;
    out[op++] = (in3 >>> 4) & 0xf;
    out[op++] = (in3 >>> 8) & 0xf;
    out[op++] = (in3 >>> 12) & 0xf;
    out[op++] = (in3 >>> 16) & 0xf;
    out[op++] = (in3 >>> 20) & 0xf;
    out[op++] = (in3 >>> 24) & 0xf;
    out[op++] = (in3 >>> 28) & 0xf;
}

export function fastUnpack32_5(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    out[op++] = (in0 >>> 0) & 0x1f;
    out[op++] = (in0 >>> 5) & 0x1f;
    out[op++] = (in0 >>> 10) & 0x1f;
    out[op++] = (in0 >>> 15) & 0x1f;
    out[op++] = (in0 >>> 20) & 0x1f;
    out[op++] = (in0 >>> 25) & 0x1f;
    out[op++] = ((in0 >>> 30) | ((in1 & 0x7) << 2)) & 0x1f;
    out[op++] = (in1 >>> 3) & 0x1f;
    out[op++] = (in1 >>> 8) & 0x1f;
    out[op++] = (in1 >>> 13) & 0x1f;
    out[op++] = (in1 >>> 18) & 0x1f;
    out[op++] = (in1 >>> 23) & 0x1f;
    out[op++] = ((in1 >>> 28) | ((in2 & 0x1) << 4)) & 0x1f;
    out[op++] = (in2 >>> 1) & 0x1f;
    out[op++] = (in2 >>> 6) & 0x1f;
    out[op++] = (in2 >>> 11) & 0x1f;
    out[op++] = (in2 >>> 16) & 0x1f;
    out[op++] = (in2 >>> 21) & 0x1f;
    out[op++] = (in2 >>> 26) & 0x1f;
    out[op++] = ((in2 >>> 31) | ((in3 & 0xf) << 1)) & 0x1f;
    out[op++] = (in3 >>> 4) & 0x1f;
    out[op++] = (in3 >>> 9) & 0x1f;
    out[op++] = (in3 >>> 14) & 0x1f;
    out[op++] = (in3 >>> 19) & 0x1f;
    out[op++] = (in3 >>> 24) & 0x1f;
    out[op++] = ((in3 >>> 29) | ((in4 & 0x3) << 3)) & 0x1f;
    out[op++] = (in4 >>> 2) & 0x1f;
    out[op++] = (in4 >>> 7) & 0x1f;
    out[op++] = (in4 >>> 12) & 0x1f;
    out[op++] = (in4 >>> 17) & 0x1f;
    out[op++] = (in4 >>> 22) & 0x1f;
    out[op++] = (in4 >>> 27) & 0x1f;
}

export function fastUnpack32_6(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    out[op++] = (in0 >>> 0) & 0x3f;
    out[op++] = (in0 >>> 6) & 0x3f;
    out[op++] = (in0 >>> 12) & 0x3f;
    out[op++] = (in0 >>> 18) & 0x3f;
    out[op++] = (in0 >>> 24) & 0x3f;
    out[op++] = ((in0 >>> 30) | ((in1 & 0xf) << 2)) & 0x3f;
    out[op++] = (in1 >>> 4) & 0x3f;
    out[op++] = (in1 >>> 10) & 0x3f;
    out[op++] = (in1 >>> 16) & 0x3f;
    out[op++] = (in1 >>> 22) & 0x3f;
    out[op++] = ((in1 >>> 28) | ((in2 & 0x3) << 4)) & 0x3f;
    out[op++] = (in2 >>> 2) & 0x3f;
    out[op++] = (in2 >>> 8) & 0x3f;
    out[op++] = (in2 >>> 14) & 0x3f;
    out[op++] = (in2 >>> 20) & 0x3f;
    out[op++] = (in2 >>> 26) & 0x3f;
    out[op++] = (in3 >>> 0) & 0x3f;
    out[op++] = (in3 >>> 6) & 0x3f;
    out[op++] = (in3 >>> 12) & 0x3f;
    out[op++] = (in3 >>> 18) & 0x3f;
    out[op++] = (in3 >>> 24) & 0x3f;
    out[op++] = ((in3 >>> 30) | ((in4 & 0xf) << 2)) & 0x3f;
    out[op++] = (in4 >>> 4) & 0x3f;
    out[op++] = (in4 >>> 10) & 0x3f;
    out[op++] = (in4 >>> 16) & 0x3f;
    out[op++] = (in4 >>> 22) & 0x3f;
    out[op++] = ((in4 >>> 28) | ((in5 & 0x3) << 4)) & 0x3f;
    out[op++] = (in5 >>> 2) & 0x3f;
    out[op++] = (in5 >>> 8) & 0x3f;
    out[op++] = (in5 >>> 14) & 0x3f;
    out[op++] = (in5 >>> 20) & 0x3f;
    out[op++] = (in5 >>> 26) & 0x3f;
}

export function fastUnpack32_7(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    out[op++] = (in0 >>> 0) & 0x7f;
    out[op++] = (in0 >>> 7) & 0x7f;
    out[op++] = (in0 >>> 14) & 0x7f;
    out[op++] = (in0 >>> 21) & 0x7f;
    out[op++] = ((in0 >>> 28) | ((in1 & 0x7) << 4)) & 0x7f;
    out[op++] = (in1 >>> 3) & 0x7f;
    out[op++] = (in1 >>> 10) & 0x7f;
    out[op++] = (in1 >>> 17) & 0x7f;
    out[op++] = (in1 >>> 24) & 0x7f;
    out[op++] = ((in1 >>> 31) | ((in2 & 0x3f) << 1)) & 0x7f;
    out[op++] = (in2 >>> 6) & 0x7f;
    out[op++] = (in2 >>> 13) & 0x7f;
    out[op++] = (in2 >>> 20) & 0x7f;
    out[op++] = ((in2 >>> 27) | ((in3 & 0x3) << 5)) & 0x7f;
    out[op++] = (in3 >>> 2) & 0x7f;
    out[op++] = (in3 >>> 9) & 0x7f;
    out[op++] = (in3 >>> 16) & 0x7f;
    out[op++] = (in3 >>> 23) & 0x7f;
    out[op++] = ((in3 >>> 30) | ((in4 & 0x1f) << 2)) & 0x7f;
    out[op++] = (in4 >>> 5) & 0x7f;
    out[op++] = (in4 >>> 12) & 0x7f;
    out[op++] = (in4 >>> 19) & 0x7f;
    out[op++] = ((in4 >>> 26) | ((in5 & 0x1) << 6)) & 0x7f;
    out[op++] = (in5 >>> 1) & 0x7f;
    out[op++] = (in5 >>> 8) & 0x7f;
    out[op++] = (in5 >>> 15) & 0x7f;
    out[op++] = (in5 >>> 22) & 0x7f;
    out[op++] = ((in5 >>> 29) | ((in6 & 0xf) << 3)) & 0x7f;
    out[op++] = (in6 >>> 4) & 0x7f;
    out[op++] = (in6 >>> 11) & 0x7f;
    out[op++] = (in6 >>> 18) & 0x7f;
    out[op++] = (in6 >>> 25) & 0x7f;
}

export function fastUnpack32_8(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    out[op++] = (in0 >>> 0) & 0xff;
    out[op++] = (in0 >>> 8) & 0xff;
    out[op++] = (in0 >>> 16) & 0xff;
    out[op++] = (in0 >>> 24) & 0xff;
    out[op++] = (in1 >>> 0) & 0xff;
    out[op++] = (in1 >>> 8) & 0xff;
    out[op++] = (in1 >>> 16) & 0xff;
    out[op++] = (in1 >>> 24) & 0xff;
    out[op++] = (in2 >>> 0) & 0xff;
    out[op++] = (in2 >>> 8) & 0xff;
    out[op++] = (in2 >>> 16) & 0xff;
    out[op++] = (in2 >>> 24) & 0xff;
    out[op++] = (in3 >>> 0) & 0xff;
    out[op++] = (in3 >>> 8) & 0xff;
    out[op++] = (in3 >>> 16) & 0xff;
    out[op++] = (in3 >>> 24) & 0xff;
    out[op++] = (in4 >>> 0) & 0xff;
    out[op++] = (in4 >>> 8) & 0xff;
    out[op++] = (in4 >>> 16) & 0xff;
    out[op++] = (in4 >>> 24) & 0xff;
    out[op++] = (in5 >>> 0) & 0xff;
    out[op++] = (in5 >>> 8) & 0xff;
    out[op++] = (in5 >>> 16) & 0xff;
    out[op++] = (in5 >>> 24) & 0xff;
    out[op++] = (in6 >>> 0) & 0xff;
    out[op++] = (in6 >>> 8) & 0xff;
    out[op++] = (in6 >>> 16) & 0xff;
    out[op++] = (in6 >>> 24) & 0xff;
    out[op++] = (in7 >>> 0) & 0xff;
    out[op++] = (in7 >>> 8) & 0xff;
    out[op++] = (in7 >>> 16) & 0xff;
    out[op++] = (in7 >>> 24) & 0xff;
}

export function fastUnpack32_9(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    out[op++] = (in0 >>> 0) & 0x1ff;
    out[op++] = (in0 >>> 9) & 0x1ff;
    out[op++] = (in0 >>> 18) & 0x1ff;
    out[op++] = ((in0 >>> 27) | ((in1 & 0xf) << 5)) & 0x1ff;
    out[op++] = (in1 >>> 4) & 0x1ff;
    out[op++] = (in1 >>> 13) & 0x1ff;
    out[op++] = (in1 >>> 22) & 0x1ff;
    out[op++] = ((in1 >>> 31) | ((in2 & 0xff) << 1)) & 0x1ff;
    out[op++] = (in2 >>> 8) & 0x1ff;
    out[op++] = (in2 >>> 17) & 0x1ff;
    out[op++] = ((in2 >>> 26) | ((in3 & 0x7) << 6)) & 0x1ff;
    out[op++] = (in3 >>> 3) & 0x1ff;
    out[op++] = (in3 >>> 12) & 0x1ff;
    out[op++] = (in3 >>> 21) & 0x1ff;
    out[op++] = ((in3 >>> 30) | ((in4 & 0x7f) << 2)) & 0x1ff;
    out[op++] = (in4 >>> 7) & 0x1ff;
    out[op++] = (in4 >>> 16) & 0x1ff;
    out[op++] = ((in4 >>> 25) | ((in5 & 0x3) << 7)) & 0x1ff;
    out[op++] = (in5 >>> 2) & 0x1ff;
    out[op++] = (in5 >>> 11) & 0x1ff;
    out[op++] = (in5 >>> 20) & 0x1ff;
    out[op++] = ((in5 >>> 29) | ((in6 & 0x3f) << 3)) & 0x1ff;
    out[op++] = (in6 >>> 6) & 0x1ff;
    out[op++] = (in6 >>> 15) & 0x1ff;
    out[op++] = ((in6 >>> 24) | ((in7 & 0x1) << 8)) & 0x1ff;
    out[op++] = (in7 >>> 1) & 0x1ff;
    out[op++] = (in7 >>> 10) & 0x1ff;
    out[op++] = (in7 >>> 19) & 0x1ff;
    out[op++] = ((in7 >>> 28) | ((in8 & 0x1f) << 4)) & 0x1ff;
    out[op++] = (in8 >>> 5) & 0x1ff;
    out[op++] = (in8 >>> 14) & 0x1ff;
    out[op++] = (in8 >>> 23) & 0x1ff;
}

export function fastUnpack32_10(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;
    out[op++] = (in0 >>> 0) & 0x3ff;
    out[op++] = (in0 >>> 10) & 0x3ff;
    out[op++] = (in0 >>> 20) & 0x3ff;
    out[op++] = ((in0 >>> 30) | ((in1 & 0xff) << 2)) & 0x3ff;
    out[op++] = (in1 >>> 8) & 0x3ff;
    out[op++] = (in1 >>> 18) & 0x3ff;
    out[op++] = ((in1 >>> 28) | ((in2 & 0x3f) << 4)) & 0x3ff;
    out[op++] = (in2 >>> 6) & 0x3ff;
    out[op++] = (in2 >>> 16) & 0x3ff;
    out[op++] = ((in2 >>> 26) | ((in3 & 0xf) << 6)) & 0x3ff;
    out[op++] = (in3 >>> 4) & 0x3ff;
    out[op++] = (in3 >>> 14) & 0x3ff;
    out[op++] = ((in3 >>> 24) | ((in4 & 0x3) << 8)) & 0x3ff;
    out[op++] = (in4 >>> 2) & 0x3ff;
    out[op++] = (in4 >>> 12) & 0x3ff;
    out[op++] = (in4 >>> 22) & 0x3ff;
    out[op++] = (in5 >>> 0) & 0x3ff;
    out[op++] = (in5 >>> 10) & 0x3ff;
    out[op++] = (in5 >>> 20) & 0x3ff;
    out[op++] = ((in5 >>> 30) | ((in6 & 0xff) << 2)) & 0x3ff;
    out[op++] = (in6 >>> 8) & 0x3ff;
    out[op++] = (in6 >>> 18) & 0x3ff;
    out[op++] = ((in6 >>> 28) | ((in7 & 0x3f) << 4)) & 0x3ff;
    out[op++] = (in7 >>> 6) & 0x3ff;
    out[op++] = (in7 >>> 16) & 0x3ff;
    out[op++] = ((in7 >>> 26) | ((in8 & 0xf) << 6)) & 0x3ff;
    out[op++] = (in8 >>> 4) & 0x3ff;
    out[op++] = (in8 >>> 14) & 0x3ff;
    out[op++] = ((in8 >>> 24) | ((in9 & 0x3) << 8)) & 0x3ff;
    out[op++] = (in9 >>> 2) & 0x3ff;
    out[op++] = (in9 >>> 12) & 0x3ff;
    out[op++] = (in9 >>> 22) & 0x3ff;
}

export function fastUnpack32_11(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;
    const in10 = inValues[inPos + 10] >>> 0;
    out[op++] = (in0 >>> 0) & 0x7ff;
    out[op++] = (in0 >>> 11) & 0x7ff;
    out[op++] = ((in0 >>> 22) | ((in1 & 0x1) << 10)) & 0x7ff;
    out[op++] = (in1 >>> 1) & 0x7ff;
    out[op++] = (in1 >>> 12) & 0x7ff;
    out[op++] = ((in1 >>> 23) | ((in2 & 0x3) << 9)) & 0x7ff;
    out[op++] = (in2 >>> 2) & 0x7ff;
    out[op++] = (in2 >>> 13) & 0x7ff;
    out[op++] = ((in2 >>> 24) | ((in3 & 0x7) << 8)) & 0x7ff;
    out[op++] = (in3 >>> 3) & 0x7ff;
    out[op++] = (in3 >>> 14) & 0x7ff;
    out[op++] = ((in3 >>> 25) | ((in4 & 0xf) << 7)) & 0x7ff;
    out[op++] = (in4 >>> 4) & 0x7ff;
    out[op++] = (in4 >>> 15) & 0x7ff;
    out[op++] = ((in4 >>> 26) | ((in5 & 0x1f) << 6)) & 0x7ff;
    out[op++] = (in5 >>> 5) & 0x7ff;
    out[op++] = (in5 >>> 16) & 0x7ff;
    out[op++] = ((in5 >>> 27) | ((in6 & 0x3f) << 5)) & 0x7ff;
    out[op++] = (in6 >>> 6) & 0x7ff;
    out[op++] = (in6 >>> 17) & 0x7ff;
    out[op++] = ((in6 >>> 28) | ((in7 & 0x7f) << 4)) & 0x7ff;
    out[op++] = (in7 >>> 7) & 0x7ff;
    out[op++] = (in7 >>> 18) & 0x7ff;
    out[op++] = ((in7 >>> 29) | ((in8 & 0xff) << 3)) & 0x7ff;
    out[op++] = (in8 >>> 8) & 0x7ff;
    out[op++] = (in8 >>> 19) & 0x7ff;
    out[op++] = ((in8 >>> 30) | ((in9 & 0x1ff) << 2)) & 0x7ff;
    out[op++] = (in9 >>> 9) & 0x7ff;
    out[op++] = (in9 >>> 20) & 0x7ff;
    out[op++] = ((in9 >>> 31) | ((in10 & 0x3ff) << 1)) & 0x7ff;
    out[op++] = (in10 >>> 10) & 0x7ff;
    out[op++] = (in10 >>> 21) & 0x7ff;
}

export function fastUnpack32_12(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;
    const in10 = inValues[inPos + 10] >>> 0;
    const in11 = inValues[inPos + 11] >>> 0;
    out[op++] = (in0 >>> 0) & 0xfff;
    out[op++] = (in0 >>> 12) & 0xfff;
    out[op++] = ((in0 >>> 24) | ((in1 & 0xf) << 8)) & 0xfff;
    out[op++] = (in1 >>> 4) & 0xfff;
    out[op++] = (in1 >>> 16) & 0xfff;
    out[op++] = ((in1 >>> 28) | ((in2 & 0xff) << 4)) & 0xfff;
    out[op++] = (in2 >>> 8) & 0xfff;
    out[op++] = (in2 >>> 20) & 0xfff;
    out[op++] = (in3 >>> 0) & 0xfff;
    out[op++] = (in3 >>> 12) & 0xfff;
    out[op++] = ((in3 >>> 24) | ((in4 & 0xf) << 8)) & 0xfff;
    out[op++] = (in4 >>> 4) & 0xfff;
    out[op++] = (in4 >>> 16) & 0xfff;
    out[op++] = ((in4 >>> 28) | ((in5 & 0xff) << 4)) & 0xfff;
    out[op++] = (in5 >>> 8) & 0xfff;
    out[op++] = (in5 >>> 20) & 0xfff;
    out[op++] = (in6 >>> 0) & 0xfff;
    out[op++] = (in6 >>> 12) & 0xfff;
    out[op++] = ((in6 >>> 24) | ((in7 & 0xf) << 8)) & 0xfff;
    out[op++] = (in7 >>> 4) & 0xfff;
    out[op++] = (in7 >>> 16) & 0xfff;
    out[op++] = ((in7 >>> 28) | ((in8 & 0xff) << 4)) & 0xfff;
    out[op++] = (in8 >>> 8) & 0xfff;
    out[op++] = (in8 >>> 20) & 0xfff;
    out[op++] = (in9 >>> 0) & 0xfff;
    out[op++] = (in9 >>> 12) & 0xfff;
    out[op++] = ((in9 >>> 24) | ((in10 & 0xf) << 8)) & 0xfff;
    out[op++] = (in10 >>> 4) & 0xfff;
    out[op++] = (in10 >>> 16) & 0xfff;
    out[op++] = ((in10 >>> 28) | ((in11 & 0xff) << 4)) & 0xfff;
    out[op++] = (in11 >>> 8) & 0xfff;
    out[op++] = (in11 >>> 20) & 0xfff;
}

export function fastUnpack32_16(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    const in0 = inValues[inPos] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;
    const in10 = inValues[inPos + 10] >>> 0;
    const in11 = inValues[inPos + 11] >>> 0;
    const in12 = inValues[inPos + 12] >>> 0;
    const in13 = inValues[inPos + 13] >>> 0;
    const in14 = inValues[inPos + 14] >>> 0;
    const in15 = inValues[inPos + 15] >>> 0;
    out[op++] = (in0 >>> 0) & 0xffff;
    out[op++] = (in0 >>> 16) & 0xffff;
    out[op++] = (in1 >>> 0) & 0xffff;
    out[op++] = (in1 >>> 16) & 0xffff;
    out[op++] = (in2 >>> 0) & 0xffff;
    out[op++] = (in2 >>> 16) & 0xffff;
    out[op++] = (in3 >>> 0) & 0xffff;
    out[op++] = (in3 >>> 16) & 0xffff;
    out[op++] = (in4 >>> 0) & 0xffff;
    out[op++] = (in4 >>> 16) & 0xffff;
    out[op++] = (in5 >>> 0) & 0xffff;
    out[op++] = (in5 >>> 16) & 0xffff;
    out[op++] = (in6 >>> 0) & 0xffff;
    out[op++] = (in6 >>> 16) & 0xffff;
    out[op++] = (in7 >>> 0) & 0xffff;
    out[op++] = (in7 >>> 16) & 0xffff;
    out[op++] = (in8 >>> 0) & 0xffff;
    out[op++] = (in8 >>> 16) & 0xffff;
    out[op++] = (in9 >>> 0) & 0xffff;
    out[op++] = (in9 >>> 16) & 0xffff;
    out[op++] = (in10 >>> 0) & 0xffff;
    out[op++] = (in10 >>> 16) & 0xffff;
    out[op++] = (in11 >>> 0) & 0xffff;
    out[op++] = (in11 >>> 16) & 0xffff;
    out[op++] = (in12 >>> 0) & 0xffff;
    out[op++] = (in12 >>> 16) & 0xffff;
    out[op++] = (in13 >>> 0) & 0xffff;
    out[op++] = (in13 >>> 16) & 0xffff;
    out[op++] = (in14 >>> 0) & 0xffff;
    out[op++] = (in14 >>> 16) & 0xffff;
    out[op++] = (in15 >>> 0) & 0xffff;
    out[op++] = (in15 >>> 16) & 0xffff;
}

export function fastUnpack256_1(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x1;
        out[op++] = (in0 >>> 1) & 0x1;
        out[op++] = (in0 >>> 2) & 0x1;
        out[op++] = (in0 >>> 3) & 0x1;
        out[op++] = (in0 >>> 4) & 0x1;
        out[op++] = (in0 >>> 5) & 0x1;
        out[op++] = (in0 >>> 6) & 0x1;
        out[op++] = (in0 >>> 7) & 0x1;
        out[op++] = (in0 >>> 8) & 0x1;
        out[op++] = (in0 >>> 9) & 0x1;
        out[op++] = (in0 >>> 10) & 0x1;
        out[op++] = (in0 >>> 11) & 0x1;
        out[op++] = (in0 >>> 12) & 0x1;
        out[op++] = (in0 >>> 13) & 0x1;
        out[op++] = (in0 >>> 14) & 0x1;
        out[op++] = (in0 >>> 15) & 0x1;
        out[op++] = (in0 >>> 16) & 0x1;
        out[op++] = (in0 >>> 17) & 0x1;
        out[op++] = (in0 >>> 18) & 0x1;
        out[op++] = (in0 >>> 19) & 0x1;
        out[op++] = (in0 >>> 20) & 0x1;
        out[op++] = (in0 >>> 21) & 0x1;
        out[op++] = (in0 >>> 22) & 0x1;
        out[op++] = (in0 >>> 23) & 0x1;
        out[op++] = (in0 >>> 24) & 0x1;
        out[op++] = (in0 >>> 25) & 0x1;
        out[op++] = (in0 >>> 26) & 0x1;
        out[op++] = (in0 >>> 27) & 0x1;
        out[op++] = (in0 >>> 28) & 0x1;
        out[op++] = (in0 >>> 29) & 0x1;
        out[op++] = (in0 >>> 30) & 0x1;
        out[op++] = (in0 >>> 31) & 0x1;
    }
}

export function fastUnpack256_2(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x3;
        out[op++] = (in0 >>> 2) & 0x3;
        out[op++] = (in0 >>> 4) & 0x3;
        out[op++] = (in0 >>> 6) & 0x3;
        out[op++] = (in0 >>> 8) & 0x3;
        out[op++] = (in0 >>> 10) & 0x3;
        out[op++] = (in0 >>> 12) & 0x3;
        out[op++] = (in0 >>> 14) & 0x3;
        out[op++] = (in0 >>> 16) & 0x3;
        out[op++] = (in0 >>> 18) & 0x3;
        out[op++] = (in0 >>> 20) & 0x3;
        out[op++] = (in0 >>> 22) & 0x3;
        out[op++] = (in0 >>> 24) & 0x3;
        out[op++] = (in0 >>> 26) & 0x3;
        out[op++] = (in0 >>> 28) & 0x3;
        out[op++] = (in0 >>> 30) & 0x3;
        out[op++] = (in1 >>> 0) & 0x3;
        out[op++] = (in1 >>> 2) & 0x3;
        out[op++] = (in1 >>> 4) & 0x3;
        out[op++] = (in1 >>> 6) & 0x3;
        out[op++] = (in1 >>> 8) & 0x3;
        out[op++] = (in1 >>> 10) & 0x3;
        out[op++] = (in1 >>> 12) & 0x3;
        out[op++] = (in1 >>> 14) & 0x3;
        out[op++] = (in1 >>> 16) & 0x3;
        out[op++] = (in1 >>> 18) & 0x3;
        out[op++] = (in1 >>> 20) & 0x3;
        out[op++] = (in1 >>> 22) & 0x3;
        out[op++] = (in1 >>> 24) & 0x3;
        out[op++] = (in1 >>> 26) & 0x3;
        out[op++] = (in1 >>> 28) & 0x3;
        out[op++] = (in1 >>> 30) & 0x3;
    }
}

export function fastUnpack256_3(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x7;
        out[op++] = (in0 >>> 3) & 0x7;
        out[op++] = (in0 >>> 6) & 0x7;
        out[op++] = (in0 >>> 9) & 0x7;
        out[op++] = (in0 >>> 12) & 0x7;
        out[op++] = (in0 >>> 15) & 0x7;
        out[op++] = (in0 >>> 18) & 0x7;
        out[op++] = (in0 >>> 21) & 0x7;
        out[op++] = (in0 >>> 24) & 0x7;
        out[op++] = (in0 >>> 27) & 0x7;
        out[op++] = ((in0 >>> 30) | ((in1 & 0x1) << 2)) & 0x7;
        out[op++] = (in1 >>> 1) & 0x7;
        out[op++] = (in1 >>> 4) & 0x7;
        out[op++] = (in1 >>> 7) & 0x7;
        out[op++] = (in1 >>> 10) & 0x7;
        out[op++] = (in1 >>> 13) & 0x7;
        out[op++] = (in1 >>> 16) & 0x7;
        out[op++] = (in1 >>> 19) & 0x7;
        out[op++] = (in1 >>> 22) & 0x7;
        out[op++] = (in1 >>> 25) & 0x7;
        out[op++] = (in1 >>> 28) & 0x7;
        out[op++] = ((in1 >>> 31) | ((in2 & 0x3) << 1)) & 0x7;
        out[op++] = (in2 >>> 2) & 0x7;
        out[op++] = (in2 >>> 5) & 0x7;
        out[op++] = (in2 >>> 8) & 0x7;
        out[op++] = (in2 >>> 11) & 0x7;
        out[op++] = (in2 >>> 14) & 0x7;
        out[op++] = (in2 >>> 17) & 0x7;
        out[op++] = (in2 >>> 20) & 0x7;
        out[op++] = (in2 >>> 23) & 0x7;
        out[op++] = (in2 >>> 26) & 0x7;
        out[op++] = (in2 >>> 29) & 0x7;
    }
}

export function fastUnpack256_4(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0xf;
        out[op++] = (in0 >>> 4) & 0xf;
        out[op++] = (in0 >>> 8) & 0xf;
        out[op++] = (in0 >>> 12) & 0xf;
        out[op++] = (in0 >>> 16) & 0xf;
        out[op++] = (in0 >>> 20) & 0xf;
        out[op++] = (in0 >>> 24) & 0xf;
        out[op++] = (in0 >>> 28) & 0xf;
        out[op++] = (in1 >>> 0) & 0xf;
        out[op++] = (in1 >>> 4) & 0xf;
        out[op++] = (in1 >>> 8) & 0xf;
        out[op++] = (in1 >>> 12) & 0xf;
        out[op++] = (in1 >>> 16) & 0xf;
        out[op++] = (in1 >>> 20) & 0xf;
        out[op++] = (in1 >>> 24) & 0xf;
        out[op++] = (in1 >>> 28) & 0xf;
        out[op++] = (in2 >>> 0) & 0xf;
        out[op++] = (in2 >>> 4) & 0xf;
        out[op++] = (in2 >>> 8) & 0xf;
        out[op++] = (in2 >>> 12) & 0xf;
        out[op++] = (in2 >>> 16) & 0xf;
        out[op++] = (in2 >>> 20) & 0xf;
        out[op++] = (in2 >>> 24) & 0xf;
        out[op++] = (in2 >>> 28) & 0xf;
        out[op++] = (in3 >>> 0) & 0xf;
        out[op++] = (in3 >>> 4) & 0xf;
        out[op++] = (in3 >>> 8) & 0xf;
        out[op++] = (in3 >>> 12) & 0xf;
        out[op++] = (in3 >>> 16) & 0xf;
        out[op++] = (in3 >>> 20) & 0xf;
        out[op++] = (in3 >>> 24) & 0xf;
        out[op++] = (in3 >>> 28) & 0xf;
    }
}

export function fastUnpack256_5(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        const in4 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x1f;
        out[op++] = (in0 >>> 5) & 0x1f;
        out[op++] = (in0 >>> 10) & 0x1f;
        out[op++] = (in0 >>> 15) & 0x1f;
        out[op++] = (in0 >>> 20) & 0x1f;
        out[op++] = (in0 >>> 25) & 0x1f;
        out[op++] = ((in0 >>> 30) | ((in1 & 0x7) << 2)) & 0x1f;
        out[op++] = (in1 >>> 3) & 0x1f;
        out[op++] = (in1 >>> 8) & 0x1f;
        out[op++] = (in1 >>> 13) & 0x1f;
        out[op++] = (in1 >>> 18) & 0x1f;
        out[op++] = (in1 >>> 23) & 0x1f;
        out[op++] = ((in1 >>> 28) | ((in2 & 0x1) << 4)) & 0x1f;
        out[op++] = (in2 >>> 1) & 0x1f;
        out[op++] = (in2 >>> 6) & 0x1f;
        out[op++] = (in2 >>> 11) & 0x1f;
        out[op++] = (in2 >>> 16) & 0x1f;
        out[op++] = (in2 >>> 21) & 0x1f;
        out[op++] = (in2 >>> 26) & 0x1f;
        out[op++] = ((in2 >>> 31) | ((in3 & 0xf) << 1)) & 0x1f;
        out[op++] = (in3 >>> 4) & 0x1f;
        out[op++] = (in3 >>> 9) & 0x1f;
        out[op++] = (in3 >>> 14) & 0x1f;
        out[op++] = (in3 >>> 19) & 0x1f;
        out[op++] = (in3 >>> 24) & 0x1f;
        out[op++] = ((in3 >>> 29) | ((in4 & 0x3) << 3)) & 0x1f;
        out[op++] = (in4 >>> 2) & 0x1f;
        out[op++] = (in4 >>> 7) & 0x1f;
        out[op++] = (in4 >>> 12) & 0x1f;
        out[op++] = (in4 >>> 17) & 0x1f;
        out[op++] = (in4 >>> 22) & 0x1f;
        out[op++] = (in4 >>> 27) & 0x1f;
    }
}

export function fastUnpack256_6(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        const in4 = inValues[ip++] >>> 0;
        const in5 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x3f;
        out[op++] = (in0 >>> 6) & 0x3f;
        out[op++] = (in0 >>> 12) & 0x3f;
        out[op++] = (in0 >>> 18) & 0x3f;
        out[op++] = (in0 >>> 24) & 0x3f;
        out[op++] = ((in0 >>> 30) | ((in1 & 0xf) << 2)) & 0x3f;
        out[op++] = (in1 >>> 4) & 0x3f;
        out[op++] = (in1 >>> 10) & 0x3f;
        out[op++] = (in1 >>> 16) & 0x3f;
        out[op++] = (in1 >>> 22) & 0x3f;
        out[op++] = ((in1 >>> 28) | ((in2 & 0x3) << 4)) & 0x3f;
        out[op++] = (in2 >>> 2) & 0x3f;
        out[op++] = (in2 >>> 8) & 0x3f;
        out[op++] = (in2 >>> 14) & 0x3f;
        out[op++] = (in2 >>> 20) & 0x3f;
        out[op++] = (in2 >>> 26) & 0x3f;
        out[op++] = (in3 >>> 0) & 0x3f;
        out[op++] = (in3 >>> 6) & 0x3f;
        out[op++] = (in3 >>> 12) & 0x3f;
        out[op++] = (in3 >>> 18) & 0x3f;
        out[op++] = (in3 >>> 24) & 0x3f;
        out[op++] = ((in3 >>> 30) | ((in4 & 0xf) << 2)) & 0x3f;
        out[op++] = (in4 >>> 4) & 0x3f;
        out[op++] = (in4 >>> 10) & 0x3f;
        out[op++] = (in4 >>> 16) & 0x3f;
        out[op++] = (in4 >>> 22) & 0x3f;
        out[op++] = ((in4 >>> 28) | ((in5 & 0x3) << 4)) & 0x3f;
        out[op++] = (in5 >>> 2) & 0x3f;
        out[op++] = (in5 >>> 8) & 0x3f;
        out[op++] = (in5 >>> 14) & 0x3f;
        out[op++] = (in5 >>> 20) & 0x3f;
        out[op++] = (in5 >>> 26) & 0x3f;
    }
}

export function fastUnpack256_7(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        const in4 = inValues[ip++] >>> 0;
        const in5 = inValues[ip++] >>> 0;
        const in6 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0x7f;
        out[op++] = (in0 >>> 7) & 0x7f;
        out[op++] = (in0 >>> 14) & 0x7f;
        out[op++] = (in0 >>> 21) & 0x7f;
        out[op++] = ((in0 >>> 28) | ((in1 & 0x7) << 4)) & 0x7f;
        out[op++] = (in1 >>> 3) & 0x7f;
        out[op++] = (in1 >>> 10) & 0x7f;
        out[op++] = (in1 >>> 17) & 0x7f;
        out[op++] = (in1 >>> 24) & 0x7f;
        out[op++] = ((in1 >>> 31) | ((in2 & 0x3f) << 1)) & 0x7f;
        out[op++] = (in2 >>> 6) & 0x7f;
        out[op++] = (in2 >>> 13) & 0x7f;
        out[op++] = (in2 >>> 20) & 0x7f;
        out[op++] = ((in2 >>> 27) | ((in3 & 0x3) << 5)) & 0x7f;
        out[op++] = (in3 >>> 2) & 0x7f;
        out[op++] = (in3 >>> 9) & 0x7f;
        out[op++] = (in3 >>> 16) & 0x7f;
        out[op++] = (in3 >>> 23) & 0x7f;
        out[op++] = ((in3 >>> 30) | ((in4 & 0x1f) << 2)) & 0x7f;
        out[op++] = (in4 >>> 5) & 0x7f;
        out[op++] = (in4 >>> 12) & 0x7f;
        out[op++] = (in4 >>> 19) & 0x7f;
        out[op++] = ((in4 >>> 26) | ((in5 & 0x1) << 6)) & 0x7f;
        out[op++] = (in5 >>> 1) & 0x7f;
        out[op++] = (in5 >>> 8) & 0x7f;
        out[op++] = (in5 >>> 15) & 0x7f;
        out[op++] = (in5 >>> 22) & 0x7f;
        out[op++] = ((in5 >>> 29) | ((in6 & 0xf) << 3)) & 0x7f;
        out[op++] = (in6 >>> 4) & 0x7f;
        out[op++] = (in6 >>> 11) & 0x7f;
        out[op++] = (in6 >>> 18) & 0x7f;
        out[op++] = (in6 >>> 25) & 0x7f;
    }
}

export function fastUnpack256_8(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        const in4 = inValues[ip++] >>> 0;
        const in5 = inValues[ip++] >>> 0;
        const in6 = inValues[ip++] >>> 0;
        const in7 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 0xff;
        out[op++] = (in0 >>> 8) & 0xff;
        out[op++] = (in0 >>> 16) & 0xff;
        out[op++] = (in0 >>> 24) & 0xff;
        out[op++] = (in1 >>> 0) & 0xff;
        out[op++] = (in1 >>> 8) & 0xff;
        out[op++] = (in1 >>> 16) & 0xff;
        out[op++] = (in1 >>> 24) & 0xff;
        out[op++] = (in2 >>> 0) & 0xff;
        out[op++] = (in2 >>> 8) & 0xff;
        out[op++] = (in2 >>> 16) & 0xff;
        out[op++] = (in2 >>> 24) & 0xff;
        out[op++] = (in3 >>> 0) & 0xff;
        out[op++] = (in3 >>> 8) & 0xff;
        out[op++] = (in3 >>> 16) & 0xff;
        out[op++] = (in3 >>> 24) & 0xff;
        out[op++] = (in4 >>> 0) & 0xff;
        out[op++] = (in4 >>> 8) & 0xff;
        out[op++] = (in4 >>> 16) & 0xff;
        out[op++] = (in4 >>> 24) & 0xff;
        out[op++] = (in5 >>> 0) & 0xff;
        out[op++] = (in5 >>> 8) & 0xff;
        out[op++] = (in5 >>> 16) & 0xff;
        out[op++] = (in5 >>> 24) & 0xff;
        out[op++] = (in6 >>> 0) & 0xff;
        out[op++] = (in6 >>> 8) & 0xff;
        out[op++] = (in6 >>> 16) & 0xff;
        out[op++] = (in6 >>> 24) & 0xff;
        out[op++] = (in7 >>> 0) & 0xff;
        out[op++] = (in7 >>> 8) & 0xff;
        out[op++] = (in7 >>> 16) & 0xff;
        out[op++] = (in7 >>> 24) & 0xff;
    }
}

export function fastUnpack256_16(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let i = 0; i < 128; i++) {
        const in0 = inValues[ip++] >>> 0;
        out[op++] = in0 & 0xffff;
        out[op++] = (in0 >>> 16) & 0xffff;
    }
}


export function fastUnpack256_Generic(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    const mask = MASKS[bitWidth] >>> 0;

    let inputWordIndex = inPos;
    let bitOffset = 0;
    let currentWord = inValues[inputWordIndex] >>> 0;
    let op = outPos;

    for (let c = 0; c < 8; c++) {
        for (let i = 0; i < 32; i++) {
            if (bitOffset + bitWidth <= 32) {
                const value = (currentWord >>> bitOffset) & mask;
                out[op + i] = value | 0;
                bitOffset += bitWidth;

                if (bitOffset === 32) {
                    bitOffset = 0;
                    inputWordIndex++;
                    if (i !== 31) {
                        currentWord = inValues[inputWordIndex] >>> 0;
                    }
                }
            } else {
                const lowBits = 32 - bitOffset;
                const low = currentWord >>> bitOffset;

                inputWordIndex++;
                currentWord = inValues[inputWordIndex] >>> 0;

                const highBits = bitWidth - lowBits;
                const highMask = (-1 >>> (32 - highBits)) >>> 0;

                const high = currentWord & highMask;

                const value = (low | (high << lowBits)) & mask;
                out[op + i] = value | 0;
                bitOffset = highBits;
            }
        }
        op += 32;

        bitOffset = 0;
        if (c < 7) {
            currentWord = inValues[inputWordIndex] >>> 0;
        }
    }
}
