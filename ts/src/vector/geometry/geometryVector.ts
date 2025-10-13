import TopologyVector from "../../vector/geometry/topologyVector";
import {convertGeometryVector} from "./geometryVectorConverter";
import {SelectionVector} from "../filter/selectionVector";
import ZOrderCurve from "./zOrderCurve";
import Point from "./point";
import {SINGLE_PART_GEOMETRY_TYPE} from "./geometryType";

export type Geometry = Array<Array<Point>>;

export interface MortonSettings {
    numBits: number;
    coordinateShift: number;
}

export enum VertexBufferType {
    MORTON,
    VEC_2,
    VEC_3
}

export abstract class GeometryVector implements Iterable<Geometry> {

    protected constructor(
        private readonly _vertexBufferType: VertexBufferType,
        private readonly _topologyVector: TopologyVector,
        private readonly _vertexOffsets: Int32Array,
        private readonly _vertexBuffer: Int32Array,
        private readonly _mortonSettings?: MortonSettings
    ) {
    }

    get vertexBufferType(): VertexBufferType{
        return this._vertexBufferType;
    }

    get topologyVector(): TopologyVector {
        return this._topologyVector;
    }

    get vertexOffsets(): Int32Array{
        return this._vertexOffsets
    }

    get vertexBuffer(): Int32Array{
        return this._vertexBuffer;
    }
    *[Symbol.iterator](): Iterator<Geometry> {
        const geometries = convertGeometryVector(this);
        let index = 0;

        while (index < this.numGeometries) {
            yield geometries[index++];
        }
    }

    /* Allows faster access to the vertices since morton encoding is currently not used in the POC. Morton encoding
       will be used after adapting the shader to decode the morton codes on the GPU. */
    getSimpleEncodedVertex(index: number): [number, number]{
        const offset = this.vertexOffsets? this.vertexOffsets[index] * 2 : index * 2;
        const x = this.vertexBuffer[offset];
        const y = this.vertexBuffer[offset+1];
        return [x, y];
    }

    //TODO: add scaling information to the constructor
    getVertex(index: number): [number, number]{
        if(this.vertexOffsets && this.mortonSettings){
            //TODO: move decoding of the morton codes on the GPU in the vertex shader
            const vertexOffset = this.vertexOffsets[index];
            const mortonEncodedVertex = this.vertexBuffer[vertexOffset];
                //TODO: improve performance -> inline calculation and move to decoding of VertexBuffer
            const vertex = ZOrderCurve.decode(mortonEncodedVertex, this.mortonSettings.numBits,
                    this.mortonSettings.coordinateShift);
            return [vertex.x, vertex.y];
        }

        const offset = this.vertexOffsets? this.vertexOffsets[index] * 2 : index * 2;
        const x = this.vertexBuffer[offset];
        const y = this.vertexBuffer[offset+1];
        return [x, y];
    }

    getGeometries(): Geometry[]{
        return convertGeometryVector(this);
    }

    get mortonSettings(): MortonSettings | undefined {
        return this._mortonSettings;
    }

    abstract containsPolygonGeometry(): boolean;

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector;

    abstract filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector);

    abstract containsSingleGeometryType(): boolean;

}
