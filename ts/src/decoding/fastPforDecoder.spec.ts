import { describe, expect, it } from "vitest";

import { encodeFastPforInt32 } from "../encoding/fastPforEncoder";
import { decodeFastPforInt32, createDecoderWorkspace } from "./fastPforDecoder";
import { BLOCK_SIZE } from "./fastPforSpec";

/**
 * Parses the first block header from an encoded FastPFOR page.
 * Used by tests to validate that test fixtures are structurally sound.
 *
 * @param encoded - The encoded FastPFOR page (`Int32Array`).
 * @returns The first block header fields.
 */
function readFirstBlockHeader(encoded: Int32Array): { b: number; cExcept: number; maxBits: number } {
    if (encoded.length < 2) {
        throw new Error(
            `readFirstBlockHeader: expected at least 2 int32 words ([outLength, whereMeta]), got encoded.length=${encoded.length}`,
        );
    }

    const whereMeta = encoded[1];
    if (whereMeta < 1) {
        throw new Error(`readFirstBlockHeader: invalid whereMeta=${whereMeta} (expected >= 1, encoded.length=${encoded.length})`);
    }

    const metaStart = 1 + whereMeta;
    if (metaStart >= encoded.length) {
        throw new Error(
            `readFirstBlockHeader: whereMeta=${whereMeta} points outside buffer (metaStart=${metaStart}, encoded.length=${encoded.length})`,
        );
    }

    const byteSize = encoded[metaStart];
    if (byteSize < 2) {
        throw new Error(`readFirstBlockHeader: invalid byteSize=${byteSize} (expected >= 2, encoded.length=${encoded.length})`);
    }

    const byteContainerStart = metaStart + 1;
    if (byteContainerStart >= encoded.length) {
        throw new Error(
            `readFirstBlockHeader: byteContainerStart=${byteContainerStart} out of bounds (encoded.length=${encoded.length})`,
        );
    }

    const metaInts = (byteSize + 3) >>> 2;
    if (byteContainerStart + metaInts > encoded.length) {
        throw new Error(
            `readFirstBlockHeader: byteContainer overflows buffer (byteContainerStart=${byteContainerStart}, metaInts=${metaInts}, encoded.length=${encoded.length})`,
        );
    }

    const firstWord = encoded[byteContainerStart];

    const b = firstWord & 0xff;
    const cExcept = (firstWord >>> 8) & 0xff;

    const minBytesForExceptions = 3 + cExcept;
    if (cExcept > 0 && byteSize < minBytesForExceptions) {
        throw new Error(
            `readFirstBlockHeader: invalid byteSize=${byteSize} for cExcept=${cExcept} (need >= ${minBytesForExceptions} bytes)`,
        );
    }

    const maxBits = cExcept > 0 ? (firstWord >>> 16) & 0xff : 0;

    return { b, cExcept, maxBits };
}

const wordCountForBits = (bitCount: number) => (bitCount + 31) >>> 5;

/**
 * Creates a one-block page (256 values) with bitwidth `bw` and no exceptions.
 *
 * @param bw - Bitwidth in the range 0..32.
 * @returns The encoded page and the expected decoded values.
 */
function createManualPackedPage(bw: number): { encoded: Int32Array; values: Int32Array } {
    const b = bw;
    const cExcept = 0;

    const values = new Int32Array(BLOCK_SIZE);
    const maxVal = bw === 32 ? -1 : ((2 ** bw) - 1) >>> 0;

    if (bw > 0) {
        for (let i = 0; i < BLOCK_SIZE; i++) {
            values[i] = (i ^ 0xAAAA5555) & maxVal;
        }
        values[0] = maxVal | 0;
    }

    const packedSize = wordCountForBits(BLOCK_SIZE * bw);
    const packedData = new Int32Array(packedSize);

    if (bw === 32) {
        packedData.set(values);
    } else if (bw > 0) {
        let bitOffset = 0;
        let wordIdx = 0;
        for (let i = 0; i < BLOCK_SIZE; i++) {
            const u = values[i] >>> 0;
            const masked = u & maxVal;

            const space = 32 - bitOffset;

            if (wordIdx >= packedData.length) {
                throw new Error(`Packer overflow (wordIdx=${wordIdx}, packedData.length=${packedData.length}, bw=${bw})`);
            }

            if (bw <= space) {
                packedData[wordIdx] |= (masked << bitOffset);
                bitOffset += bw;
                if (bitOffset === 32) { bitOffset = 0; wordIdx++; }
            } else {
                packedData[wordIdx] |= (masked << bitOffset);
                wordIdx++;
                if (wordIdx >= packedData.length) {
                    throw new Error(`Packer overflow (wordIdx=${wordIdx}, packedData.length=${packedData.length}, bw=${bw})`);
                }
                packedData[wordIdx] |= (masked >>> space);
                bitOffset = bw - space;
            }
        }
    }

    const bytes: number[] = [b, cExcept];
    while (bytes.length % 4 !== 0) bytes.push(0);

    const rawByteSize = 2;
    const metaIntsCount = bytes.length >>> 2;
    const byteContainerInts = new Int32Array(metaIntsCount);

    for (let i = 0; i < metaIntsCount; i++) {
        const base = i * 4;
        byteContainerInts[i] =
            (bytes[base]) |
            ((bytes[base + 1] || 0) << 8) |
            ((bytes[base + 2] || 0) << 16) |
            ((bytes[base + 3] || 0) << 24);
    }

    const totalSize = 1 + 1 + packedSize + 1 + metaIntsCount + 1;
    const encoded = new Int32Array(totalSize);
    let p = 0;

    encoded[p++] = BLOCK_SIZE;
    encoded[p++] = 1 + packedSize;

    encoded.set(packedData, p);
    p += packedSize;

    encoded[p++] = rawByteSize;
    encoded.set(byteContainerInts, p);
    p += metaIntsCount;

    encoded[p++] = 0;

    return { encoded, values };
}

/**
 * Creates a one-block page (256 values) with 2 exceptions at positions 0 and 128.
 *
 * @param bitwidth_k - Exception stream bitwidth (k).
 * @param b - Packed bitwidth (b).
 * @returns The encoded page.
 */
function createManualExceptionPage(bitwidth_k: number, b = 0): Int32Array {
    const cExcept = 2;
    const index = bitwidth_k - b;

    if (index < 1) throw new Error(`Invalid manual page params (bitwidth_k=${bitwidth_k}, b=${b}): expected bitwidth_k > b`);
    if (b > 32 || bitwidth_k > 32) throw new Error(`Invalid manual page params: bitwidth > 32 (bitwidth_k=${bitwidth_k}, b=${b})`);

    const exceptions = [0, BLOCK_SIZE >>> 1];

    const bytes: number[] = [];
    bytes.push(b);
    bytes.push(cExcept);
    bytes.push(bitwidth_k);
    bytes.push(exceptions[0]);
    bytes.push(exceptions[1]);

    const rawByteSize = bytes.length;

    while (bytes.length % 4 !== 0) bytes.push(0);

    const metaIntsCount = bytes.length >>> 2;
    const byteContainerInts = new Int32Array(metaIntsCount);
    for (let i = 0; i < metaIntsCount; i++) {
        const base = i * 4;
        byteContainerInts[i] =
            (bytes[base]) |
            ((bytes[base + 1] || 0) << 8) |
            ((bytes[base + 2] || 0) << 16) |
            ((bytes[base + 3] || 0) << 24);
    }

    let exceptionData = new Int32Array(0);

    if (index > 1) {
        const bitsTotal = cExcept * index;
        const intsNeeded = wordCountForBits(bitsTotal);
        exceptionData = new Int32Array(intsNeeded);

        if (index === 32) {
            exceptionData.fill(-1);
        } else {
            const exVal = ((2 ** index) - 1) >>> 0;

            let bitOffset = 0;
            let wordIdx = 0;

            for (let i = 0; i < cExcept; i++) {
                if (wordIdx >= exceptionData.length) {
                    throw new Error(
                        `Exception packer overflow (wordIdx=${wordIdx}, exceptionData.length=${exceptionData.length}, index=${index})`,
                    );
                }

                const space = 32 - bitOffset;
                if (index <= space) {
                    exceptionData[wordIdx] |= (exVal << bitOffset);
                    bitOffset += index;
                    if (bitOffset === 32) { bitOffset = 0; wordIdx++; }
                } else {
                    exceptionData[wordIdx] |= (exVal << bitOffset);
                    wordIdx++;
                    if (wordIdx >= exceptionData.length) {
                        throw new Error(
                            `Exception packer overflow (wordIdx=${wordIdx}, exceptionData.length=${exceptionData.length}, index=${index})`,
                        );
                    }

                    exceptionData[wordIdx] |= (exVal >>> space);
                    bitOffset = index - space;
                }
            }
        }
    }

    const packedSize = wordCountForBits(BLOCK_SIZE * b);
    const packedData = new Int32Array(packedSize);

    let bitmap = 0;
    if (index > 1) {
        bitmap = (index === 32) ? 0x80000000 | 0 : (1 << (index - 1));
    }

    const totalSize =
        1 + 1 + packedSize +
        1 + metaIntsCount +
        1 +
        (index > 1 ? (1 + exceptionData.length) : 0);

    const page = new Int32Array(totalSize);
    let p = 0;
    page[p++] = BLOCK_SIZE;

    const whereMeta = 1 + packedSize;
    page[p++] = whereMeta;

    page.set(packedData, p);
    p += packedSize;

    page[p++] = rawByteSize;
    page.set(byteContainerInts, p);
    p += metaIntsCount;

    page[p++] = bitmap;

    if (index > 1) {
        page[p++] = cExcept;
        page.set(exceptionData, p);
        p += exceptionData.length;
    }

    return page;
}

describe("FastPFOR Decoder: bitwidth dispatch (Manual Packed Pages)", () => {
    for (let bw = 0; bw <= 32; bw++) {
        it(`decodes manual packed block with bitwidth ${bw}`, () => {
            const { encoded, values } = createManualPackedPage(bw);

            const header = readFirstBlockHeader(encoded);
            expect(header.b).toBe(bw);
            expect(header.cExcept).toBe(0);

            const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);
            expect(decoded).toEqual(values);
        });
    }
});

describe("FastPFOR Decoder: exception logic (Manual Pages)", () => {
    it("handles exception optimization for index=1 (b=5, maxBits=6)", () => {
        const b = 5;
        const k = 6;
        const encoded = createManualExceptionPage(k, b);
        const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);

        const expected = new Int32Array(BLOCK_SIZE);
        const exceptionValue = (1 << b) | 0;
        expected[0] = exceptionValue;
        expected[BLOCK_SIZE >>> 1] = exceptionValue;

        expect(decoded).toEqual(expected);
    });

    for (let k = 2; k <= 32; k++) {
        it(`decodes manual page with exception stream k=${k} (b=0)`, () => {
            const encoded = createManualExceptionPage(k, 0);
            const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);

            const expected = new Int32Array(BLOCK_SIZE);
            const val = k === 32 ? -1 : ((2 ** k) - 1) >>> 0;
            expected[0] = val | 0;
            expected[BLOCK_SIZE >>> 1] = val | 0;

            expect(decoded).toEqual(expected);
        });
    }

    it("decodes manual page with mixed data: packed b=7 and exceptions k=13 (index=6)", () => {
        const b = 7;
        const k = 13;
        const encoded = createManualExceptionPage(k, b);
        const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);

        const expected = new Int32Array(BLOCK_SIZE);
        const index = k - b;
        const exAdder = ((((2 ** index) - 1) >>> 0) << b) | 0;

        expected[0] = exAdder;
        expected[BLOCK_SIZE >>> 1] = exAdder;

        expect(decoded).toEqual(expected);
    });
});

describe("FastPFOR Decoder: Corruption Guards", () => {
    const validValues = new Int32Array(BLOCK_SIZE).fill(123);
    const validEncoded = encodeFastPforInt32(validValues);

    it("throws if whereMeta is negative", () => {
        const corrupted = validEncoded.slice();
        corrupted[1] = -5;
        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if whereMeta is out of bounds", () => {
        const corrupted = validEncoded.slice();
        corrupted[1] = 999999;
        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if byteSize implies metaInts overflow", () => {
        const corrupted = validEncoded.slice();
        const whereMeta = corrupted[1];
        const metaStart = 1 + whereMeta;
        corrupted[metaStart] = 0x7FFFFFFF;
        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if block bitwidth b > 32", () => {
        const corrupted = validEncoded.slice();
        const whereMeta = corrupted[1];
        const bcStart = 1 + whereMeta + 1;

        const word = corrupted[bcStart];
        const newWord = (word & 0xFFFFFF00) | 33;
        corrupted[bcStart] = newWord;

        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if maxBits < b", () => {
        const b = 10;
        const cExcept = 1;
        const maxBits = 5;
        const val = b | (cExcept << 8) | (maxBits << 16);

        const packedSize = (BLOCK_SIZE * b + 31) >>> 5;

        const page = new Int32Array(10 + packedSize);
        page[0] = BLOCK_SIZE;
        page[1] = 1 + packedSize;

        const metaStart = 2 + packedSize;
        page[metaStart] = 4;
        page[metaStart + 1] = val;
        page[metaStart + 2] = 0;

        expect(() => decodeFastPforInt32(page, BLOCK_SIZE)).toThrow();
    });

    it("throws on packed region mismatch", () => {
        const valid = encodeFastPforInt32(new Int32Array(BLOCK_SIZE).fill(0));

        const corrupted = new Int32Array(valid.length + 10);
        corrupted.set(valid);

        const wm = valid[1];
        const metaPart = valid.subarray(1 + wm);
        corrupted.set(metaPart, 1 + wm + 5);
        corrupted[1] = wm + 5;

        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if output buffer is too small (capacity check)", () => {
        const valid = encodeFastPforInt32(new Int32Array(BLOCK_SIZE).fill(10));
        expect(() => decodeFastPforInt32(valid, 10)).toThrow();
    });

    it("throws if exception stream is truncated", () => {
        const b = 0;
        const k = 5;
        const valid = createManualExceptionPage(k, b);

        const truncated = valid.slice(0, valid.length - 1);

        expect(() => decodeFastPforInt32(truncated, BLOCK_SIZE)).toThrow();
    });

    it("throws on truncated VByte stream", () => {
        const vals = new Int32Array(BLOCK_SIZE + 1).fill(1);
        const encoded = encodeFastPforInt32(vals);
        const truncated = encoded.slice(0, encoded.length - 1);
        expect(() => decodeFastPforInt32(truncated, BLOCK_SIZE + 1)).toThrow();
    });

    it("throws on corrupted VByte (varint too long)", () => {
        const vals = new Int32Array(BLOCK_SIZE + 1).fill(1);
        const encoded = encodeFastPforInt32(vals);

        const corrupted = new Int32Array(encoded.length + 10);
        corrupted.set(encoded);
        corrupted[encoded.length - 1] = 0;

        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE + 1)).toThrow();
    });
});

describe("FastPFOR Decoder: workspace management", () => {
    it("supports explicit workspace passed to decoder", () => {
        const ws = createDecoderWorkspace();
        const v1 = new Int32Array(BLOCK_SIZE).fill(10);
        const e1 = encodeFastPforInt32(v1);
        const d1 = decodeFastPforInt32(e1, BLOCK_SIZE, ws);
        expect(d1).toEqual(v1);

        const v2 = new Int32Array(BLOCK_SIZE).fill(20);
        const e2 = encodeFastPforInt32(v2);
        const d2 = decodeFastPforInt32(e2, BLOCK_SIZE, ws);
        expect(d2).toEqual(v2);
    });
});
