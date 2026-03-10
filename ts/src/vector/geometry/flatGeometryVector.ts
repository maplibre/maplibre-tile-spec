import { GeometryVector, type MortonSettings } from "./geometryVector";
import { GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import type { TopologyVector } from "../../vector/geometry/topologyVector";

export function createFlatGeometryVector(
    geometryTypes: Uint32Array,
    topologyVector: TopologyVector,
    vertexOffsets: Uint32Array | undefined,
    vertexBuffer: Int32Array | Uint32Array,
): FlatGeometryVector {
    return new FlatGeometryVector(VertexBufferType.VEC_2, geometryTypes, topologyVector, vertexOffsets, vertexBuffer);
}

export function createFlatGeometryVectorMortonEncoded(
    geometryTypes: Uint32Array,
    topologyVector: TopologyVector,
    vertexOffsets: Uint32Array | undefined,
    vertexBuffer: Int32Array | Uint32Array,
    mortonInfo: MortonSettings,
): FlatGeometryVector {
    return new FlatGeometryVector(
        VertexBufferType.MORTON,
        geometryTypes,
        topologyVector,
        vertexOffsets,
        vertexBuffer,
        mortonInfo,
    );
}

export class FlatGeometryVector extends GeometryVector {
    constructor(
        vertexBufferType: VertexBufferType,
        private readonly _geometryTypes: Uint32Array,
        topologyVector: TopologyVector,
        vertexOffsets: Uint32Array | undefined,
        vertexBuffer: Int32Array | Uint32Array,
        mortonSettings?: MortonSettings,
    ) {
        super(vertexBufferType, topologyVector, vertexOffsets, vertexBuffer, mortonSettings);
    }

    geometryType(index: number): number {
        return this._geometryTypes[index];
    }

    get numGeometries(): number {
        return this._geometryTypes.length;
    }

    containsPolygonGeometry(): boolean {
        for (let i = 0; i < this.numGeometries; i++) {
            if (this.geometryType(i) === GEOMETRY_TYPE.POLYGON || this.geometryType(i) === GEOMETRY_TYPE.MULTIPOLYGON) {
                return true;
            }
        }
        return false;
    }

    containsSingleGeometryType(): boolean {
        return false;
    }
}
