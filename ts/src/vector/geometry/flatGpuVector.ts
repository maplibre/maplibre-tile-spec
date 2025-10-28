import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import { GpuVector } from "./gpuVector";
import { type SINGLE_PART_GEOMETRY_TYPE } from "./geometryType";
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

    //TODO: refactor -> quick and dirty -> let a multi part geometry be equal to a single part geometry
    //to produce the same results as with MVT and the existing styles
    filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.numGeometries; i++) {
            if (this.geometryType(i) === geometryType || this.geometryType(i) === geometryType + 3) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector) {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = selectionVector[i];
            if (this.geometryType(index) === geometryType || this.geometryType(index) === geometryType + 3) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    containsSingleGeometryType(): boolean {
        return false;
    }
}
