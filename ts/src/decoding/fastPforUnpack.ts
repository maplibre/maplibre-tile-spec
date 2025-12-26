/**
 * FastPFOR Bit-Unpacking Functions
 *
 * This file contains optimized bit-unpacking routines for FastPFOR decoding.
 * Note: This code is mechanically structured and should be treated as "generated code"
 * even though it was hand-written for performance optimization.
 * 
 * Avoid manually editing individual unpack functions if possible. If changes are needed,
 * update the pattern and verify all functions together.
 *
 * Exports:
 *  - fastUnpack32_N(inValues, inPos, out, outPos) for N in selected bitwidths (1-12, 16)
 *  - fastUnpack256_N(inValues, inPos, out, outPos) for N in selected bitwidths (1-6, 8, 16)
 *  - fastUnpack256_Generic(inValues, inPos, out, outPos, bitWidth) for other bitwidths (incl. 7, 9-15, 17-31)
 */

import { MASKS } from "./fastPforSpec";

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
    const in0 = inValues[inPos + 0] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;

    out[op++] = ((in0 >>> 0) & 0x3ff) | 0;
    out[op++] = ((in0 >>> 10) & 0x3ff) | 0;
    out[op++] = ((in0 >>> 20) & 0x3ff) | 0;
    out[op++] = (((in0 >>> 30) | ((in1 & 0xff) << 2)) & 0x3ff) | 0;
    out[op++] = ((in1 >>> 8) & 0x3ff) | 0;
    out[op++] = ((in1 >>> 18) & 0x3ff) | 0;
    out[op++] = (((in1 >>> 28) | ((in2 & 0x3f) << 4)) & 0x3ff) | 0;
    out[op++] = ((in2 >>> 6) & 0x3ff) | 0;
    out[op++] = ((in2 >>> 16) & 0x3ff) | 0;
    out[op++] = (((in2 >>> 26) | ((in3 & 0xf) << 6)) & 0x3ff) | 0;
    out[op++] = ((in3 >>> 4) & 0x3ff) | 0;
    out[op++] = ((in3 >>> 14) & 0x3ff) | 0;
    out[op++] = (((in3 >>> 24) | ((in4 & 0x3) << 8)) & 0x3ff) | 0;
    out[op++] = ((in4 >>> 2) & 0x3ff) | 0;
    out[op++] = ((in4 >>> 12) & 0x3ff) | 0;
    out[op++] = ((in4 >>> 22) & 0x3ff) | 0;
    out[op++] = ((in5 >>> 0) & 0x3ff) | 0;
    out[op++] = ((in5 >>> 10) & 0x3ff) | 0;
    out[op++] = ((in5 >>> 20) & 0x3ff) | 0;
    out[op++] = (((in5 >>> 30) | ((in6 & 0xff) << 2)) & 0x3ff) | 0;
    out[op++] = ((in6 >>> 8) & 0x3ff) | 0;
    out[op++] = ((in6 >>> 18) & 0x3ff) | 0;
    out[op++] = (((in6 >>> 28) | ((in7 & 0x3f) << 4)) & 0x3ff) | 0;
    out[op++] = ((in7 >>> 6) & 0x3ff) | 0;
    out[op++] = ((in7 >>> 16) & 0x3ff) | 0;
    out[op++] = (((in7 >>> 26) | ((in8 & 0xf) << 6)) & 0x3ff) | 0;
    out[op++] = ((in8 >>> 4) & 0x3ff) | 0;
    out[op++] = ((in8 >>> 14) & 0x3ff) | 0;
    out[op++] = (((in8 >>> 24) | ((in9 & 0x3) << 8)) & 0x3ff) | 0;
    out[op++] = ((in9 >>> 2) & 0x3ff) | 0;
    out[op++] = ((in9 >>> 12) & 0x3ff) | 0;
    out[op++] = ((in9 >>> 22) & 0x3ff) | 0;
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

//fastUnpack256 variants
// These flatten the 8 calls to fastUnpack32 into a single function to enable
// better register allocation and avoid call overhead.

export function fastUnpack256_1(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    // 1 bit per value = 32 values per int32. 
    // 256 values = 8 int32s.
    // Unrolling 8 iterations of fastUnpack32_1 logic.
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 1; out[op++] = (in0 >>> 1) & 1;
        out[op++] = (in0 >>> 2) & 1; out[op++] = (in0 >>> 3) & 1;
        out[op++] = (in0 >>> 4) & 1; out[op++] = (in0 >>> 5) & 1;
        out[op++] = (in0 >>> 6) & 1; out[op++] = (in0 >>> 7) & 1;
        out[op++] = (in0 >>> 8) & 1; out[op++] = (in0 >>> 9) & 1;
        out[op++] = (in0 >>> 10) & 1; out[op++] = (in0 >>> 11) & 1;
        out[op++] = (in0 >>> 12) & 1; out[op++] = (in0 >>> 13) & 1;
        out[op++] = (in0 >>> 14) & 1; out[op++] = (in0 >>> 15) & 1;
        out[op++] = (in0 >>> 16) & 1; out[op++] = (in0 >>> 17) & 1;
        out[op++] = (in0 >>> 18) & 1; out[op++] = (in0 >>> 19) & 1;
        out[op++] = (in0 >>> 20) & 1; out[op++] = (in0 >>> 21) & 1;
        out[op++] = (in0 >>> 22) & 1; out[op++] = (in0 >>> 23) & 1;
        out[op++] = (in0 >>> 24) & 1; out[op++] = (in0 >>> 25) & 1;
        out[op++] = (in0 >>> 26) & 1; out[op++] = (in0 >>> 27) & 1;
        out[op++] = (in0 >>> 28) & 1; out[op++] = (in0 >>> 29) & 1;
        out[op++] = (in0 >>> 30) & 1; out[op++] = (in0 >>> 31) & 1;
    }
}

export function fastUnpack256_2(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    // 2 bits per value = 16 values per int32.
    // 256 values = 16 int32s.
    // Each fastUnpack32_2 consumes 2 ints for 32 values.
    // We need 8 chunks.
    let op = outPos;
    let ip = inPos;
    // Manual unroll of outer loop 8 times? Or strict loop?
    // Loop of 8 is likely fine, JS Engine unrolls small constant loops.
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 3; out[op++] = (in0 >>> 2) & 3;
        out[op++] = (in0 >>> 4) & 3; out[op++] = (in0 >>> 6) & 3;
        out[op++] = (in0 >>> 8) & 3; out[op++] = (in0 >>> 10) & 3;
        out[op++] = (in0 >>> 12) & 3; out[op++] = (in0 >>> 14) & 3;
        out[op++] = (in0 >>> 16) & 3; out[op++] = (in0 >>> 18) & 3;
        out[op++] = (in0 >>> 20) & 3; out[op++] = (in0 >>> 22) & 3;
        out[op++] = (in0 >>> 24) & 3; out[op++] = (in0 >>> 26) & 3;
        out[op++] = (in0 >>> 28) & 3; out[op++] = (in0 >>> 30) & 3;
        out[op++] = (in1 >>> 0) & 3; out[op++] = (in1 >>> 2) & 3;
        out[op++] = (in1 >>> 4) & 3; out[op++] = (in1 >>> 6) & 3;
        out[op++] = (in1 >>> 8) & 3; out[op++] = (in1 >>> 10) & 3;
        out[op++] = (in1 >>> 12) & 3; out[op++] = (in1 >>> 14) & 3;
        out[op++] = (in1 >>> 16) & 3; out[op++] = (in1 >>> 18) & 3;
        out[op++] = (in1 >>> 20) & 3; out[op++] = (in1 >>> 22) & 3;
        out[op++] = (in1 >>> 24) & 3; out[op++] = (in1 >>> 26) & 3;
        out[op++] = (in1 >>> 28) & 3; out[op++] = (in1 >>> 30) & 3;
    }
}

export function fastUnpack256_3(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        out[op++] = (in0 >>> 0) & 7;
        out[op++] = (in0 >>> 3) & 7;
        out[op++] = (in0 >>> 6) & 7;
        out[op++] = (in0 >>> 9) & 7;
        out[op++] = (in0 >>> 12) & 7;
        out[op++] = (in0 >>> 15) & 7;
        out[op++] = (in0 >>> 18) & 7;
        out[op++] = (in0 >>> 21) & 7;
        out[op++] = (in0 >>> 24) & 7;
        out[op++] = (in0 >>> 27) & 7;
        out[op++] = ((in0 >>> 30) | ((in1 & 1) << 2)) & 7;
        out[op++] = (in1 >>> 1) & 7;
        out[op++] = (in1 >>> 4) & 7;
        out[op++] = (in1 >>> 7) & 7;
        out[op++] = (in1 >>> 10) & 7;
        out[op++] = (in1 >>> 13) & 7;
        out[op++] = (in1 >>> 16) & 7;
        out[op++] = (in1 >>> 19) & 7;
        out[op++] = (in1 >>> 22) & 7;
        out[op++] = (in1 >>> 25) & 7;
        out[op++] = (in1 >>> 28) & 7;
        out[op++] = ((in1 >>> 31) | ((in2 & 3) << 1)) & 7;
        out[op++] = (in2 >>> 2) & 7;
        out[op++] = (in2 >>> 5) & 7;
        out[op++] = (in2 >>> 8) & 7;
        out[op++] = (in2 >>> 11) & 7;
        out[op++] = (in2 >>> 14) & 7;
        out[op++] = (in2 >>> 17) & 7;
        out[op++] = (in2 >>> 20) & 7;
        out[op++] = (in2 >>> 23) & 7;
        out[op++] = (in2 >>> 26) & 7;
        out[op++] = (in2 >>> 29) & 7;
    }
}

export function fastUnpack256_4(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    // 4 bits -> 8 values per Int32. 32 values need 4 Int32s.
    // 8 chunks.
    let op = outPos;
    let ip = inPos;
    for (let c = 0; c < 8; c++) {
        const in0 = inValues[ip++] >>> 0;
        const in1 = inValues[ip++] >>> 0;
        const in2 = inValues[ip++] >>> 0;
        const in3 = inValues[ip++] >>> 0;
        out[op++] = in0 & 15; out[op++] = (in0 >>> 4) & 15;
        out[op++] = (in0 >>> 8) & 15; out[op++] = (in0 >>> 12) & 15;
        out[op++] = (in0 >>> 16) & 15; out[op++] = (in0 >>> 20) & 15;
        out[op++] = (in0 >>> 24) & 15; out[op++] = (in0 >>> 28) & 15;
        out[op++] = in1 & 15; out[op++] = (in1 >>> 4) & 15;
        out[op++] = (in1 >>> 8) & 15; out[op++] = (in1 >>> 12) & 15;
        out[op++] = (in1 >>> 16) & 15; out[op++] = (in1 >>> 20) & 15;
        out[op++] = (in1 >>> 24) & 15; out[op++] = (in1 >>> 28) & 15;
        out[op++] = in2 & 15; out[op++] = (in2 >>> 4) & 15;
        out[op++] = (in2 >>> 8) & 15; out[op++] = (in2 >>> 12) & 15;
        out[op++] = (in2 >>> 16) & 15; out[op++] = (in2 >>> 20) & 15;
        out[op++] = (in2 >>> 24) & 15; out[op++] = (in2 >>> 28) & 15;
        out[op++] = in3 & 15; out[op++] = (in3 >>> 4) & 15;
        out[op++] = (in3 >>> 8) & 15; out[op++] = (in3 >>> 12) & 15;
        out[op++] = (in3 >>> 16) & 15; out[op++] = (in3 >>> 20) & 15;
        out[op++] = (in3 >>> 24) & 15; out[op++] = (in3 >>> 28) & 15;
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
        out[op++] = in0 & 31; out[op++] = (in0 >>> 5) & 31;
        out[op++] = (in0 >>> 10) & 31; out[op++] = (in0 >>> 15) & 31;
        out[op++] = (in0 >>> 20) & 31; out[op++] = (in0 >>> 25) & 31;
        out[op++] = ((in0 >>> 30) | ((in1 & 7) << 2));
        out[op++] = (in1 >>> 3) & 31; out[op++] = (in1 >>> 8) & 31;
        out[op++] = (in1 >>> 13) & 31; out[op++] = (in1 >>> 18) & 31;
        out[op++] = (in1 >>> 23) & 31;
        out[op++] = ((in1 >>> 28) | ((in2 & 1) << 4));
        out[op++] = (in2 >>> 1) & 31; out[op++] = (in2 >>> 6) & 31;
        out[op++] = (in2 >>> 11) & 31; out[op++] = (in2 >>> 16) & 31;
        out[op++] = (in2 >>> 21) & 31; out[op++] = (in2 >>> 26) & 31;
        out[op++] = ((in2 >>> 31) | ((in3 & 15) << 1));
        out[op++] = (in3 >>> 4) & 31; out[op++] = (in3 >>> 9) & 31;
        out[op++] = (in3 >>> 14) & 31; out[op++] = (in3 >>> 19) & 31;
        out[op++] = (in3 >>> 24) & 31;
        out[op++] = ((in3 >>> 29) | ((in4 & 3) << 3));
        out[op++] = (in4 >>> 2) & 31; out[op++] = (in4 >>> 7) & 31;
        out[op++] = (in4 >>> 12) & 31; out[op++] = (in4 >>> 17) & 31;
        out[op++] = (in4 >>> 22) & 31; out[op++] = (in4 >>> 27) & 31;
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
        out[op++] = in0 & 63; out[op++] = (in0 >>> 6) & 63;
        out[op++] = (in0 >>> 12) & 63; out[op++] = (in0 >>> 18) & 63;
        out[op++] = (in0 >>> 24) & 63;
        out[op++] = ((in0 >>> 30) | ((in1 & 15) << 2));
        out[op++] = (in1 >>> 4) & 63; out[op++] = (in1 >>> 10) & 63;
        out[op++] = (in1 >>> 16) & 63; out[op++] = (in1 >>> 22) & 63;
        out[op++] = ((in1 >>> 28) | ((in2 & 3) << 4));
        out[op++] = (in2 >>> 2) & 63; out[op++] = (in2 >>> 8) & 63;
        out[op++] = (in2 >>> 14) & 63; out[op++] = (in2 >>> 20) & 63;
        out[op++] = (in2 >>> 26) & 63;
        out[op++] = in3 & 63; out[op++] = (in3 >>> 6) & 63;
        out[op++] = (in3 >>> 12) & 63; out[op++] = (in3 >>> 18) & 63;
        out[op++] = (in3 >>> 24) & 63;
        out[op++] = ((in3 >>> 30) | ((in4 & 15) << 2));
        out[op++] = (in4 >>> 4) & 63; out[op++] = (in4 >>> 10) & 63;
        out[op++] = (in4 >>> 16) & 63; out[op++] = (in4 >>> 22) & 63;
        out[op++] = ((in4 >>> 28) | ((in5 & 3) << 4));
        out[op++] = (in5 >>> 2) & 63; out[op++] = (in5 >>> 8) & 63;
        out[op++] = (in5 >>> 14) & 63; out[op++] = (in5 >>> 20) & 63;
        out[op++] = (in5 >>> 26) & 63;
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
        out[op++] = (in0 >>> 0) & 0x7f; out[op++] = (in0 >>> 7) & 0x7f;
        out[op++] = (in0 >>> 14) & 0x7f; out[op++] = (in0 >>> 21) & 0x7f;
        out[op++] = ((in0 >>> 28) | ((in1 & 0x7) << 4)) & 0x7f;
        out[op++] = (in1 >>> 3) & 0x7f; out[op++] = (in1 >>> 10) & 0x7f;
        out[op++] = (in1 >>> 17) & 0x7f; out[op++] = (in1 >>> 24) & 0x7f;
        out[op++] = ((in1 >>> 31) | ((in2 & 0x3f) << 1)) & 0x7f;
        out[op++] = (in2 >>> 6) & 0x7f; out[op++] = (in2 >>> 13) & 0x7f;
        out[op++] = (in2 >>> 20) & 0x7f;
        out[op++] = ((in2 >>> 27) | ((in3 & 0x3) << 5)) & 0x7f;
        out[op++] = (in3 >>> 2) & 0x7f; out[op++] = (in3 >>> 9) & 0x7f;
        out[op++] = (in3 >>> 16) & 0x7f; out[op++] = (in3 >>> 23) & 0x7f;
        out[op++] = ((in3 >>> 30) | ((in4 & 0x1f) << 2)) & 0x7f;
        out[op++] = (in4 >>> 5) & 0x7f; out[op++] = (in4 >>> 12) & 0x7f;
        out[op++] = (in4 >>> 19) & 0x7f;
        out[op++] = ((in4 >>> 26) | ((in5 & 0x1) << 6)) & 0x7f;
        out[op++] = (in5 >>> 1) & 0x7f; out[op++] = (in5 >>> 8) & 0x7f;
        out[op++] = (in5 >>> 15) & 0x7f; out[op++] = (in5 >>> 22) & 0x7f;
        out[op++] = ((in5 >>> 29) | ((in6 & 0xf) << 3)) & 0x7f;
        out[op++] = (in6 >>> 4) & 0x7f; out[op++] = (in6 >>> 11) & 0x7f;
        out[op++] = (in6 >>> 18) & 0x7f; out[op++] = (in6 >>> 25) & 0x7f;
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
        out[op++] = (in0 >>> 0) & 0xff; out[op++] = (in0 >>> 8) & 0xff;
        out[op++] = (in0 >>> 16) & 0xff; out[op++] = (in0 >>> 24) & 0xff;
        out[op++] = (in1 >>> 0) & 0xff; out[op++] = (in1 >>> 8) & 0xff;
        out[op++] = (in1 >>> 16) & 0xff; out[op++] = (in1 >>> 24) & 0xff;
        out[op++] = (in2 >>> 0) & 0xff; out[op++] = (in2 >>> 8) & 0xff;
        out[op++] = (in2 >>> 16) & 0xff; out[op++] = (in2 >>> 24) & 0xff;
        out[op++] = (in3 >>> 0) & 0xff; out[op++] = (in3 >>> 8) & 0xff;
        out[op++] = (in3 >>> 16) & 0xff; out[op++] = (in3 >>> 24) & 0xff;
        out[op++] = (in4 >>> 0) & 0xff; out[op++] = (in4 >>> 8) & 0xff;
        out[op++] = (in4 >>> 16) & 0xff; out[op++] = (in4 >>> 24) & 0xff;
        out[op++] = (in5 >>> 0) & 0xff; out[op++] = (in5 >>> 8) & 0xff;
        out[op++] = (in5 >>> 16) & 0xff; out[op++] = (in5 >>> 24) & 0xff;
        out[op++] = (in6 >>> 0) & 0xff; out[op++] = (in6 >>> 8) & 0xff;
        out[op++] = (in6 >>> 16) & 0xff; out[op++] = (in6 >>> 24) & 0xff;
        out[op++] = (in7 >>> 0) & 0xff; out[op++] = (in7 >>> 8) & 0xff;
        out[op++] = (in7 >>> 16) & 0xff; out[op++] = (in7 >>> 24) & 0xff;
    }
}

export function fastUnpack256_16(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    let op = outPos;
    let ip = inPos;
    // 16 bits = 2 values per int. 256 values = 128 ints.
    for (let i = 0; i < 128; i++) {
        const in0 = inValues[ip++] >>> 0;
        out[op++] = in0 & 0xffff;
        out[op++] = (in0 >>> 16) & 0xffff;
    }
}


export function fastUnpack256_Generic(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    // mask for bitWidth bits
    const mask = MASKS[bitWidth] >>> 0;

    let inputWordIndex = inPos;
    let bitOffset = 0;
    let currentWord = inValues[inputWordIndex] >>> 0;
    let op = outPos;

    for (let c = 0; c < 8; c++) {
        // Unpack 32 values
        for (let i = 0; i < 32; i++) {
            if (bitOffset + bitWidth <= 32) {
                const value = (currentWord >>> bitOffset) & mask;
                out[op + i] = value | 0;
                bitOffset += bitWidth;

                if (bitOffset === 32) {
                    bitOffset = 0;
                    inputWordIndex++;
                    // Only fetch next word if we are not done with this block of 32
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

        // After 32 values (block of bitWidth*32 bits), we align to word boundary.
        bitOffset = 0;
        // Load next word for the next iteration of 'c', unless we are done.
        if (c < 7) {
            // currentWord needs to be refreshed from the already-incremented inputWordIndex
            currentWord = inValues[inputWordIndex] >>> 0;
        }
    }
}
