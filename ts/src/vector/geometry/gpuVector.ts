import { SelectionVector } from "../filter/selectionVector";
import { SINGLE_PART_GEOMETRY_TYPE } from "./geometryType";
import { Geometry } from "./geometryVector";
import TopologyVector from "./topologyVector";

export abstract class GpuVector implements Iterable<Geometry> {
    protected constructor(
        private readonly _triangleOffsets: Int32Array,
        private readonly _indexBuffer: Int32Array,
        private readonly _vertexBuffer: Int32Array,
        private readonly _topologyVector?: TopologyVector | null,
    ) {}

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector;

    abstract filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector);

    abstract containsSingleGeometryType(): boolean;

    get triangleOffsets(): Int32Array {
        return this._triangleOffsets;
    }

    get indexBuffer(): Int32Array {
        return this._indexBuffer;
    }

    get vertexBuffer(): Int32Array {
        return this._vertexBuffer;
    }

    get topologyVector(): TopologyVector | null {
        return this._topologyVector;
    }

    [Symbol.iterator](): Iterator<Geometry> {
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
