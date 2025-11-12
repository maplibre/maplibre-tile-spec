import { type SelectionVector } from "../filter/selectionVector";
import { type SINGLE_PART_GEOMETRY_TYPE } from "./geometryType";
import { GpuVector } from "./gpuVector";
import type TopologyVector from "./topologyVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

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

    //TODO: refactor -> quick and dirty -> let a multi part geometry be equal to a single part geometry
    //to produce the same results as with MVT and the existing styles
    filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector {
        if (geometryType !== this._geometryType && geometryType + 3 !== this._geometryType) {
            return new FlatSelectionVector([]);
        }

        //TODO: use ConstSelectionVector
        const selectionVector = new Array(this.numGeometries);
        for (let i = 0; i < this.numGeometries; i++) {
            selectionVector[i] = i;
        }
        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector) {
        if (geometryType !== this._geometryType && geometryType + 3 !== this._geometryType) {
            selectionVector.setLimit(0);
        }
    }

}
