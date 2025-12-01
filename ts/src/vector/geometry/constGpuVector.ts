import { GpuVector } from "./gpuVector";
import type TopologyVector from "./topologyVector";

export function createConstGpuVector(
    numGeometries: number,
    geometryType: number,
    triangleOffsets: Int32Array,
    indexBuffer: Int32Array,
    vertexBuffer: Int32Array,
    topologyVector?: TopologyVector | null,
): GpuVector {
    return new ConstGpuVector(
        numGeometries,
        geometryType,
        triangleOffsets,
        indexBuffer,
        vertexBuffer,
        topologyVector,
    );
}

//TODO: extend from GeometryVector -> make topology vector optional
export class ConstGpuVector extends GpuVector {
    constructor(
        private readonly _numGeometries: number,
        private readonly _geometryType: number,
        triangleOffsets: Int32Array,
        indexBuffer: Int32Array,
        vertexBuffer: Int32Array,
        topologyVector?: TopologyVector | null,
    ) {
        super(triangleOffsets, indexBuffer, vertexBuffer, topologyVector);
    }

    geometryType(index: number): number {
        return this._geometryType;
    }

    get numGeometries(): number {
        return this._numGeometries;
    }

    containsSingleGeometryType(): boolean {
        return true;
    }
}
