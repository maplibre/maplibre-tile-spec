import { convertGeometryVector } from "./geometryVectorConverter";
import { decodeZOrderCurve } from "./zOrderCurve";
import type Point from "@mapbox/point-geometry";
import type { GEOMETRY_TYPE } from "./geometryType";
import type { VertexBufferType } from "./vertexBufferType";
import type { TopologyVector } from "../../vector/geometry/topologyVector";

export type CoordinatesArray = Array<Array<Point>>;

export type Geometry = {
    coordinates: CoordinatesArray;
    type: GEOMETRY_TYPE;
};

export interface MortonSettings {
    numBits: number;
    coordinateShift: number;
}

export abstract class GeometryVector {
    protected constructor(
        private readonly _vertexBufferType: VertexBufferType,
        private readonly _topologyVector: TopologyVector,
        private readonly _vertexOffsets: Uint32Array | undefined,
        private readonly _vertexBuffer: Int32Array | Uint32Array,
        private readonly _mortonSettings?: MortonSettings,
    ) {}

    get vertexBufferType(): VertexBufferType {
        return this._vertexBufferType;
    }

    get topologyVector(): TopologyVector {
        return this._topologyVector;
    }

    get vertexOffsets(): Uint32Array | undefined {
        return this._vertexOffsets;
    }

    get vertexBuffer(): Int32Array | Uint32Array {
        return this._vertexBuffer;
    }

    /* Allows faster access to the vertices since morton encoding is currently not used in the POC. Morton encoding
       will be used after adapting the shader to decode the morton codes on the GPU. */
    getSimpleEncodedVertex(index: number): [number, number] {
        const offset = this.vertexOffsets ? this.vertexOffsets[index] * 2 : index * 2;
        const x = this.vertexBuffer[offset];
        const y = this.vertexBuffer[offset + 1];
        return [x, y];
    }

    //TODO: add scaling information to the constructor
    getVertex(index: number): [number, number] {
        if (this.vertexOffsets && this.mortonSettings) {
            //TODO: move decoding of the morton codes on the GPU in the vertex shader
            const vertexOffset = this.vertexOffsets[index];
            const mortonEncodedVertex = this.vertexBuffer[vertexOffset];
            //TODO: improve performance -> inline calculation and move to decoding of VertexBuffer
            const vertex = decodeZOrderCurve(
                mortonEncodedVertex,
                this.mortonSettings.numBits,
                this.mortonSettings.coordinateShift,
            );
            return [vertex.x, vertex.y];
        }

        const offset = this.vertexOffsets ? this.vertexOffsets[index] * 2 : index * 2;
        const x = this.vertexBuffer[offset];
        const y = this.vertexBuffer[offset + 1];
        return [x, y];
    }

    getGeometries(): CoordinatesArray[] {
        return convertGeometryVector(this);
    }

    get mortonSettings(): MortonSettings | undefined {
        return this._mortonSettings;
    }

    abstract containsPolygonGeometry(): boolean;

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract containsSingleGeometryType(): boolean;
}
