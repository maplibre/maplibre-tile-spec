import { describe, expect, it } from "vitest";
import IntWrapper from "../../decoding/intWrapper";
import { createRleMetadata, createStreamMetadata, encodeStreamMetadata } from "../../decoding/decodingTestUtils";
import { encodeVarintInt32 } from "../../encoding/integerEncodingUtils";
import { LogicalLevelTechnique } from "./logicalLevelTechnique";
import { PhysicalLevelTechnique } from "./physicalLevelTechnique";
import { decodeStreamMetadata } from "./streamMetadataDecoder";

function concatenate(...buffers: Uint8Array[]): Uint8Array {
    const totalLength = buffers.reduce((sum, buffer) => sum + buffer.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const buffer of buffers) {
        result.set(buffer, offset);
        offset += buffer.length;
    }
    return result;
}

describe("decodeStreamMetadata", () => {
    it("throws on empty input before stream_type and restores the offset", () => {
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(new Uint8Array([]), offset)).toThrow(
            /truncated stream metadata while reading stream_type/,
        );
        expect(offset.get()).toBe(0);
    });

    it("throws on truncated input before encodings_header and restores the offset", () => {
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(new Uint8Array([0x00]), offset)).toThrow(
            /truncated stream metadata while reading encodings_header/,
        );
        expect(offset.get()).toBe(0);
    });

    it("throws on invalid DATA logical stream type and restores the offset", () => {
        const encoded = new Uint8Array([0x1f, 0x00, 0x01, 0x01]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(/Invalid dictionary type: 15/);
        expect(offset.get()).toBe(0);
    });

    it("throws on invalid OFFSET logical stream type and restores the offset", () => {
        const encoded = new Uint8Array([0x2f, 0x00, 0x01, 0x01]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(/Invalid offset type: 15/);
        expect(offset.get()).toBe(0);
    });

    it("throws on invalid LENGTH logical stream type and restores the offset", () => {
        const encoded = new Uint8Array([0x3f, 0x00, 0x01, 0x01]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(/Invalid length type: 15/);
        expect(offset.get()).toBe(0);
    });

    it("throws on truncated numValues varint and restores the offset", () => {
        const encoded = new Uint8Array([0x00, 0x00, 0x80]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(/truncated stream metadata while reading numValues/);
        expect(offset.get()).toBe(0);
    });

    it("throws on truncated byteLength varint and restores the offset", () => {
        const encoded = new Uint8Array([0x00, 0x00, 0x01, 0x80]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(
            /truncated stream metadata while reading byteLength/,
        );
        expect(offset.get()).toBe(0);
    });

    it("throws on malformed byteLength varint and restores the offset", () => {
        const encoded = new Uint8Array([0x00, 0x00, 0x01, 0xff, 0xff, 0xff, 0xff, 0x10]);
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(
            /invalid stream metadata while reading byteLength at offset=3: invalid varint32 at offset=3/,
        );
        expect(offset.get()).toBe(0);
    });

    it("throws on truncated morton metadata and restores the offset", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 3);
        const encoded = concatenate(encodeStreamMetadata(metadata), new Uint8Array([0x80]));
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(
            /truncated stream metadata while reading morton metadata/,
        );
        expect(offset.get()).toBe(0);
    });

    it("throws on truncated rle metadata and restores the offset", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 3);
        const encoded = concatenate(encodeStreamMetadata(metadata), new Uint8Array([0x80]));
        const offset = new IntWrapper(0);

        expect(() => decodeStreamMetadata(encoded, offset)).toThrow(/truncated stream metadata while reading rle metadata/);
        expect(offset.get()).toBe(0);
    });

    it("decodes valid NONE metadata unchanged", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.NONE, LogicalLevelTechnique.NONE, 4);
        const encoded = encodeStreamMetadata(metadata);
        const offset = new IntWrapper(0);

        const decoded = decodeStreamMetadata(encoded, offset);

        expect(decoded.logicalLevelTechnique1).toBe(LogicalLevelTechnique.NONE);
        expect(decoded.logicalLevelTechnique2).toBe(LogicalLevelTechnique.NONE);
        expect(decoded.physicalLevelTechnique).toBe(PhysicalLevelTechnique.VARINT);
        expect(decoded.numValues).toBe(metadata.numValues);
        expect(decoded.byteLength).toBe(metadata.byteLength);
        expect(decoded.decompressedCount).toBe(metadata.decompressedCount);
        expect(offset.get()).toBe(encoded.length);
    });

    it("decodes valid MORTON metadata including extra varints", () => {
        const metadata = createStreamMetadata(LogicalLevelTechnique.MORTON, LogicalLevelTechnique.NONE, 5);
        const mortonInfo = encodeVarintInt32(new Uint32Array([12, 34]));
        const encoded = concatenate(encodeStreamMetadata(metadata), mortonInfo);
        const offset = new IntWrapper(0);

        const decoded = decodeStreamMetadata(encoded, offset);

        expect(decoded.logicalLevelTechnique1).toBe(LogicalLevelTechnique.MORTON);
        expect("numBits" in decoded && decoded.numBits).toBe(12);
        expect("coordinateShift" in decoded && decoded.coordinateShift).toBe(34);
        expect(offset.get()).toBe(encoded.length);
    });

    it("decodes valid RLE metadata including extra varints", () => {
        const metadata = createRleMetadata(LogicalLevelTechnique.RLE, LogicalLevelTechnique.NONE, 2, 6);
        const encoded = encodeStreamMetadata(metadata);
        const offset = new IntWrapper(0);

        const decoded = decodeStreamMetadata(encoded, offset);

        expect(decoded.logicalLevelTechnique1).toBe(LogicalLevelTechnique.RLE);
        expect("runs" in decoded && decoded.runs).toBe(2);
        expect("numRleValues" in decoded && decoded.numRleValues).toBe(6);
        expect(decoded.decompressedCount).toBe(6);
        expect(offset.get()).toBe(encoded.length);
    });
});
