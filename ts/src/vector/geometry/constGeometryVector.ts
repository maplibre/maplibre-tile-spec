import { GeometryVector, type MortonSettings } from "./geometryVector";
import { GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import type { TopologyVector } from "../../vector/geometry/topologyVector";

export function createConstGeometryVector(
    numGeometries: number,
    geometryType: number,
    topologyVector: TopologyVector,
    vertexOffsets: Uint32Array | undefined,
    vertexBuffer: Int32Array | Uint32Array,
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

export function createMortonEncodedConstGeometryVector(
    numGeometries: number,
    geometryType: number,
    topologyVector: TopologyVector,
    vertexOffsets: Uint32Array | undefined,
    vertexBuffer: Int32Array | Uint32Array,
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

export class ConstGeometryVector extends GeometryVector {
    constructor(
        private readonly _numGeometries: number,
        private readonly _geometryType: number,
        vertexBufferType: VertexBufferType,
        topologyVector: TopologyVector,
        vertexOffsets: Uint32Array | undefined,
        vertexBuffer: Int32Array | Uint32Array,
        mortonSettings?: MortonSettings,
    ) {
        super(vertexBufferType, topologyVector, vertexOffsets, vertexBuffer, mortonSettings);
    }

    geometryType(_index: number): number {
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
}
