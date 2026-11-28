import { GpuVector } from "./gpuVector";
import type { TopologyVector } from "./topologyVector";
import { VertexBufferType } from "./vertexBufferType";

export function createFlatGpuVector(
    geometryTypes: Uint32Array,
    triangleOffsets: Uint32Array,
    indexBuffer: Uint32Array,
    vertexBuffer: Int32Array | Uint32Array,
    topologyVector?: TopologyVector,
    vertexBufferType: VertexBufferType = VertexBufferType.VEC_2,
): GpuVector {
    return new FlatGpuVector(
        geometryTypes,
        triangleOffsets,
        indexBuffer,
        vertexBuffer,
        topologyVector,
        vertexBufferType,
    );
}

//TODO: extend from GeometryVector -> make topology vector optional
export class FlatGpuVector extends GpuVector {
    constructor(
        private readonly _geometryTypes: Uint32Array,
        triangleOffsets: Uint32Array,
        indexBuffer: Uint32Array,
        vertexBuffer: Int32Array | Uint32Array,
        topologyVector?: TopologyVector,
        vertexBufferType: VertexBufferType = VertexBufferType.VEC_2,
    ) {
        super(triangleOffsets, indexBuffer, vertexBuffer, topologyVector, vertexBufferType);
    }

    geometryType(index: number): number {
        return this._geometryTypes[index];
    }

    get numGeometries(): number {
        return this._geometryTypes.length;
    }

    containsSingleGeometryType(): boolean {
        return false;
    }
}
