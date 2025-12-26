/**
 * FastPFOR Decoder Unit Tests
 *
 * Tests for the FastPFOR decoder - the production hot-path.
 * Tests bitwidth dispatch, edge cases, and workspace management.
 *
 * DESIGN NOTE:
 * These tests prioritize DETERMINISM.
 * - Bitwidth tests use "Manual Packed Pages" to verify decoder dispatch without encoder dependency.
 * - Exception tests use "Manual Exception Pages" to test exception logic directly.
 * - Corruption tests take a valid encoded buffer and surgically modify fields.
 */

import { describe, expect, it } from "vitest";

import { encodeFastPforInt32 } from "../encoding/fastPforEncoder";
import { decodeFastPforInt32, createDecoderWorkspace } from "./fastPforDecoder";
import { BLOCK_SIZE } from "./fastPforSpec";

// Helper to parse the first block header purely for verification
function readFirstBlockHeader(encoded: Int32Array) {
    if (encoded.length < 2) throw new Error("Buffer too short");

    // Encoded format: [outLength, whereMeta, ...packed..., byteSize, ...byteContainer...]
    const whereMeta = encoded[1];
    // whereMeta must be at least 1 (pointing passed itself). 0 would mean meta starts at 1, overlapping whereMeta.
    if (whereMeta < 1) throw new Error(`Invalid whereMeta ${whereMeta}`);

    const metaStart = 1 + whereMeta;
    if (metaStart >= encoded.length) {
        throw new Error(`Invalid whereMeta ${whereMeta}: points outside buffer`);
    }

    const byteSize = encoded[metaStart];
    if (byteSize < 2) {
        throw new Error(`Invalid byteSize ${byteSize}: too small for header`);
    }

    const byteContainerStart = metaStart + 1;
    // byteContainerStart must be valid if we are to read from it
    if (byteContainerStart >= encoded.length) {
        throw new Error("ByteContainer start out of bounds");
    }

    // New Check: Ensure byteContainer has enough data for byteSize
    const metaInts = (byteSize + 3) >>> 2;
    if (byteContainerStart + metaInts > encoded.length) {
        throw new Error(`ByteContainer end out of bounds: needed ${metaInts} ints`);
    }

    // Read first few bytes from byteContainer to get block header
    // byteContainer is stored as Little Endian int32s
    const firstWord = encoded[byteContainerStart];

    const b = firstWord & 0xff;
    const cExcept = (firstWord >>> 8) & 0xff;

    // Safety check: if exceptions exist, header is 3 bytes (b, c, maxBits) + positions...
    const minBytesForExceptions = 3 + cExcept;
    if (cExcept > 0 && byteSize < minBytesForExceptions) {
        throw new Error(`Invalid byteSize ${byteSize} for cExcept=${cExcept} (need >= ${minBytesForExceptions})`);
    }

    // Only read maxBits if exceptions exist
    const maxBits = cExcept > 0 ? (firstWord >>> 16) & 0xff : 0;

    return { b, cExcept, maxBits };
}

// Helper to calculate packed size in 32-bit words
// ceil(bits / 32)
const getPackedInts = (bits: number) => (bits + 31) >>> 5;

// Helper to manually construct a basic FastPFOR page with NO exceptions
// This allows testing the bitwidth dispatch logic completely independently of the encoder.
function createManualPackedPage(bw: number): { encoded: Int32Array, values: Int32Array } {
    // b = bw
    // cExcept = 0
    // byteSize = 2 (b, cExcept)
    // No exceptions, no maxBits in header.
    // Packed data: we use a deterministic pattern.

    const b = bw;
    const cExcept = 0;

    const values = new Int32Array(BLOCK_SIZE);
    const maxVal = bw === 32 ? -1 : ((2 ** bw) - 1) >>> 0;

    if (bw > 0) {
        for (let i = 0; i < BLOCK_SIZE; i++) {
            // Use a simple pattern: i modulo something, masked by maxVal
            // + Ensure we use the full bit range to verify unpacking
            values[i] = (i ^ 0xAAAA5555) & maxVal;
        }
        // Force at least one value to be exactly maxVal to test full range
        values[0] = maxVal | 0;
    }

    // 1. Pack Data (Strict bit-level packing)
    const packedSize = getPackedInts(BLOCK_SIZE * bw);
    const packedData = new Int32Array(packedSize);

    if (bw === 32) {
        // Optimization: Direct copy for 32-bit width to avoid bitwise shift ambiguities
        packedData.set(values);
    } else if (bw > 0) {
        let bitOffset = 0;
        let wordIdx = 0;
        for (let i = 0; i < BLOCK_SIZE; i++) {
            const u = values[i] >>> 0;        // Unsigned view
            const masked = u & maxVal;        // Explicit masking (redundant but safe)

            const space = 32 - bitOffset;

            // Safety guard for packer correctness
            if (wordIdx >= packedData.length) throw new Error("Packer overflow");

            if (bw <= space) {
                packedData[wordIdx] |= (masked << bitOffset);
                bitOffset += bw;
                if (bitOffset === 32) { bitOffset = 0; wordIdx++; }
            } else {
                packedData[wordIdx] |= (masked << bitOffset);
                wordIdx++;
                // Safety guard before second write
                if (wordIdx >= packedData.length) throw new Error("Packer overflow");
                packedData[wordIdx] |= (masked >>> space);
                bitOffset = bw - space;
            }
        }
    }

    // 2. Build ByteContainer
    const bytes: number[] = [b, cExcept];
    // Pad to 4 bytes
    while (bytes.length % 4 !== 0) bytes.push(0);

    const rawByteSize = 2;
    const metaIntsCount = bytes.length >>> 2;
    const byteContainerInts = new Int32Array(metaIntsCount);

    // Little endian pack
    for (let i = 0; i < metaIntsCount; i++) {
        const base = i * 4;
        byteContainerInts[i] =
            (bytes[base]) |
            ((bytes[base + 1] || 0) << 8) |
            ((bytes[base + 2] || 0) << 16) |
            ((bytes[base + 3] || 0) << 24);
    }

    // 3. Assemble
    // [Len] [WhereMeta] [Packed...] [ByteSize] [ByteContainer...] [Bitmap] (no exception streams when bitmap=0)

    const totalSize = 1 + 1 + packedSize + 1 + metaIntsCount + 1; // + bitmap
    const encoded = new Int32Array(totalSize);
    let p = 0;

    encoded[p++] = BLOCK_SIZE;
    encoded[p++] = 1 + packedSize; // WhereMeta

    encoded.set(packedData, p);
    p += packedSize;

    encoded[p++] = rawByteSize;
    encoded.set(byteContainerInts, p);
    p += metaIntsCount;

    encoded[p++] = 0; // Bitmap (0)

    return { encoded, values };
}

// Helper to manually construct valid FastPFOR pages for testing logic directly
function createManualExceptionPage(bitwidth_k: number, b = 0): Int32Array {
    // We construct a page with:
    // - b (packed bitwidth)
    // - cExcept = 2 (exceptions at 0 and BLOCK_SIZE/2)
    // - maxBits = bitwidth_k
    // - index = bitwidth_k - b

    const cExcept = 2;
    const index = bitwidth_k - b;

    if (index < 1) throw new Error("Invalid manual page params: k <= b");
    if (b > 32 || bitwidth_k > 32) throw new Error("Bitwidth > 32 not supported in manual page helper");

    // Exceptions positions
    const exceptions = [0, BLOCK_SIZE >>> 1];

    // 1. Build ByteContainer (Header + Positions)
    const bytes: number[] = [];
    bytes.push(b);
    bytes.push(cExcept);
    bytes.push(bitwidth_k); // maxBits
    bytes.push(exceptions[0]);
    bytes.push(exceptions[1]);

    const rawByteSize = bytes.length;

    // Pad to multiple of 4 for int32 storage
    while (bytes.length % 4 !== 0) bytes.push(0);

    // Explicit Little Endian packing
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

    // 2. Exception Stream (Only if index > 1)
    // If index == 1, exception value is implicit (single bit `b` set). No stream data.
    let exceptionData = new Int32Array(0);

    if (index > 1) {
        // We pack values of width `index`.
        const bitsTotal = cExcept * index;
        const intsNeeded = getPackedInts(bitsTotal);
        exceptionData = new Int32Array(intsNeeded);

        if (index === 32) {
            // Fill with all 1s (-1) if exception width is 32
            exceptionData.fill(-1);
        } else {
            // Pattern: All 1s for the exception width.
            const exVal = ((2 ** index) - 1) >>> 0;

            let bitOffset = 0;
            let wordIdx = 0;

            for (let i = 0; i < cExcept; i++) { // 2 exceptions
                // Safety guard for packer correctness
                if (wordIdx >= exceptionData.length) throw new Error("Exception packer overflow");

                const space = 32 - bitOffset;
                if (index <= space) {
                    exceptionData[wordIdx] |= (exVal << bitOffset);
                    bitOffset += index;
                    if (bitOffset === 32) { bitOffset = 0; wordIdx++; }
                } else {
                    exceptionData[wordIdx] |= (exVal << bitOffset);
                    wordIdx++;
                    // Safety guard before second write
                    if (wordIdx >= exceptionData.length) throw new Error("Exception packer overflow");

                    exceptionData[wordIdx] |= (exVal >>> space);
                    bitOffset = index - space;
                }
            }
        }
    }

    // 3. Packed Region
    const packedSize = getPackedInts(BLOCK_SIZE * b);
    const packedData = new Int32Array(packedSize);

    // 4. Bitmap
    // If index > 1, check bit `index-1`.
    let bitmap = 0;
    if (index > 1) {
        bitmap = (index === 32) ? 0x80000000 | 0 : (1 << (index - 1));
    }

    // 5. Assemble Buffer
    // [Len] [WhereMeta] [Packed...] [ByteSize] [ByteContainer...] [Bitmap] [StreamSize] [Stream...]
    const totalSize =
        1 + 1 + packedSize +
        1 + metaIntsCount +
        1 +
        (index > 1 ? (1 + exceptionData.length) : 0);

    const page = new Int32Array(totalSize);
    let p = 0;
    page[p++] = BLOCK_SIZE;          // outLength

    const whereMeta = 1 + packedSize; // relative to p (Index 1 is start)
    page[p++] = whereMeta;

    page.set(packedData, p);
    p += packedSize;

    page[p++] = rawByteSize; // Store true byte size, not padded size
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
    // We construct pages manually to strictly test the decoder's dispatch logic
    // without depending on the encoder's heuristics.

    for (let bw = 0; bw <= 32; bw++) {
        it(`decodes manual packed block with bitwidth ${bw}`, () => {
            const { encoded, values } = createManualPackedPage(bw);

            // Critical: Verify our manual page is well-formed regarding the header
            const header = readFirstBlockHeader(encoded);
            expect(header.b).toBe(bw);
            expect(header.cExcept).toBe(0);

            const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);
            expect(decoded).toEqual(values);
        });
    }
});

describe("FastPFOR Decoder: exception logic (Manual Pages)", () => {
    // To strictly test the decoder's exception handling without relying on encoder heuristics,
    // we manually construct valid FastPFOR pages.

    // 1. Test index=1 specific case (Optimized branch in decoder)
    it("handles exception optimization for index=1 (b=5, maxBits=6)", () => {
        const b = 5;
        const k = 6;
        const encoded = createManualExceptionPage(k, b);
        const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);

        const expected = new Int32Array(BLOCK_SIZE);
        // Packed region is 0.
        // Exception at 0: adds 1<<b = 32. Total 32.
        expected[0] = 32;
        expected[BLOCK_SIZE >>> 1] = 32;

        expect(decoded).toEqual(expected);
    });

    // 2. Test generic exception loop for k=2..32 (forcing b=0 for simplicity)
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

    // 3. Test mixed case: b > 0 AND index > 1 (Both packed data and exception stream)
    it("decodes manual page with mixed data: packed b=7 and exceptions k=13 (index=6)", () => {
        const b = 7;
        const k = 13; // index = 6
        const encoded = createManualExceptionPage(k, b);
        const decoded = decodeFastPforInt32(encoded, BLOCK_SIZE);

        const expected = new Int32Array(BLOCK_SIZE);
        // Page packed with 0s.
        // Exceptions add value `(2^index)-1` shifted by `b`
        // Validation formula: (2^6 - 1) << 7
        const exAdder = ((((2 ** (k - b)) - 1) >>> 0) << b) | 0;

        expected[0] = exAdder;
        expected[BLOCK_SIZE >>> 1] = exAdder;

        expect(decoded).toEqual(expected);
    });
});

describe("FastPFOR Decoder: Corruption Guards", () => {
    // We use a valid encoded buffer and surgically corrupt it to verifying throwing guards.
    // For corruption tests, verifying against "perfectly valid" structural encoded data
    // is best achieved by asking the encoder to produce it, then corrupting it.
    const validValues = new Int32Array(BLOCK_SIZE).fill(123);
    const validEncoded = encodeFastPforInt32(validValues);

    // Layout: [Len] [WhereMeta] ... [ByteSize] ...

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
        // Corrupt byteSize
        corrupted[metaStart] = 0x7FFFFFFF;
        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if block bitwidth b > 32", () => {
        const corrupted = validEncoded.slice();
        const whereMeta = corrupted[1];
        const bcStart = 1 + whereMeta + 1;

        // Retrieve and corrupt block header word
        const word = corrupted[bcStart];
        const newWord = (word & 0xFFFFFF00) | 33; // b=33
        corrupted[bcStart] = newWord;

        expect(() => decodeFastPforInt32(corrupted, BLOCK_SIZE)).toThrow();
    });

    it("throws if maxBits < b", () => {
        // To test this properly, we must construct a STRUCTURALLY VALID page.
        // If packed region is too small for b, 'packed region mismatch' might fire first.

        const b = 10;
        const cExcept = 1;
        const maxBits = 5; // Invalid: < b

        // Bytes: [b, c, max, ...pos...]
        const val = b | (cExcept << 8) | (maxBits << 16);

        // Calculate packed size for b=10
        const packedSize = (BLOCK_SIZE * b + 31) >>> 5;

        const page = new Int32Array(10 + packedSize);
        page[0] = BLOCK_SIZE;
        page[1] = 1 + packedSize; // WhereMeta points after packed data

        // Fill packed region (zeros are fine)
        // page indices 2 ... 2+packedSize-1

        const metaStart = 2 + packedSize;
        page[metaStart] = 4; // byteSize
        page[metaStart + 1] = val; // msg (invalid maxBits)

        // Bitmap (empty)
        page[metaStart + 2] = 0;

        expect(() => decodeFastPforInt32(page, BLOCK_SIZE)).toThrow();
    });

    it("throws on packed region mismatch", () => {
        // We take a valid b=0 page (size 0 packed region) and assert logic check.
        const valid = encodeFastPforInt32(new Int32Array(BLOCK_SIZE).fill(0));

        const corrupted = new Int32Array(valid.length + 10);
        corrupted.set(valid);

        // Move metadata further down to create a "gap"
        const wm = valid[1];
        const metaPart = valid.subarray(1 + wm);
        corrupted.set(metaPart, 1 + wm + 5);
        // Update whereMeta
        corrupted[1] = wm + 5;

        // tmpInPos (decoder pos) will be < packedEnd (calculated from whereMeta)
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

        // Physically reduce input buffer size (cutoff last word of exception stream)
        const truncated = valid.slice(0, valid.length - 1);

        // This should trigger the new 'buffer overflow' guard when trying to read stream words
        expect(() => decodeFastPforInt32(truncated, BLOCK_SIZE)).toThrow();
    });

    it("throws on truncated VByte stream", () => {
        const vals = new Int32Array(BLOCK_SIZE + 1).fill(1);
        const encoded = encodeFastPforInt32(vals);
        // slice off the last int which contains the VByte data
        const truncated = encoded.slice(0, encoded.length - 1);
        expect(() => decodeFastPforInt32(truncated, BLOCK_SIZE + 1)).toThrow();
    });

    it("throws on corrupted VByte (varint too long)", () => {
        const vals = new Int32Array(BLOCK_SIZE + 1).fill(1);
        const encoded = encodeFastPforInt32(vals);

        const corrupted = new Int32Array(encoded.length + 10);
        corrupted.set(encoded);
        // Overwrite the VByte part with 0 (no MSB bit set, never terminates)
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
