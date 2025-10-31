import { GpuVector } from "./gpuVector";
import type TopologyVector from "./topologyVector";

//TODO: extend from GeometryVector -> make topology vector optional
export class FlatGpuVector extends GpuVector {
    constructor(
        private readonly _geometryTypes: Int32Array,
        triangleOffsets: Int32Array,
        indexBuffer: Int32Array,
        vertexBuffer: Int32Array,
        topologyVector: TopologyVector | null,
    ) {
        super(triangleOffsets, indexBuffer, vertexBuffer, topologyVector);
    }

    static create(
        geometryTypes: Int32Array,
        triangleOffsets: Int32Array,
        indexBuffer: Int32Array,
        vertexBuffer: Int32Array,
        topologyVector?: TopologyVector | null,
    ): GpuVector {
        return new FlatGpuVector(geometryTypes, triangleOffsets, indexBuffer, vertexBuffer, topologyVector);
    }

    /*static createMortonEncoded(
        geometryTypes: Int32Array,
        triangleOffsets: Int32Array,
        indexBuffer: Int32Array,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonInfo: MortonSettings
    ): GpuVector {
        //TODO: refactor to use unsigned integers
        return new FlatGpuVector(
            VertexBufferType.MORTON,
            geometryTypes,
            triangleOffsets,
            indexBuffer,
            vertexOffsets,
            vertexBuffer,
            mortonInfo
        );
    }*/

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
