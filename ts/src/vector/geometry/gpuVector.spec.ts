import { describe, it, expect } from "vitest";
import { ConstGpuVector } from "./constGpuVector";
import { GEOMETRY_TYPE } from "./geometryType";
import type { TopologyVector } from "./topologyVector";

function gpuVector(geometryType: number, topologyVector?: TopologyVector): ConstGpuVector {
    return new ConstGpuVector(
        1,
        geometryType,
        new Uint32Array([0]), // triangleOffsets
        new Uint32Array([0]), // indexBuffer
        new Int32Array([0, 0]), // vertexBuffer
        topologyVector,
    );
}

describe("GpuVector.getGeometries invariants", () => {
    it("throws without topology information", () => {
        expect(() => gpuVector(GEOMETRY_TYPE.POLYGON).getGeometries()).toThrow(
            "Cannot convert GpuVector to coordinates without topology information",
        );
    });

    it("throws when part or ring offsets are missing", () => {
        expect(() => gpuVector(GEOMETRY_TYPE.POLYGON, {}).getGeometries()).toThrow(
            "Cannot convert GpuVector to coordinates without part and ring offsets",
        );
    });

    it("throws for a MultiPolygon without geometry offsets", () => {
        const topology: TopologyVector = {
            partOffsets: new Uint32Array([0, 1]),
            ringOffsets: new Uint32Array([0, 1]),
        };
        expect(() => gpuVector(GEOMETRY_TYPE.MULTIPOLYGON, topology).getGeometries()).toThrow(
            "Cannot convert MultiPolygon GpuVector to coordinates without geometry offsets",
        );
    });
});

describe("GpuVector accessors", () => {
    it("exposes the buffers and topology passed to the constructor", () => {
        const triangleOffsets = new Uint32Array([0, 1]);
        const indexBuffer = new Uint32Array([2, 3]);
        const vertexBuffer = new Int32Array([4, 5]);
        const topology: TopologyVector = { partOffsets: new Uint32Array([0, 1]) };
        const vector = new ConstGpuVector(
            1,
            GEOMETRY_TYPE.POLYGON,
            triangleOffsets,
            indexBuffer,
            vertexBuffer,
            topology,
        );

        expect(vector.triangleOffsets).toBe(triangleOffsets);
        expect(vector.indexBuffer).toBe(indexBuffer);
        expect(vector.vertexBuffer).toBe(vertexBuffer);
        expect(vector.topologyVector).toBe(topology);
    });
});
