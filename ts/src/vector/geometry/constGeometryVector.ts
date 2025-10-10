import { GeometryVector, MortonSettings, VertexBufferType } from "./geometryVector";
import TopologyVector from "../../vector/geometry/topologyVector";
import { SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import { GEOMETRY_TYPE, SINGLE_PART_GEOMETRY_TYPE } from "./geometryType";

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

    containsSingleGeometryType(): boolean {
        return true;
    }
}
