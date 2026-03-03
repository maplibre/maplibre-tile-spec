import { ConstGeometryVector } from "../vector/geometry/constGeometryVector";
import { GEOMETRY_TYPE } from "../vector/geometry/geometryType";
import { VertexBufferType } from "../vector/geometry/vertexBufferType";
import { encodeZOrderCurve } from "./zOrderCurveEncoder";
import type { GeometryVector, MortonSettings } from "../vector/geometry/geometryVector";

export const DEFAULT_MORTON_SETTINGS: MortonSettings = { numBits: 16, coordinateShift: 0 } as MortonSettings;

export function encode(x: number, y: number): number {
    return encodeZOrderCurve(x, y, DEFAULT_MORTON_SETTINGS.numBits, DEFAULT_MORTON_SETTINGS.coordinateShift);
}

export function encodePointGeometryVector(x: number, y: number): GeometryVector {
    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.POINT,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: new Uint32Array([0]),
            partOffsets: new Uint32Array([0]),
            ringOffsets: new Uint32Array([0]),
        },
        undefined,
        new Int32Array([x, y]),
    );
}

export function encodePointGeometryVectorWithOffset(x: number, y: number): GeometryVector {
    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.POINT,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: new Uint32Array([0]),
            partOffsets: new Uint32Array([0]),
            ringOffsets: new Uint32Array([0]),
        },
        new Int32Array([1]),
        new Int32Array([99, 99, x, y]),
    );
}

export function encodePointGeometryVectorWithMortonEncoding(x: number, y: number): GeometryVector {
    const mortonEncoded = encode(x, y);
    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.POINT,
        VertexBufferType.MORTON,
        {
            geometryOffsets: new Uint32Array([0]),
            partOffsets: new Uint32Array([0]),
            ringOffsets: new Uint32Array([0]),
        },
        new Int32Array([0]),
        new Int32Array([mortonEncoded]),
        DEFAULT_MORTON_SETTINGS,
    );
}

export function encodePointsGeometryVector(points: number[]): GeometryVector {
    return new ConstGeometryVector(
        points.length / 2,
        GEOMETRY_TYPE.POINT,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: new Uint32Array([0]),
            partOffsets: new Uint32Array([0]),
            ringOffsets: new Uint32Array([0]),
        },
        undefined,
        new Int32Array(points),
    );
}

export function encodeMultiPointGeometryVector(points: number[][]): GeometryVector {
    const numPoints = points.length;
    const vertexBuffer = new Int32Array(numPoints * 2);
    for (let i = 0; i < numPoints; i++) {
        vertexBuffer[i * 2] = points[i][0];
        vertexBuffer[i * 2 + 1] = points[i][1];
    }

    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.MULTIPOINT,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: new Uint32Array([0, numPoints]),
            partOffsets: undefined,
            ringOffsets: undefined,
        },
        undefined,
        vertexBuffer,
    );
}

export function encodeLineStringGeometryVector(lines: [number, number][]): GeometryVector {
    const numVertices = lines.length;
    const vertexBuffer = new Int32Array(numVertices * 2);
    for (let i = 0; i < numVertices; i++) {
        vertexBuffer[i * 2] = lines[i][0];
        vertexBuffer[i * 2 + 1] = lines[i][1];
    }

    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.LINESTRING,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: undefined,
            partOffsets: new Uint32Array([0, numVertices]),
            ringOffsets: undefined,
        },
        undefined,
        vertexBuffer,
    );
}

export function encodeLineStringGeometryVectorWithMortonEncoding(line: [number, number][]): GeometryVector {
    const numVertices = line.length;
    const vertexBuffer = new Int32Array(numVertices);
    const offsetBuffer = new Int32Array(numVertices);
    for (let i = 0; i < numVertices; i++) {
        vertexBuffer[i] = encode(line[i][0], line[i][1]);
        offsetBuffer[i] = i;
    }

    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.LINESTRING,
        VertexBufferType.MORTON,
        {
            geometryOffsets: undefined,
            partOffsets: new Uint32Array([0, numVertices]),
            ringOffsets: undefined,
        },
        offsetBuffer,
        vertexBuffer,
        DEFAULT_MORTON_SETTINGS,
    );
}

export function encodePolygonGeometryVector(polygons: [number, number][][]): GeometryVector {
    const numVertices = polygons.reduce((sum, polygon) => sum + polygon.length, 0);
    const vertexBuffer = new Int32Array(numVertices * 2);
    let vertexIndex = 0;
    for (const polygon of polygons) {
        for (const point of polygon) {
            vertexBuffer[vertexIndex * 2] = point[0];
            vertexBuffer[vertexIndex * 2 + 1] = point[1];
            vertexIndex++;
        }
    }

    return new ConstGeometryVector(
        1,
        GEOMETRY_TYPE.POLYGON,
        VertexBufferType.VEC_2,
        {
            geometryOffsets: undefined,
            partOffsets: new Uint32Array([0, polygons.length]),
            ringOffsets: new Uint32Array([0, numVertices]),
        },
        undefined,
        vertexBuffer,
    );
}
