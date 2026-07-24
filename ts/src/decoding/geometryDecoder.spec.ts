import { describe, it, expect } from "vitest";
import IntWrapper from "./intWrapper";
import { decodeGeometryColumn } from "./geometryDecoder";
import { createStream, concatenateBuffers } from "./decodingTestUtils";
import { encodeVarintInt32 } from "../encoding/integerEncodingUtils";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { OffsetType } from "../metadata/tile/offsetType";

// A single-value geometry-type stream decodes to VectorType.CONST, a multi-value one to FLAT.
const constGeometryType = createStream(PhysicalStreamType.DATA, encodeVarintInt32(new Uint32Array([3])), {
    technique: PhysicalLevelTechnique.VARINT,
    count: 1,
});
const flatGeometryType = createStream(PhysicalStreamType.DATA, encodeVarintInt32(new Uint32Array([1, 1])), {
    technique: PhysicalLevelTechnique.VARINT,
    count: 2,
});
const vertexStream = createStream(PhysicalStreamType.DATA, encodeVarintInt32(new Uint32Array([0, 0])), {
    logical: { dictionaryType: DictionaryType.VERTEX },
    technique: PhysicalLevelTechnique.VARINT,
    count: 2,
});
const indexStream = createStream(PhysicalStreamType.OFFSET, encodeVarintInt32(new Uint32Array([0, 1])), {
    logical: { offsetType: OffsetType.INDEX },
    technique: PhysicalLevelTechnique.VARINT,
    count: 2,
});

describe("decodeGeometryColumn invariants", () => {
    it("throws when a single-geometry-type column is missing its vertex buffer", () => {
        expect(() => decodeGeometryColumn(constGeometryType, 1, new IntWrapper(0), 1)).toThrow(
            "Geometry column is missing its vertex buffer.",
        );
    });

    it("throws when a mixed-geometry-type column is missing its vertex buffer", () => {
        expect(() => decodeGeometryColumn(flatGeometryType, 1, new IntWrapper(0), 2)).toThrow(
            "Geometry column is missing its vertex buffer.",
        );
    });

    it("throws when a single-geometry-type tessellated column is missing its triangle offsets", () => {
        const column = concatenateBuffers(constGeometryType, vertexStream, indexStream);
        expect(() => decodeGeometryColumn(column, 3, new IntWrapper(0), 1)).toThrow(
            "Tessellated geometry is missing its triangle offsets.",
        );
    });

    it("throws when a mixed-geometry-type tessellated column is missing its triangle offsets", () => {
        const column = concatenateBuffers(flatGeometryType, vertexStream, indexStream);
        expect(() => decodeGeometryColumn(column, 3, new IntWrapper(0), 2)).toThrow(
            "Tessellated geometry is missing its triangle offsets.",
        );
    });
});
