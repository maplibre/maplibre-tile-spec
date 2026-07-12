import { describe, expect, it } from "vitest";
import {
    encodeLineStringGeometryVector,
    encodeLineStringGeometryVectorWithMortonEncoding,
    encodeMultiLineStringGeometryVector,
    encodeMultiLineStringGeometryVectorWithMortonOffsets,
    encodeMultiLineStringGeometryVectorWithOffsets,
    encodeMultiPointGeometryVector,
    encodeMultiPolygonGeometryVector,
    encodeMultiPolygonGeometryVectorWithMortonOffsets,
    encodeMultiPolygonGeometryVectorWithOffsets,
    encodePointGeometryVector,
    encodePointGeometryVectorWithMortonEncoding,
    encodePointGeometryVectorWithOffset,
    encodePointsGeometryVector,
    encodePolygonGeometryVector,
    encodePolygonGeometryVectorWithMortonOffsets,
    encodePolygonGeometryVectorWithOffsets,
} from "../../encoding/constGeometryVectorEncoder";
import { GEOMETRY_TYPE } from "./geometryType";
import { ConstGpuVector } from "./constGpuVector";
import { ConstGeometryVector } from "./constGeometryVector";
import type { GeometryVector } from "./geometryVector";
import { convertGeometryVector } from "./geometryVectorConverter";
import { VertexBufferType } from "./vertexBufferType";

const line: [number, number][] = [
    [1, 2],
    [3, 4],
    [5, 6],
];
const polygon: [number, number][][] = [
    [
        [0, 0],
        [10, 0],
        [10, 10],
    ],
    [
        [2, 2],
        [3, 2],
        [3, 3],
    ],
];
const multiLine: [number, number][][] = [
    line,
    [
        [7, 8],
        [9, 10],
    ],
];
const multiPolygon: [number, number][][][] = [
    polygon,
    [
        [
            [20, 20],
            [30, 20],
            [30, 30],
        ],
    ],
];

describe("GeometryVector.getGeometry", () => {
    const vectors: Array<[string, () => GeometryVector]> = [
        ["point", () => encodePointGeometryVector(1, 2)],
        ["dictionary point", () => encodePointGeometryVectorWithOffset(1, 2)],
        ["Morton point", () => encodePointGeometryVectorWithMortonEncoding(1, 2)],
        ["multipoint", () => encodeMultiPointGeometryVector(line)],
        ["line", () => encodeLineStringGeometryVector(line)],
        ["Morton line", () => encodeLineStringGeometryVectorWithMortonEncoding(line)],
        ["polygon", () => encodePolygonGeometryVector(polygon)],
        ["dictionary polygon", () => encodePolygonGeometryVectorWithOffsets(polygon)],
        ["Morton polygon", () => encodePolygonGeometryVectorWithMortonOffsets(polygon)],
        ["multiline", () => encodeMultiLineStringGeometryVector(multiLine)],
        ["dictionary multiline", () => encodeMultiLineStringGeometryVectorWithOffsets(multiLine)],
        ["Morton multiline", () => encodeMultiLineStringGeometryVectorWithMortonOffsets(multiLine)],
        ["multipolygon", () => encodeMultiPolygonGeometryVector(multiPolygon)],
        ["dictionary multipolygon", () => encodeMultiPolygonGeometryVectorWithOffsets(multiPolygon)],
        ["Morton multipolygon", () => encodeMultiPolygonGeometryVectorWithMortonOffsets(multiPolygon)],
    ];

    it.each(vectors)("matches whole-vector conversion for %s", (_name, createVector) => {
        const vector = createVector();
        const expected = convertGeometryVector(vector);
        for (let i = vector.numGeometries - 1; i >= 0; i--) {
            expect(vector.getGeometry(i)).toEqual(expected[i]);
        }
    });

    it("supports random access to vectors containing multiple geometries", () => {
        const vector = encodePointsGeometryVector([1, 2, 3, 4, 5, 6]);
        const expected = convertGeometryVector(vector);
        expect(vector.getGeometry(2)).toEqual(expected[2]);
        expect(vector.getGeometry(0)).toEqual(expected[0]);
        expect(vector.getGeometry(1)).toEqual(expected[1]);
    });

    it("returns independently mutable coordinates", () => {
        const vector = encodePolygonGeometryVector(polygon);
        const first = vector.getGeometry(0);
        first[0][0].x = 999;
        first[0].push(first[0][0]);
        expect(vector.getGeometry(0)).toEqual(convertGeometryVector(vector)[0]);
    });

    it("rejects invalid indexes", () => {
        const vector = encodePointGeometryVector(1, 2);
        expect(() => vector.getGeometry(-1)).toThrow(RangeError);
        expect(() => vector.getGeometry(1)).toThrow(RangeError);
        expect(() => vector.getGeometry(0.5)).toThrow(RangeError);
    });

    it("treats an empty vertex-offset vector as sequential encoding", () => {
        const vector = new ConstGeometryVector(
            1,
            GEOMETRY_TYPE.LINESTRING,
            VertexBufferType.VEC_2,
            { partOffsets: new Uint32Array([0, 2]) },
            new Uint32Array(),
            new Int32Array([1, 2, 3, 4]),
        );
        expect(vector.getGeometry(0)).toEqual(convertGeometryVector(vector)[0]);
    });
});

describe("GpuVector.getGeometry", () => {
    it("supports polygon random access", () => {
        const vector = new ConstGpuVector(
            2,
            GEOMETRY_TYPE.POLYGON,
            new Uint32Array(),
            new Uint32Array(),
            new Int32Array([0, 0, 10, 0, 10, 10, 20, 20, 30, 20, 30, 30]),
            {
                partOffsets: new Uint32Array([0, 1, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
        );

        expect(vector.getGeometry(1)).toEqual(vector.getGeometries()[1]);
        expect(vector.getGeometry(0)).toEqual(vector.getGeometries()[0]);
    });

    it("requires topology information", () => {
        const vector = new ConstGpuVector(
            1,
            GEOMETRY_TYPE.POLYGON,
            new Uint32Array(),
            new Uint32Array(),
            new Int32Array(),
        );
        expect(() => vector.getGeometry(0)).toThrow("without topology information");
    });
});
