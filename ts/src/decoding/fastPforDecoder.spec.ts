import { describe, expect, it } from "vitest";

import { createFastPforEncoderWorkspace, encodeFastPforInt32WithWorkspace } from "../encoding/fastPforEncoder";
import {
    createDecoderWorkspace,
    createFastPforWireDecodeWorkspace,
    decodeFastPforInt32,
    ensureFastPforWireEncodedWordsCapacity,
} from "./fastPforDecoder";
import { BLOCK_SIZE } from "./fastPforShared";

const LARGE_OUTLIER_VALUE = 1_048_576;
const DEFAULT_OUTLIER_VALUE = 4;
const HEADER_BYTE_MASK = 0xff;
const EXCEPTION_COUNT_SHIFT = 8;
const MAX_BITS_SHIFT = 16;

function createSingleBlockValuesWithExceptionOutliers(outlierValue: number): Int32Array {
    const values = new Int32Array(BLOCK_SIZE);
    for (let i = 0; i < values.length; i++) values[i] = i % 2;
    values[10] = outlierValue;
    values[100] = outlierValue;
    return values;
}

function getSinglePageWordLayout(encodedWords: Int32Array) {
    const metadataOffsetWordIndex = 1;
    const metadataOffsetInWords = encodedWords[metadataOffsetWordIndex] | 0;
    const packedDataEndWordIndex = (metadataOffsetWordIndex + metadataOffsetInWords) | 0;
    const metadataByteLength = encodedWords[packedDataEndWordIndex] >>> 0;
    const metadataWordCount = (metadataByteLength + 3) >>> 2;
    const byteContainerStartWordIndex = (packedDataEndWordIndex + 1) | 0;
    const exceptionBitmapWordIndex = (byteContainerStartWordIndex + metadataWordCount) | 0;
    return { packedDataEndWordIndex, byteContainerStartWordIndex, exceptionBitmapWordIndex };
}

function parseBlockHeaderWord(headerWord: number): { bitWidth: number; exceptionCount: number; maxBits: number } {
    return {
        bitWidth: headerWord & HEADER_BYTE_MASK,
        exceptionCount: (headerWord >>> EXCEPTION_COUNT_SHIFT) & HEADER_BYTE_MASK,
        maxBits: (headerWord >>> MAX_BITS_SHIFT) & HEADER_BYTE_MASK,
    };
}

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
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
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
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips exactly one aligned block (256 values)", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips full-width signed values", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = -(i + 1);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips representative block bit-widths", () => {
        const bitWidthCases = [
            { bitWidth: 0, samples: [0] },
            { bitWidth: 1, samples: [0, 1, 0, 1] },
            { bitWidth: 2, samples: [2, 3, 2, 3] },
            { bitWidth: 3, samples: [4, 5, 6, 7] },
            { bitWidth: 4, samples: [8, 9, 10, 11] },
            { bitWidth: 5, samples: [16, 17, 18, 19] },
            { bitWidth: 6, samples: [32, 33, 34, 35] },
            { bitWidth: 7, samples: [64, 65, 66, 67] },
            { bitWidth: 8, samples: [128, 129, 130, 131] },
            { bitWidth: 9, samples: [256, 257, 258, 259] },
            { bitWidth: 12, samples: [2_048, 2_049, 2_050, 2_051] },
            { bitWidth: 13, samples: [4_096, 4_097, 4_098, 4_099] },
            { bitWidth: 14, samples: [8_192, 8_193, 8_194, 8_195] },
            { bitWidth: 15, samples: [16_384, 16_385, 16_386, 16_387] },
            { bitWidth: 16, samples: [32_768, 32_769, 32_770, 32_771] },
            { bitWidth: 31, samples: [1_073_741_824, 1_073_741_825, 1_073_741_826, 1_073_741_827] },
        ] as const;

        for (const { bitWidth, samples } of bitWidthCases) {
            const values = new Int32Array(BLOCK_SIZE);
            for (let i = 0; i < values.length; i++) values[i] = samples[i % samples.length];

            const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
            const decoded = decodeFastPforInt32(encoded, values.length);
            expect(decoded, `round-trip mismatch for bitWidth=${bitWidth}`).toEqual(values);
        }
    });

    it("round-trips aligned blocks plus VByte tail", () => {
        const values = new Int32Array(BLOCK_SIZE * 2 + 3);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
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
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });

    it("round-trips exception streams across widths", () => {
        const exceptionBitWidthCases = [
            { exceptionBitWidth: 2, outlierValue: 4 },
            { exceptionBitWidth: 3, outlierValue: 8 },
            { exceptionBitWidth: 4, outlierValue: 16 },
            { exceptionBitWidth: 5, outlierValue: 32 },
            { exceptionBitWidth: 6, outlierValue: 64 },
            { exceptionBitWidth: 7, outlierValue: 128 },
            { exceptionBitWidth: 8, outlierValue: 256 },
            { exceptionBitWidth: 9, outlierValue: 512 },
            { exceptionBitWidth: 10, outlierValue: 1_024 },
            { exceptionBitWidth: 11, outlierValue: 2_048 },
            { exceptionBitWidth: 12, outlierValue: 4_096 },
            { exceptionBitWidth: 13, outlierValue: 8_192 },
            { exceptionBitWidth: 16, outlierValue: 65_536 },
            { exceptionBitWidth: 32, outlierValue: -1 },
        ] as const;

        for (const { exceptionBitWidth, outlierValue } of exceptionBitWidthCases) {
            const values = new Int32Array(BLOCK_SIZE);
            if (exceptionBitWidth === 32) {
                values[0] = outlierValue;
            } else {
                for (let i = 0; i < values.length; i++) values[i] = i % 2;
                values[10] = outlierValue;
                values[100] = outlierValue;
            }
            const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
            const decoded = decodeFastPforInt32(encoded, values.length);
            expect(decoded, `round-trip mismatch for exceptionBitWidth=${exceptionBitWidth}`).toEqual(values);
        }
    });

    it("round-trips exceptionBitWidth=1 fast-path", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i & 1;
        values[42] = 2;
        values[128] = 2;

        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);
        const headerWord = encoded[byteContainerStartWordIndex] >>> 0;
        const blockHeader = parseBlockHeaderWord(headerWord);

        expect(blockHeader.exceptionCount).toBeGreaterThan(0);
        expect(blockHeader.maxBits).toBe(blockHeader.bitWidth + 1);

        const decoded = decodeFastPforInt32(encoded, values.length);
        expect(decoded).toEqual(values);
    });
});

describe("FastPFOR decoder error cases", () => {
    function withForcedByteSizeAndNoExceptionStreams(encoded: Int32Array, forcedByteSize: number): Int32Array {
        const corrupted = encoded.slice();
        const { packedDataEndWordIndex, byteContainerStartWordIndex } = getSinglePageWordLayout(corrupted);
        corrupted[packedDataEndWordIndex] = forcedByteSize;

        const bitmapWordIndex = byteContainerStartWordIndex + ((forcedByteSize + 3) >>> 2);
        corrupted[bitmapWordIndex] = 0;
        return corrupted;
    }

    it("throws on truncated input (missing page data)", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const truncated = encoded.subarray(0, 5);
        expect(() => decodeFastPforInt32(truncated, values.length)).toThrow(/invalid whereMeta/);
    });

    it("throws on invalid whereMeta in page header", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 3;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const corruptedEncoded = encoded.slice();
        corruptedEncoded[1] = 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid whereMeta/);
    });

    it("throws on invalid block bitWidth in byte container", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 7;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        corruptedEncoded[byteContainerStartWordIndex] = ((blockHeaderWord & 0xffffff00) | 33) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid bitWidth/);
    });

    it("throws on packed region mismatch when block metadata is inconsistent", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 31;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        corruptedEncoded[byteContainerStartWordIndex] = (blockHeaderWord & 0xffffff00) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/packed region mismatch/);
    });

    it("throws on invalid maxBits in exception metadata", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(LARGE_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        const { bitWidth: blockBitWidth } = parseBlockHeaderWord(blockHeaderWord);

        const invalidMaxBits = (blockBitWidth - 1) & HEADER_BYTE_MASK;
        const clearMaxBitsMask = ~(HEADER_BYTE_MASK << MAX_BITS_SHIFT);
        corruptedEncoded[byteContainerStartWordIndex] =
            ((blockHeaderWord & clearMaxBitsMask) | (invalidMaxBits << MAX_BITS_SHIFT)) | 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid maxBits/);
    });

    it("throws on invalid byteSize that moves bitmap out of bounds", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { packedDataEndWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[packedDataEndWordIndex] = 0x7fffffff;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid byteSize/);
    });

    it("throws on truncated exception stream header", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 11;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[exceptionBitmapWordIndex] = corruptedEncoded[exceptionBitmapWordIndex] | (1 << 1);
        const truncatedEncoded = corruptedEncoded.subarray(0, exceptionBitmapWordIndex + 1);

        expect(() => decodeFastPforInt32(truncatedEncoded, values.length)).toThrow(/truncated exception stream header/);
    });

    it("throws on truncated exception stream payload", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * 13;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = new Int32Array(encoded.length + 1);
        corruptedEncoded.set(encoded);
        corruptedEncoded[exceptionBitmapWordIndex] = corruptedEncoded[exceptionBitmapWordIndex] | (1 << 1);
        corruptedEncoded[exceptionBitmapWordIndex + 1] = 1;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/truncated exception stream/);
    });

    it("throws when byteSize is too small for exception metadata", () => {
        const smallByteSizeCases = [
            { forcedByteSize: 1, expectedError: /byteContainer underflow/ },
            { forcedByteSize: 2, expectedError: /exception header underflow/ },
            { forcedByteSize: 4, expectedError: /exception positions underflow/ },
        ];

        for (const { forcedByteSize, expectedError } of smallByteSizeCases) {
            const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
            const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
            const corruptedEncoded = withForcedByteSizeAndNoExceptionStreams(encoded, forcedByteSize);

            expect(() => decodeFastPforInt32(corruptedEncoded, values.length), `forcedByteSize=${forcedByteSize}`).toThrow(
                expectedError,
            );
        }
    });

    it("throws when maxBits equals bitWidth but exceptions are present", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { byteContainerStartWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        const blockHeaderWord = corruptedEncoded[byteContainerStartWordIndex] >>> 0;
        const { bitWidth: blockBitWidth } = parseBlockHeaderWord(blockHeaderWord);
        const bytes = new Uint8Array(corruptedEncoded.buffer, corruptedEncoded.byteOffset, corruptedEncoded.byteLength);
        bytes[byteContainerStartWordIndex * 4 + 2] = blockBitWidth & HEADER_BYTE_MASK;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/invalid exceptionBitWidth=0/);
    });

    it("throws when exception stream is missing for declared exceptionBitWidth", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[exceptionBitmapWordIndex] = 0;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/missing exception stream/);
    });

    it("throws when exception stream pointer overflows stream size", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const { exceptionBitmapWordIndex } = getSinglePageWordLayout(encoded);

        const corruptedEncoded = encoded.slice();
        corruptedEncoded[exceptionBitmapWordIndex + 1] = 1;

        expect(() => decodeFastPforInt32(corruptedEncoded, values.length)).toThrow(/exception stream overflow/);
    });

    it("throws on unterminated VByte value", () => {
        const encoded = new Int32Array([0, 0x7f7f7f7f, 0x0000007f]);
        expect(() => decodeFastPforInt32(encoded, 1)).toThrow(/unterminated value/);
    });

    it("throws when numValues exceeds decoded count", () => {
        const values = new Int32Array(100);
        for (let i = 0; i < values.length; i++) values[i] = i;
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        expect(() => decodeFastPforInt32(encoded, 200)).toThrow(/truncated stream/);
    });
});

describe("FastPFOR decoder workspace paths", () => {
    it("reallocates byteContainer when provided workspace is too small", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const workspace = createDecoderWorkspace();
        workspace.byteContainer = new Uint8Array(1);
        workspace.byteContainerI32 = undefined;

        const decoded = decodeFastPforInt32(encoded, values.length, workspace);
        expect(decoded).toEqual(values);
        expect(workspace.byteContainer.length).toBeGreaterThan(1);
    });

    it("handles unaligned byteContainer workspace via byte fallback copy path", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const workspace = createDecoderWorkspace();
        const unalignedByteContainer = new Uint8Array(new ArrayBuffer(2049), 1, 2048);
        workspace.byteContainer = unalignedByteContainer;
        workspace.byteContainerI32 = undefined;

        const decoded = decodeFastPforInt32(encoded, values.length, workspace);
        expect(decoded).toEqual(values);
        expect(workspace.byteContainer.byteOffset & 3).toBe(1);
    });

    it("rebuilds aligned int view when workspace has stale int32 view", () => {
        const values = createSingleBlockValuesWithExceptionOutliers(DEFAULT_OUTLIER_VALUE);
        const encoded = encodeFastPforInt32WithWorkspace(values, createFastPforEncoderWorkspace());
        const workspace = createDecoderWorkspace();
        workspace.byteContainer = new Uint8Array(4096);
        workspace.byteContainerI32 = new Int32Array(4);

        const decoded = decodeFastPforInt32(encoded, values.length, workspace);
        expect(decoded).toEqual(values);
        const byteContainerI32 = workspace.byteContainerI32;
        expect(byteContainerI32).toBeDefined();
        if (!byteContainerI32) throw new Error("expected byteContainerI32 to be rebuilt");
        expect(byteContainerI32.buffer).toBe(workspace.byteContainer.buffer);
        expect(byteContainerI32.byteOffset).toBe(workspace.byteContainer.byteOffset);
    });

});
