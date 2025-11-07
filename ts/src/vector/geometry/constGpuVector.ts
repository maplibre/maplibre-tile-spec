import { GpuVector } from "./gpuVector";
import type TopologyVector from "./topologyVector";

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

    static create(
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

    /*static createMortonEncoded(
        numGeometries: number,
        geometryType: number,
        triangleOffsets: Int32Array,
        indexBuffer: Int32Array,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonInfo: MortonSettings
    ): GpuVector {
        //TODO: refactor to use unsigned integers
        return new ConstGpuVector(
            numGeometries,
            geometryType,
            VertexBufferType.MORTON,
            triangleOffsets,
            indexBuffer,
            vertexOffsets,
            vertexBuffer,
            mortonInfo
       );
    }*/

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
