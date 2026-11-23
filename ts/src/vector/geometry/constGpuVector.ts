import { GpuVector } from "./gpuVector";
import type { TopologyVector } from "./topologyVector";
import { VertexBufferType } from "./vertexBufferType";

export function createConstGpuVector(
    numGeometries: number,
    geometryType: number,
    triangleOffsets: Uint32Array,
    indexBuffer: Uint32Array,
    vertexBuffer: Int32Array | Uint32Array,
    topologyVector?: TopologyVector,
    vertexBufferType: VertexBufferType = VertexBufferType.VEC_2,
): GpuVector {
    return new ConstGpuVector(
        numGeometries,
        geometryType,
        triangleOffsets,
        indexBuffer,
        vertexBuffer,
        topologyVector,
        vertexBufferType,
    );
}

//TODO: extend from GeometryVector -> make topology vector optional
export class ConstGpuVector extends GpuVector {
    constructor(
        private readonly _numGeometries: number,
        private readonly _geometryType: number,
        triangleOffsets: Uint32Array,
        indexBuffer: Uint32Array,
        vertexBuffer: Int32Array | Uint32Array,
        topologyVector?: TopologyVector,
        vertexBufferType: VertexBufferType = VertexBufferType.VEC_2,
    ) {
        super(triangleOffsets, indexBuffer, vertexBuffer, topologyVector, vertexBufferType);
    }

    geometryType(_index: number): number {
        return this._geometryType;
    }

    get numGeometries(): number {
        return this._numGeometries;
    }

    containsSingleGeometryType(): boolean {
        return true;
    }
}
