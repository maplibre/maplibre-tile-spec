import { createFlatGeometryVector } from "./flatGeometryVector";
import type { CoordinatesArray } from "./geometryVector";
import type { TopologyVector } from "./topologyVector";

export abstract class GpuVector implements Iterable<CoordinatesArray> {
    protected constructor(
        private readonly _triangleOffsets: Uint32Array,
        private readonly _indexBuffer: Uint32Array,
        private readonly _vertexBuffer: Int32Array | Uint32Array,
        private readonly _topologyVector?: TopologyVector,
    ) {}

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract containsSingleGeometryType(): boolean;

    get triangleOffsets(): Uint32Array {
        return this._triangleOffsets;
    }

    get indexBuffer(): Uint32Array {
        return this._indexBuffer;
    }

    get vertexBuffer(): Int32Array | Uint32Array {
        return this._vertexBuffer;
    }

    get topologyVector(): TopologyVector | undefined {
        return this._topologyVector;
    }

    getGeometries(): CoordinatesArray[] {
        if (!this._topologyVector) {
            throw new Error("Cannot convert GpuVector to coordinates without topology information");
        }
        const types = new Uint32Array(this.numGeometries);
        for (let i = 0; i < this.numGeometries; i++) {
            types[i] = this.geometryType(i);
        }
        return createFlatGeometryVector(types, this._topologyVector, undefined, this._vertexBuffer).getGeometries();
    }

    [Symbol.iterator](): Iterator<CoordinatesArray> {
        /*for(let i = 1; i < this.triangleOffsets.length; i++) {
           const numTriangles = this.triangleOffsets[i] - this.triangleOffsets[i-1];
           const startIndex = this.triangleOffsets[i-1] * 3;
           const endIndex = this.triangleOffsets[i] * 3;
       }

        while (index < this.numGeometries) {
            yield geometries[index++];
        }*/

        //throw new Error("Iterator on a GpuVector is not implemented yet.");
        return null;
    }
}
