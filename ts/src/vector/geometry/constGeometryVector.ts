import { GeometryVector, type MortonSettings } from "./geometryVector";
import type TopologyVector from "../../vector/geometry/topologyVector";
import { GEOMETRY_TYPE, type SINGLE_PART_GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import { type SelectionVector } from "../filter/selectionVector";
import { SequenceSelectionVector } from "../filter/sequenceSelectionVector";

export class ConstGeometryVector extends GeometryVector {
    constructor(
        private readonly _numGeometries: number,
        private readonly _geometryType: number,
        vertexBufferType: VertexBufferType,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonSettings?: MortonSettings,
    ) {
        super(vertexBufferType, topologyVector, vertexOffsets, vertexBuffer, mortonSettings);
    }

    static createMortonEncoded(
        numGeometries: number,
        geometryType: number,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonInfo: MortonSettings,
    ): ConstGeometryVector {
        return new ConstGeometryVector(
            numGeometries,
            geometryType,
            VertexBufferType.MORTON,
            topologyVector,
            vertexOffsets,
            vertexBuffer,
            mortonInfo,
        );
    }

    static create(
        numGeometries: number,
        geometryType: number,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
    ): ConstGeometryVector {
        return new ConstGeometryVector(
            numGeometries,
            geometryType,
            VertexBufferType.VEC_2,
            topologyVector,
            vertexOffsets,
            vertexBuffer,
        );
    }

    geometryType(index: number): number {
        return this._geometryType;
    }

    get numGeometries(): number {
        return this._numGeometries;
    }

    containsPolygonGeometry(): boolean {
        return this._geometryType === GEOMETRY_TYPE.POLYGON || this._geometryType === GEOMETRY_TYPE.MULTIPOLYGON;
    }

    containsSingleGeometryType(): boolean {
        return true;
    }

    filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector {
        if (geometryType !== this._geometryType && geometryType + 3 !== this._geometryType) {
            return new FlatSelectionVector([]);
        }
        // All geometries match - return sequential selection (memory-efficient)
        return new SequenceSelectionVector(0, 1, this.numGeometries);
    }

    filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector) {
        if (geometryType !== this._geometryType && geometryType + 3 !== this._geometryType) {
            selectionVector.setLimit(0);
        }
    }

}
