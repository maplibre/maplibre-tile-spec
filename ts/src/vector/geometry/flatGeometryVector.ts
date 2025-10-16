import {GeometryVector, type MortonSettings, VertexBufferType} from "./geometryVector";
import type TopologyVector from "../../vector/geometry/topologyVector";
import {type SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";
import {GEOMETRY_TYPE, type SINGLE_PART_GEOMETRY_TYPE} from "./geometryType";


export class FlatGeometryVector extends GeometryVector{
    constructor(
        vertexBufferType: VertexBufferType,
        //TODO: refactor -> use UInt8Array
        private readonly _geometryTypes: Int32Array,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonSettings?: MortonSettings
    ) {
        super(vertexBufferType, topologyVector, vertexOffsets, vertexBuffer, mortonSettings);
    }

    static createMortonEncoded(
        geometryTypes: Int32Array,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array,
        mortonInfo: MortonSettings
    ): FlatGeometryVector {
        //TODO: refactor to use unsigned integers
        return new FlatGeometryVector(
            VertexBufferType.MORTON,
            geometryTypes,
            topologyVector,
            vertexOffsets,
            vertexBuffer,
            mortonInfo
        );
    }

    public static create(
        geometryTypes: Int32Array,
        topologyVector: TopologyVector,
        vertexOffsets: Int32Array,
        vertexBuffer: Int32Array
    ): FlatGeometryVector {
        return new FlatGeometryVector(
            VertexBufferType.VEC_2,
            geometryTypes,
            topologyVector,
            vertexOffsets,
            vertexBuffer,
        );
    }

    geometryType(index: number): number {
        return this._geometryTypes[index];
    }

    get numGeometries(): number {
        return this._geometryTypes.length;
    }

    containsPolygonGeometry(): boolean {
        for (let i = 0; i < this.numGeometries; i++) {
            if (
                this.geometryType(i) === GEOMETRY_TYPE.POLYGON ||
                this.geometryType(i) === GEOMETRY_TYPE.MULTIPOLYGON
            ) {
                return true;
            }
        }
        return false;
    }

    //TODO: refactor -> quick and dirty -> let a multi part geometry be equal to a single part geometry
    //to produce the same results as with MVT and the existing styles
    filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector{
        const selectionVector = [];
        for(let i = 0; i < this.numGeometries; i++){
            if(this.geometryType(i) === geometryType || this.geometryType(i) === (geometryType + 3)){
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(predicateGeometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector){
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for(let i = 0; i < selectionVector.limit; i++){
            const index = vector[i];
            const geometryType = this.geometryType(index);
            if(predicateGeometryType === geometryType || (predicateGeometryType + 3) ===  geometryType){
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    containsSingleGeometryType(): boolean{
        return false;
    }

}
