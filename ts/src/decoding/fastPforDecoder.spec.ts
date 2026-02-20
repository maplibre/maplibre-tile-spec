import { describe, expect, it } from "vitest";

import {
    createFastPforEncoderWorkspace,
    encodeFastPforInt32,
    encodeFastPforInt32WithWorkspace,
} from "../encoding/fastPforEncoder";
import {
    createFastPforWireDecodeWorkspace,
    decodeFastPforInt32,
    ensureFastPforWireEncodedWordsCapacity,
} from "./fastPforDecoder";
import { BLOCK_SIZE } from "./fastPforShared";

describe("FastPFOR decoder", () => {
    it("throws on invalid alignedLength (negative)", () => {
        expect(() => decodeFastPforInt32(new Int32Array([-1]), 0)).toThrow(/invalid alignedLength/);
    });

    it("throws on invalid alignedLength (not multiple of 256)", () => {
        expect(() => decodeFastPforInt32(new Int32Array([1]), 0)).toThrow(/invalid alignedLength/);
    });

    it("throws when alignedLength exceeds output length", () => {
        expect(() => decodeFastPforInt32(new Int32Array([BLOCK_SIZE]), 10)).toThrow(/output buffer too small/);
    });

    it("round-trips empty input", () => {
        const values = new Int32Array(0);
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("decodes an empty encoded buffer", () => {
        const decoded = decodeFastPforInt32(new Int32Array(0), 0);
        expect(decoded).toEqual(new Int32Array(0));
    });

    it("throws when wire decode workspace capacity is negative", () => {
        expect(() => createFastPforWireDecodeWorkspace(-1)).toThrow(/must be >= 0/);
    });

    it("grows wire decode workspace encodedWords buffer on demand", () => {
        const workspace = createFastPforWireDecodeWorkspace(1);
        const initialCapacity = workspace.encodedWords.length;

        const reused = ensureFastPforWireEncodedWordsCapacity(workspace, initialCapacity);
        expect(reused).toBe(workspace.encodedWords);

        const grown = ensureFastPforWireEncodedWordsCapacity(workspace, initialCapacity + 1);
        expect(grown).toBe(workspace.encodedWords);
        expect(grown.length).toBeGreaterThan(initialCapacity);
    });

    it("round-trips VByte-only payload (<256 values)", () => {
        const values = new Int32Array(100);
        for (let i = 0; i < values.length; i++) values[i] = i * 7;
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips exactly one aligned block (256 values)", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips full-width signed values", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = -(i + 1);
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips representative block bit-widths", () => {
        const bitWidths = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13, 14, 15, 16, 31];

        for (const bitWidth of bitWidths) {
            const values = new Int32Array(BLOCK_SIZE);
            if (bitWidth !== 0) {
                const lowerBound = 2 ** (bitWidth - 1);
                const span = lowerBound;
                for (let i = 0; i < values.length; i++) {
                    values[i] = lowerBound + (i % span);
                }
            }

            const encoded = encodeFastPforInt32(values);
            const decoded = decodeFastPforInt32(encoded, values.length);
            expect(decoded, `round-trip mismatch for bitWidth=${bitWidth}`).toEqual(values);
        }
    });

    it("round-trips aligned blocks plus VByte tail", () => {
        const values = new Int32Array(BLOCK_SIZE * 2 + 3);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips values with outliers (exceptions path)", () => {
        const values = new Int32Array(BLOCK_SIZE * 2);
        for (let i = 0; i < values.length; i++) values[i] = i % 16;
        values[10] = 2_147_483_647;
        values[200] = 1_073_741_824;
        values[BLOCK_SIZE + 20] = 2_147_483_647;
        values[BLOCK_SIZE + 210] = 1_073_741_824;
        const encoded = encodeFastPforInt32(values);
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips exception streams across widths", () => {
        const exceptionBitWidths = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16, 32];

        for (const exceptionBitWidth of exceptionBitWidths) {
            const values = new Int32Array(BLOCK_SIZE);
            if (exceptionBitWidth === 32) {
                values[0] = -1;
            } else {
                for (let i = 0; i < values.length; i++) values[i] = i % 2;
                const outlier = 2 ** exceptionBitWidth;
                values[10] = outlier;
                values[100] = outlier;
            }
            const encoded = encodeFastPforInt32(values);
            const decoded = decodeFastPforInt32(encoded, values.length);
            expect(decoded, `round-trip mismatch for exceptionBitWidth=${exceptionBitWidth}`).toEqual(values);
        }
    });

    it("reuses encoder workspace across successive encodes", () => {
        const workspace = createFastPforEncoderWorkspace();

        const first = new Int32Array(BLOCK_SIZE + 8);
        for (let i = 0; i < first.length; i++) first[i] = i;

        const second = new Int32Array(16);
        for (let i = 0; i < second.length; i++) second[i] = 10_000 + i;

        const firstEncoded = encodeFastPforInt32WithWorkspace(first, workspace);
        const secondEncoded = encodeFastPforInt32WithWorkspace(second, workspace);

        expect(decodeFastPforInt32(firstEncoded, first.length)).toEqual(first);
        expect(decodeFastPforInt32(secondEncoded, second.length)).toEqual(second);
    });
});

describe("FastPFOR decoder error cases", () => {
    function getSinglePageWordLayout(encodedWords: Int32Array) {
        const firstPageHeaderWordIndex = 1;
        const metadataOffsetWordCount = encodedWords[firstPageHeaderWordIndex] | 0;
        const packedDataEndWordIndex = (firstPageHeaderWordIndex + metadataOffsetWordCount) | 0;
        const metadataByteLength = encodedWords[packedDataEndWordIndex] >>> 0;
        const metadataWordCount = (metadataByteLength + 3) >>> 2;
        const byteContainerStartWordIndex = (packedDataEndWordIndex + 1) | 0;
        const exceptionBitmapWordIndex = (byteContainerStartWordIndex + metadataWordCount) | 0;
        return { packedDataEndWordIndex, byteContainerStartWordIndex, exceptionBitmapWordIndex };
    }

    it("throws on truncated input (missing page data)", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32(values);
        const truncated = encoded.subarray(0, 5);
        expect(() => decodeFastPforInt32(truncated, values.length)).toThrow(/invalid whereMeta/);
    });

    it("throws on invalid whereMeta in page header", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 3;
        const encoded = encodeFastPforInt32(values);
        const corruptedEncoded = encoded.slice();
        corruptedEncoded[1] = 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid whereMeta/);
    });

    it("throws on invalid block bitWidth in byte container", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 7;
        const encoded = encodeFastPforInt32(values);
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        corruptedEncoded[byteContainerStartWordIndex] = ((blockHeaderWord & 0xffffff00) | 33) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid bitWidth/);
    });

    it("throws on packed region mismatch when block metadata is inconsistent", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32(values);
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        corruptedEncoded[byteContainerStartWordIndex] = (blockHeaderWord & 0xffffff00) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/packed region mismatch/);
    });

    it("throws on invalid maxBits in exception metadata", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i % 2;
        values[10] = 1 << 20;
        values[100] = 1 << 20;

        const encoded = encodeFastPforInt32(values);
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        const blockBitWidth = blockHeaderWord & 0xff;

        const invalidMaxBits = (blockBitWidth - 1) & 0xff;
        corruptedEncoded[byteContainerStartWordIndex] = ((blockHeaderWord & 0xff00ffff) | (invalidMaxBits << 16)) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid maxBits/);
    });

    it("throws on invalid byteSize that moves bitmap out of bounds", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i;
        const encoded = encodeFastPforInt32(values);
        const { packedDataEndWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[packedDataEndWordIndex] = 0x7fffffff;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid byteSize/);
    });

    it("throws on truncated exception stream header", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 11;
        const encoded = encodeFastPforInt32(values);
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[exceptionBitmapWordIndex] = corruptedEncoded[exceptionBitmapWordIndex] | (1 << 1);
        const truncatedEncoded = corruptedEncoded.subarray(0, exceptionBitmapWordIndex + 1);

        expect(() => decodeFastPforInt32(truncatedEncoded, values.length)).toThrow(/truncated exception stream header/);
    });

    it("throws on truncated exception stream payload", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 13;
        const encoded = encodeFastPforInt32(values);
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = new Int32Array(encoded.length + 1);
        corruptedEncoded.set(encoded);
        corruptedEncoded[exceptionBitmapWordIndex] = corruptedEncoded[exceptionBitmapWordIndex] | (1 << 1);
        corruptedEncoded[exceptionBitmapWordIndex + 1] = 1;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/truncated exception stream/);
    });

    it("throws on unterminated VByte value", () => {
        const encoded = new Int32Array([0, 0x7f7f7f7f, 0x0000007f]);
        expect(() => decodeFastPforInt32(encoded, 1)).toThrow(/unterminated value/);
    });

    it("throws when numValues exceeds decoded count", () => {
        const values = new Int32Array(100);
        for (let i = 0; i < values.length; i++) values[i] = i;
        const encoded = encodeFastPforInt32(values);
        expect(() => decodeFastPforInt32(encoded, 200)).toThrow(/truncated stream/);
    });
});
