import IntWrapper from "./decoding/intWrapper";

type Int32Buf = Int32Array<ArrayBufferLike>;

const MASKS = (() => {
    const masks = new Uint32Array(33);
    masks[0] = 0;
    for (let bitWidth = 1; bitWidth <= 32; bitWidth++) {
        masks[bitWidth] = bitWidth === 32 ? 0xffffffff : 0xffffffff >>> (32 - bitWidth);
    }
    return masks;
})();

const OVERHEAD_OF_EACH_EXCEPT = 8;
const DEFAULT_PAGE_SIZE = 65536;
const BLOCK_SIZE = 256;

interface Int32Codec {
    compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf;
    uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void;
}

function greatestMultiple(value: number, factor: number): number {
    return value - (value % factor);
}

function roundUpToMultipleOf32(value: number): number {
    return greatestMultiple(value + 31, 32);
}

// FastPFOR operates on uint32 bit patterns stored in Int32Array
function bits(value: number): number {
    return 32 - Math.clz32(value >>> 0);
}

function normalizePageSize(pageSize: number): number {
    if (!Number.isFinite(pageSize) || pageSize <= 0) return DEFAULT_PAGE_SIZE;

    const aligned = greatestMultiple(Math.floor(pageSize), BLOCK_SIZE);
    return aligned === 0 ? BLOCK_SIZE : aligned;
}

function ensureInt32Capacity(buffer: Int32Buf, requiredLength: number): Int32Buf {
    if (requiredLength <= buffer.length) return buffer;

    let newLength = buffer.length === 0 ? 1 : buffer.length;
    while (newLength < requiredLength) {
        newLength *= 2;
    }

    const next = new Int32Array(newLength) as Int32Buf;
    next.set(buffer);
    return next;
}

function ensureUint8Capacity(buffer: Uint8Array, requiredLength: number): Uint8Array {
    if (requiredLength <= buffer.length) return buffer;

    let newLength = buffer.length === 0 ? 1 : buffer.length;
    while (newLength < requiredLength) {
        newLength *= 2;
    }

    const next = new Uint8Array(newLength);
    next.set(buffer);
    return next;
}

function getMask(bitWidth: number): number {
    return MASKS[bitWidth] >>> 0;
}

/**
 * Generic bit-packing of 32 integers, matching JavaFastPFOR BitPacking.fastpack ordering.
 * Writes exactly `bitWidth` int32 words into `out` starting at `outPos`.
 */
function fastPack32(inValues: Int32Array, inPos: number, out: Int32Buf, outPos: number, bitWidth: number): void {
    if (bitWidth === 0) return;
    if (bitWidth === 32) {
        out.set(inValues.subarray(inPos, inPos + 32), outPos);
        return;
    }

    const mask = getMask(bitWidth);
    let outputWordIndex = outPos;
    let bitOffset = 0;
    let currentWord = 0;

    for (let i = 0; i < 32; i++) {
        const value = (inValues[inPos + i] >>> 0) & mask;

        if (bitOffset + bitWidth <= 32) {
            currentWord |= value << bitOffset;
            bitOffset += bitWidth;

            if (bitOffset === 32) {
                out[outputWordIndex++] = currentWord | 0;
                bitOffset = 0;
                currentWord = 0;
            }
        } else {
            const lowBits = 32 - bitOffset;
            const lowMask = MASKS[lowBits] >>> 0;
            currentWord |= (value & lowMask) << bitOffset;
            out[outputWordIndex++] = currentWord | 0;
            currentWord = value >>> lowBits;
            bitOffset = bitWidth - lowBits;
        }
    }
}

// Specialized unpack functions for common bitwidths (avoid generic loop overhead)
function fastUnpack32_1(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    const in0 = inValues[inPos] >>> 0;
    for (let i = 0; i < 32; i++) {
        out[outPos + i] = (in0 >>> i) & 1;
    }
}

function fastUnpack32_2(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_3(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_4(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_5(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_6(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_7(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_8(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_9(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_10(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_11(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_12(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

function fastUnpack32_16(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
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

/**
 * Generic bit-unpacking of 32 integers, matching JavaFastPFOR BitPacking.fastunpack ordering.
 * Reads exactly `bitWidth` int32 words from `inValues` starting at `inPos`.
 */
// Dispatch table for specialized unpack functions (indices 1-12)
// Extended Hybrid: 1-12 + 16 are specialized, 13-15 and 17+ use generic fallback
const UNPACK_DISPATCH: ((inValues: Int32Array, inPos: number, out: Int32Array, outPos: number) => void)[] = [
    () => { }, // 0 - handled separately
    fastUnpack32_1,
    fastUnpack32_2,
    fastUnpack32_3,
    fastUnpack32_4,
    fastUnpack32_5,
    fastUnpack32_6,
    fastUnpack32_7,
    fastUnpack32_8,
    fastUnpack32_9,
    fastUnpack32_10,
    fastUnpack32_11,
    fastUnpack32_12,
];

/**
 * Generic bit-unpacking of 32 integers, matching JavaFastPFOR BitPacking.fastunpack ordering.
 * Reads exactly `bitWidth` int32 words from `inValues` starting at `inPos`.
 * Extended Hybrid: specialized for 1-12 and 16, generic fallback for 13-15 and 17+.
 */
function fastUnpack32(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    if (bitWidth === 0) {
        out.fill(0, outPos, outPos + 32);
        return;
    }
    if (bitWidth === 32) {
        out.set(inValues.subarray(inPos, inPos + 32), outPos);
        return;
    }

    // Use specialized function for bitwidths 1-12
    if (bitWidth <= 12) {
        UNPACK_DISPATCH[bitWidth](inValues, inPos, out, outPos);
        return;
    }

    // Special case for bitwidth 16 (common for coordinates)
    if (bitWidth === 16) {
        fastUnpack32_16(inValues, inPos, out, outPos);
        return;
    }

    // Generic fallback for bitwidths 13-15 and 17-31
    const mask = MASKS[bitWidth] >>> 0;
    let inputWordIndex = inPos;
    let bitOffset = 0;
    let currentWord = inValues[inputWordIndex] >>> 0;

    for (let i = 0; i < 32; i++) {
        if (bitOffset + bitWidth <= 32) {
            const value = (currentWord >>> bitOffset) & mask;
            out[outPos + i] = value | 0;
            bitOffset += bitWidth;

            if (bitOffset === 32) {
                bitOffset = 0;
                inputWordIndex++;
                if (i !== 31) currentWord = inValues[inputWordIndex] >>> 0;
            }
        } else {
            const lowBits = 32 - bitOffset;
            const low = currentWord >>> bitOffset;

            inputWordIndex++;
            currentWord = inValues[inputWordIndex] >>> 0;
            const highMask = MASKS[bitWidth - lowBits] >>> 0;
            const high = currentWord & highMask;

            const value = (low | (high << lowBits)) & mask;
            out[outPos + i] = value | 0;
            bitOffset = bitWidth - lowBits;
        }
    }
}

class FastPfor implements Int32Codec {
    private readonly pageSize: number;
    private dataToBePacked: Int32Array[] = new Array(33);
    private byteContainer: Uint8Array;
    private byteContainerPos = 0;
    private readonly dataPointers = new Int32Array(33);
    private readonly freqs = new Int32Array(33);
    private readonly best = new Int32Array(3);
    // Reusable buffer for decoding byteContainer (avoids allocation per page)
    private decodeByteContainer: Uint8Array;

    constructor(pageSize = DEFAULT_PAGE_SIZE) {
        this.pageSize = normalizePageSize(pageSize);

        const byteContainerSize = (3 * this.pageSize) / BLOCK_SIZE + this.pageSize;
        this.byteContainer = new Uint8Array(byteContainerSize);
        this.decodeByteContainer = new Uint8Array(byteContainerSize);

        const initialPackedSize = (this.pageSize / 32) * 4;
        for (let k = 1; k < this.dataToBePacked.length; k++) {
            this.dataToBePacked[k] = new Int32Array(initialPackedSize);
        }
    }

    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
        if (alignedLength === 0) return out;

        out = ensureInt32Capacity(out, outPos.get() + 1);
        out[outPos.get()] = alignedLength;
        outPos.increment();

        return this.headlessCompress(inValues, inPos, alignedLength, out, outPos);
    }

    private headlessCompress(
        inValues: Int32Array,
        inPos: IntWrapper,
        inLength: number,
        out: Int32Buf,
        outPos: IntWrapper,
    ): Int32Buf {
        const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
        const finalInPos = inPos.get() + alignedLength;

        while (inPos.get() !== finalInPos) {
            const thisSize = Math.min(this.pageSize, finalInPos - inPos.get());
            out = this.encodePage(inValues, inPos, thisSize, out, outPos);
        }

        return out;
    }

    private getBestBFromData(inValues: Int32Array, pos: number): void {
        this.freqs.fill(0);
        for (let k = pos, kEnd = pos + BLOCK_SIZE; k < kEnd; k++) {
            this.freqs[bits(inValues[k])]++;
        }

        let maxBits = 32;
        while (this.freqs[maxBits] === 0) maxBits--;

        let bestB = maxBits;
        let bestCost = maxBits * BLOCK_SIZE;
        let cExcept = 0;
        let bestCExcept = cExcept;

        for (let b = maxBits - 1; b >= 0; b--) {
            cExcept += this.freqs[b + 1];
            if (cExcept === BLOCK_SIZE) break;

            let thisCost =
                cExcept * OVERHEAD_OF_EACH_EXCEPT + cExcept * (maxBits - b) + b * BLOCK_SIZE + 8;
            if (maxBits - b === 1) thisCost -= cExcept;

            if (thisCost < bestCost) {
                bestCost = thisCost;
                bestB = b;
                bestCExcept = cExcept;
            }
        }

        this.best[0] = bestB;
        this.best[1] = bestCExcept;
        this.best[2] = maxBits;
    }

    private byteContainerClear(): void {
        this.byteContainerPos = 0;
    }

    private byteContainerPut(byteValue: number): void {
        if (this.byteContainerPos >= this.byteContainer.length) {
            this.byteContainer = ensureUint8Capacity(this.byteContainer, this.byteContainerPos + 1);
        }
        this.byteContainer[this.byteContainerPos++] = byteValue & 0xff;
    }

    private encodePage(
        inValues: Int32Array,
        inPos: IntWrapper,
        thisSize: number,
        out: Int32Buf,
        outPos: IntWrapper,
    ): Int32Buf {
        const headerPos = outPos.get();
        out = ensureInt32Capacity(out, headerPos + 1);
        outPos.increment();
        let tmpOutPos = outPos.get();

        this.dataPointers.fill(0);
        this.byteContainerClear();

        let tmpInPos = inPos.get();
        const finalInPos = tmpInPos + thisSize - BLOCK_SIZE;

        for (; tmpInPos <= finalInPos; tmpInPos += BLOCK_SIZE) {
            this.getBestBFromData(inValues, tmpInPos);

            const b = this.best[0];
            const cExcept = this.best[1];
            const maxBits = this.best[2];

            this.byteContainerPut(b);
            this.byteContainerPut(cExcept);

            if (cExcept > 0) {
                this.byteContainerPut(maxBits);
                const index = maxBits - b;

                // index must be in [1..32] range; anything else is a bug
                if (index < 1 || index > 32) {
                    throw new Error(`FastPFOR encode: invalid exception index=${index} (b=${b}, maxBits=${maxBits})`);
                }

                if (index !== 1) {
                    const needed = this.dataPointers[index] + cExcept;
                    if (needed >= this.dataToBePacked[index].length) {
                        let newSize = 2 * needed;
                        newSize = roundUpToMultipleOf32(newSize);
                        const next = new Int32Array(newSize);
                        next.set(this.dataToBePacked[index]);
                        this.dataToBePacked[index] = next;
                    }
                }

                for (let k = 0; k < BLOCK_SIZE; k++) {
                    const value = inValues[tmpInPos + k] >>> 0;
                    if ((value >>> b) !== 0) {
                        this.byteContainerPut(k);
                        if (index !== 1) {
                            this.dataToBePacked[index][this.dataPointers[index]++] = (value >>> b) | 0;
                        }
                    }
                }
            }

            for (let k = 0; k < BLOCK_SIZE; k += 32) {
                out = ensureInt32Capacity(out, tmpOutPos + b);
                fastPack32(inValues, tmpInPos + k, out, tmpOutPos, b);
                tmpOutPos += b;
            }
        }

        inPos.set(tmpInPos);
        out[headerPos] = (tmpOutPos - headerPos) | 0;

        const byteSize = this.byteContainerPos;
        while ((this.byteContainerPos & 3) !== 0) this.byteContainerPut(0);

        out = ensureInt32Capacity(out, tmpOutPos + 1);
        out[tmpOutPos++] = byteSize | 0;

        const howManyInts = this.byteContainerPos / 4;
        out = ensureInt32Capacity(out, tmpOutPos + howManyInts);
        for (let i = 0; i < howManyInts; i++) {
            const base = i * 4;
            // byteContainer is serialized in little-endian inside int32 words (matching JavaFastPFOR),
            // independent of how the overall Int32 stream is later converted to bytes.
            const v =
                (this.byteContainer[base] |
                    (this.byteContainer[base + 1] << 8) |
                    (this.byteContainer[base + 2] << 16) |
                    (this.byteContainer[base + 3] << 24)) |
                0;
            out[tmpOutPos + i] = v;
        }
        tmpOutPos += howManyInts;

        let bitmap = 0;
        for (let k = 2; k <= 32; k++) {
            if (this.dataPointers[k] !== 0) bitmap |= 1 << (k - 1);
        }

        out = ensureInt32Capacity(out, tmpOutPos + 1);
        out[tmpOutPos++] = bitmap | 0;

        for (let k = 2; k <= 32; k++) {
            const size = this.dataPointers[k];
            if (size !== 0) {
                out = ensureInt32Capacity(out, tmpOutPos + 1);
                out[tmpOutPos++] = size | 0;

                let j = 0;
                for (; j < size; j += 32) {
                    out = ensureInt32Capacity(out, tmpOutPos + k);
                    fastPack32(this.dataToBePacked[k], j, out, tmpOutPos, k);
                    tmpOutPos += k;
                }

                const overflow = j - size;
                tmpOutPos -= Math.floor((overflow * k) / 32);
            }
        }

        outPos.set(tmpOutPos);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        if (inLength === 0) return;

        // Validate that we have at least 1 int32 in the window to read the outLength header
        if (inLength < 1) {
            throw new Error(`FastPFOR: buffer too small to read alignedLength header`);
        }

        const outLength = inValues[inPos.get()];
        inPos.increment();

        // Validate outLength is non-negative
        if (outLength < 0) {
            throw new Error(`FastPFOR: negative outLength=${outLength}`);
        }

        // Validate outLength is a multiple of BLOCK_SIZE (256)
        if ((outLength & (BLOCK_SIZE - 1)) !== 0) {
            throw new Error(`FastPFOR: outLength not multiple of ${BLOCK_SIZE}: ${outLength}`);
        }

        // Validate outLength doesn't exceed output buffer capacity
        if (outPos.get() + outLength > out.length) {
            throw new Error(`FastPFOR: outLength=${outLength} exceeds output capacity ${out.length - outPos.get()}`);
        }

        this.headlessUncompress(inValues, inPos, inLength, out, outPos, outLength);
    }

    private headlessUncompress(
        inValues: Int32Array,
        inPos: IntWrapper,
        _inLength: number,
        out: Int32Array,
        outPos: IntWrapper,
        outLength: number,
    ): void {
        const alignedOutLength = greatestMultiple(outLength, BLOCK_SIZE);
        const finalOut = outPos.get() + alignedOutLength;
        while (outPos.get() !== finalOut) {
            const thisSize = Math.min(this.pageSize, finalOut - outPos.get());
            this.decodePage(inValues, inPos, out, outPos, thisSize);
        }
    }

    private decodePage(inValues: Int32Array, inPos: IntWrapper, out: Int32Array, outPos: IntWrapper, thisSize: number): void {
        const initPos = inPos.get();
        const whereMeta = inValues[inPos.get()];
        inPos.increment();

        // Validate whereMeta bounds (anti-corruption guard)
        if (whereMeta <= 0 || initPos + whereMeta >= inValues.length) {
            throw new Error(`FastPFOR: invalid whereMeta ${whereMeta} at position ${initPos}`);
        }

        let inExcept = initPos + whereMeta;

        // Validate inExcept bounds before reading byteSize
        if (inExcept >= inValues.length) {
            throw new Error(`FastPFOR decode: inExcept ${inExcept} out of bounds (length=${inValues.length})`);
        }
        const byteSize = inValues[inExcept++] >>> 0;
        const metaInts = (byteSize + 3) >>> 2;

        // Validate metaInts bounds before reading byteContainer and bitmap
        if (inExcept + metaInts >= inValues.length) {
            throw new Error(`FastPFOR decode: metaInts overflow (inExcept=${inExcept}, metaInts=${metaInts}, length=${inValues.length})`);
        }

        // Reuse pre-allocated buffer instead of allocating new Uint8Array per page
        if (this.decodeByteContainer.length < byteSize) {
            this.decodeByteContainer = new Uint8Array(byteSize * 2);
        }
        const byteContainer = this.decodeByteContainer;
        for (let i = 0; i < byteSize; i++) {
            const intIdx = inExcept + (i >> 2);
            const byteInInt = i & 3;
            byteContainer[i] = (inValues[intIdx] >>> (byteInInt * 8)) & 0xff;
        }

        inExcept += metaInts;

        const bitmap = inValues[inExcept++];

        for (let k = 2; k <= 32; k++) {
            if ((bitmap & (1 << (k - 1))) !== 0) {
                const size = inValues[inExcept++];
                const roundedUp = roundUpToMultipleOf32(size);

                if (this.dataToBePacked[k].length < roundedUp) {
                    this.dataToBePacked[k] = new Int32Array(roundedUp);
                }

                let j = 0;
                for (; j < size; j += 32) {
                    fastUnpack32(inValues, inExcept, this.dataToBePacked[k], j, k);
                    inExcept += k;
                }

                const overflow = j - size;
                inExcept -= Math.floor((overflow * k) / 32);
            }
        }

        this.dataPointers.fill(0);
        let tmpOutPos = outPos.get();
        let tmpInPos = inPos.get();

        let bytePosIn = 0;
        const blocks = thisSize / BLOCK_SIZE;

        for (let run = 0; run < blocks; run++, tmpOutPos += BLOCK_SIZE) {
            const b = byteContainer[bytePosIn++];
            const cExcept = byteContainer[bytePosIn++];

            if (b === 0) {
                out.fill(0, tmpOutPos, tmpOutPos + BLOCK_SIZE);
            } else if (b === 32) {
                out.set(inValues.subarray(tmpInPos, tmpInPos + BLOCK_SIZE), tmpOutPos);
                tmpInPos += BLOCK_SIZE;
            } else if (b <= 12) {
                // Use dispatch table for bitwidths 1-12 (unrolled 8 iterations)
                const unpackFn = UNPACK_DISPATCH[b];
                unpackFn(inValues, tmpInPos, out, tmpOutPos);
                unpackFn(inValues, tmpInPos + b, out, tmpOutPos + 32);
                unpackFn(inValues, tmpInPos + b * 2, out, tmpOutPos + 64);
                unpackFn(inValues, tmpInPos + b * 3, out, tmpOutPos + 96);
                unpackFn(inValues, tmpInPos + b * 4, out, tmpOutPos + 128);
                unpackFn(inValues, tmpInPos + b * 5, out, tmpOutPos + 160);
                unpackFn(inValues, tmpInPos + b * 6, out, tmpOutPos + 192);
                unpackFn(inValues, tmpInPos + b * 7, out, tmpOutPos + 224);
                tmpInPos += b * 8;
            } else if (b === 16) {
                // Specialized function for bitwidth 16 (common for coordinates)
                fastUnpack32_16(inValues, tmpInPos, out, tmpOutPos);
                fastUnpack32_16(inValues, tmpInPos + 16, out, tmpOutPos + 32);
                fastUnpack32_16(inValues, tmpInPos + 32, out, tmpOutPos + 64);
                fastUnpack32_16(inValues, tmpInPos + 48, out, tmpOutPos + 96);
                fastUnpack32_16(inValues, tmpInPos + 64, out, tmpOutPos + 128);
                fastUnpack32_16(inValues, tmpInPos + 80, out, tmpOutPos + 160);
                fastUnpack32_16(inValues, tmpInPos + 96, out, tmpOutPos + 192);
                fastUnpack32_16(inValues, tmpInPos + 112, out, tmpOutPos + 224);
                tmpInPos += 128;
            } else {
                // Generic fallback for bitwidths 13-15 and 17-31
                for (let k = 0; k < BLOCK_SIZE; k += 32) {
                    fastUnpack32(inValues, tmpInPos, out, tmpOutPos + k, b);
                    tmpInPos += b;
                }
            }

            if (cExcept > 0) {
                const maxBits = byteContainer[bytePosIn++];
                const index = maxBits - b;

                // index must be in [1..32] range; anything else is corruption
                if (index < 1 || index > 32) {
                    throw new Error(`FastPFOR decode: invalid exception index=${index} (b=${b}, maxBits=${maxBits})`);
                }

                if (index === 1) {
                    for (let k = 0; k < cExcept; k++) {
                        const pos = byteContainer[bytePosIn++];
                        out[pos + tmpOutPos] |= 1 << b;
                    }
                } else {
                    for (let k = 0; k < cExcept; k++) {
                        const pos = byteContainer[bytePosIn++];
                        const exceptValue = this.dataToBePacked[index][this.dataPointers[index]++];
                        out[pos + tmpOutPos] |= exceptValue << b;
                    }
                }
            }
        }

        outPos.set(tmpOutPos);
        inPos.set(inExcept);
    }
}

class VariableByte implements Int32Codec {
    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        if (inLength === 0) return out;

        const bytes: number[] = [];

        const start = inPos.get();
        for (let k = start; k < start + inLength; k++) {
            let v = inValues[k] >>> 0;
            while (v >= 0x80) {
                bytes.push(v & 0x7f);
                v >>>= 7;
            }
            bytes.push(v | 0x80);
        }

        while (bytes.length % 4 !== 0) bytes.push(0);

        const intsToWrite = bytes.length / 4;
        out = ensureInt32Capacity(out, outPos.get() + intsToWrite);

        let outIdx = outPos.get();
        for (let i = 0; i < bytes.length; i += 4) {
            const v = (bytes[i] | (bytes[i + 1] << 8) | (bytes[i + 2] << 16) | (bytes[i + 3] << 24)) | 0;
            out[outIdx++] = v;
        }

        outPos.set(outIdx);
        inPos.add(inLength);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        let s = 0;
        let p = inPos.get();
        const finalP = inPos.get() + inLength;
        let tmpOutPos = outPos.get();

        let v = 0;
        let shift = 0;

        while (p < finalP) {
            const val = inValues[p];
            const c = (val >>> s) & 0xff;
            s += 8;
            p += s >>> 5;
            s &= 31;

            v += (c & 127) << shift;
            if ((c & 128) === 128) {
                out[tmpOutPos++] = v;
                v = 0;
                shift = 0;
            } else {
                shift += 7;
            }
        }

        outPos.set(tmpOutPos);
        inPos.add(inLength);
    }
}

class Composition {
    constructor(
        private readonly first: Int32Codec,
        private readonly second: Int32Codec,
    ) { }

    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        if (inLength === 0) return out;

        const inPosInit = inPos.get();
        const outPosInit = outPos.get();

        out = this.first.compress(inValues, inPos, inLength, out, outPos);
        if (outPos.get() === outPosInit) {
            out = ensureInt32Capacity(out, outPosInit + 1);
            out[outPosInit] = 0;
            outPos.increment();
        }

        const remaining = inLength - (inPos.get() - inPosInit);
        out = this.second.compress(inValues, inPos, remaining, out, outPos);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        if (inLength === 0) return;
        const init = inPos.get();
        this.first.uncompress(inValues, inPos, inLength, out, outPos);
        const remainingLength = inLength - (inPos.get() - init);
        this.second.uncompress(inValues, inPos, remainingLength, out, outPos);
    }
}

const fastPforCodec = new Composition(new FastPfor(), new VariableByte());

export function compressFastPforInt32(values: Int32Array): Int32Buf {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    let out = new Int32Array(values.length + 1024) as Int32Buf;
    out = fastPforCodec.compress(values, inPos, values.length, out, outPos);
    return out.subarray(0, outPos.get());
}

export function uncompressFastPforInt32(encoded: Int32Buf, numValues: number): Int32Array {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    const decoded = new Int32Array(numValues);
    fastPforCodec.uncompress(encoded, inPos, encoded.length, decoded, outPos);
    return decoded;
}

export function int32sToBigEndianBytes(values: Int32Buf): Uint8Array {
    // Note: the FastPFOR codec operates on an Int32 stream. When converted to bytes for the tile format,
    // we serialize those int32 words using big-endian order (consistent with existing MLT TS code paths).
    const bytes = new Uint8Array(values.length * 4);
    for (let i = 0; i < values.length; i++) {
        const v = values[i];
        const base = i * 4;
        bytes[base] = (v >>> 24) & 0xff;
        bytes[base + 1] = (v >>> 16) & 0xff;
        bytes[base + 2] = (v >>> 8) & 0xff;
        bytes[base + 3] = v & 0xff;
    }
    return bytes;
}

export function bigEndianBytesToInt32s(bytes: Uint8Array, offset: number, byteLength: number): Int32Buf {
    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    const ints = new Int32Array(numInts) as Int32Buf;
    if (numCompleteInts > 0) {
        const absoluteOffset = bytes.byteOffset + offset;
        if ((absoluteOffset & 3) === 0) {
            const u32 = new Uint32Array(bytes.buffer, absoluteOffset, numCompleteInts);
            for (let i = 0; i < numCompleteInts; i++) {
                ints[i] = bswap32(u32[i]) | 0;
            }
        } else {
            for (let i = 0; i < numCompleteInts; i++) {
                const base = offset + i * 4;
                ints[i] =
                    ((bytes[base] << 24) | (bytes[base + 1] << 16) | (bytes[base + 2] << 8) | bytes[base + 3]) |
                    0;
            }
        }
    }

    if (hasTrailingBytes) {
        const base = offset + numCompleteInts * 4;
        const remaining = byteLength - numCompleteInts * 4;
        let v = 0;
        for (let i = 0; i < remaining; i++) {
            v |= bytes[base + i] << (24 - i * 8);
        }
        ints[numCompleteInts] = v | 0;
    }
    return ints;
}

function bswap32(value: number): number {
    const x = value >>> 0;
    return (
        (((x & 0xff) << 24) |
            ((x & 0xff00) << 8) |
            ((x >>> 8) & 0xff00) |
            ((x >>> 24) & 0xff)) >>>
        0
    );
}
