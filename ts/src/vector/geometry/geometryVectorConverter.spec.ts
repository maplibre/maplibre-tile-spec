import Point from "@mapbox/point-geometry";
import { describe, it, expect } from "vitest";
import { convertGeometryVector } from "./geometryVectorConverter";
import { GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import { encodeZOrderCurve } from "../../encoding/zOrderCurveEncoder";
import type { GeometryVector, MortonSettings } from "./geometryVector";
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
import { ConstGeometryVector } from "./constGeometryVector";
import { FlatGeometryVector } from "./flatGeometryVector";

describe("POINT – sequential vertex buffer (no vertexOffsets)", () => {
    it("creates a single point from the vertex buffer", () => {
        const x = 5;
        const y = 7;
        const gv = encodePointGeometryVector(x, y);
        const result = convertGeometryVector(gv);

        expect(result).toHaveLength(1);
        expect(result[0]).toEqual([[new Point(x, y)]]);
    });

    it("creates multiple points sequentially", () => {
        const gv = encodePointsGeometryVector([1, 2, 3, 4]);
        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(1, 2)]]);
        expect(result[1]).toEqual([[new Point(3, 4)]]);
    });
});

describe("POINT – VEC_2 dictionary encoded", () => {
    it("reads point via vertexOffsets with VEC_2 buffer type", () => {
        // vertexOffsets[0] = 1 → vertexBuffer[2], vertexBuffer[3]
        const x = 42;
        const y = 55;
        const gv = encodePointGeometryVectorWithOffset(x, y);
        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(x, y)]]);
    });
});

describe("POINT – Morton dictionary encoded", () => {
    it("decodes a point via morton encoding", () => {
        const x = 3;
        const y = 7;

        const gv = encodePointGeometryVectorWithMortonEncoding(x, y);
        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(x, y)]]);
    });

    it("decodes a point with a non-zero coordinateShift", () => {
        const x = 50;
        const y = 80;
        const settings: MortonSettings = { numBits: 16, coordinateShift: 100 } as MortonSettings;
        const code = encodeZOrderCurve(x, y, settings.numBits, settings.coordinateShift);

        const gv = new ConstGeometryVector(
            1,
            GEOMETRY_TYPE.POINT,
            VertexBufferType.MORTON,
            {
                geometryOffsets: undefined,
                partOffsets: undefined,
                ringOffsets: undefined,
            },
            new Uint32Array([0]),
            new Int32Array([code]),
            settings,
        );

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(x, y)]]);
    });
});

describe("MULTIPOINT – sequential vertex buffer", () => {
    it("creates a multi-point geometry", () => {
        const gv = encodeMultiPointGeometryVector([
            [1, 2],
            [3, 4],
            [5, 6],
        ]);
        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(1, 2)], [new Point(3, 4)], [new Point(5, 6)]]);
    });
});

describe("MULTIPOINT – VEC_2 dictionary encoded", () => {
    it("reads multi-point via vertexOffsets", () => {
        // offsets [0, 2] → vertexBuffer[0,1] and vertexBuffer[4,5]
        const gv = new ConstGeometryVector(
            1,
            GEOMETRY_TYPE.MULTIPOINT,
            VertexBufferType.VEC_2,
            {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: undefined,
                ringOffsets: undefined,
            },
            new Uint32Array([0, 2]),
            new Int32Array([10, 20, 99, 99, 30, 40]),
        );

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(10, 20)], [new Point(30, 40)]]);
    });
});

describe("LINESTRING – sequential vertex buffer, no polygon context", () => {
    it("creates a line string from sequential vertices", () => {
        const gv = encodeLineStringGeometryVector([
            [0, 0],
            [1, 1],
            [2, 2],
        ]);
        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(0, 0), new Point(1, 1), new Point(2, 2)]]);
    });
});

describe("LINESTRING – sequential vertex buffer, polygon context (uses ringOffsets)", () => {
    it("creates a line string using ringOffsets when containsPolygon is true", () => {
        const gv = {
            numGeometries: 1,
            vertexBuffer: new Int32Array([1, 2, 3, 4]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 2]),
            },
            geometryType: () => GEOMETRY_TYPE.LINESTRING,
            containsPolygonGeometry: () => true,
        } as any as GeometryVector;

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(1, 2), new Point(3, 4)]]);
    });
});

describe("LINESTRING – VEC_2 dictionary encoded", () => {
    it("decodes a line string via vertexOffsets VEC_2", () => {
        // offsets [0, 1] → vertexBuffer[0,1] and [2,3]
        const gv = {
            numGeometries: 1,
            vertexBuffer: new Int32Array([5, 10, 15, 20]),
            vertexOffsets: new Uint32Array([0, 1]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.LINESTRING,
            containsPolygonGeometry: () => false,
        } as any as GeometryVector;

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(5, 10), new Point(15, 20)]]);
    });
});

describe("LINESTRING – Morton dictionary encoded", () => {
    it("decodes a line string via Morton encoding", () => {
        const gv = encodeLineStringGeometryVectorWithMortonEncoding([
            [1, 2],
            [3, 4],
            [5, 6],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(1, 2), new Point(3, 4), new Point(5, 6)]]);
    });
});

describe("POLYGON – sequential vertex buffer, no holes", () => {
    it("creates a polygon shell that is closed", () => {
        const gv = encodePolygonGeometryVector([
            [
                [0, 0],
                [10, 0],
                [10, 10],
                [0, 10],
            ],
        ]);

        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(5); // 4 + closed
        expect(shell[0]).toEqual(shell[4]);
    });
});

describe("POLYGON – sequential vertex buffer, with hole", () => {
    it("creates a polygon with one hole", () => {
        const gv = encodePolygonGeometryVector([
            [
                [0, 0],
                [10, 0],
                [0, 10],
            ],
            [
                [2, 2],
                [4, 2],
                [2, 4],
            ],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // shell has 3 points + 1 (closing point)
        expect(result[0][1]).toHaveLength(4); // hole has 3 points + 1 (closing point)
    });
});

describe("POLYGON – VEC_2 dictionary encoded, no holes", () => {
    it("creates a closed polygon shell via VEC_2 vertex offsets", () => {
        const gv = encodePolygonGeometryVectorWithOffsets([
            [
                [0, 0],
                [5, 5],
                [10, 0],
            ],
        ]);

        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(4); // closed
        expect(shell[0]).toEqual(shell[3]);
    });
});

describe("POLYGON – VEC_2 dictionary encoded, with hole", () => {
    it("creates a polygon with one hole via VEC_2 vertex offsets", () => {
        const gv = encodePolygonGeometryVectorWithOffsets([
            [
                [0, 0],
                [10, 0],
                [0, 10],
            ],
            [
                [2, 2],
                [4, 2],
                [2, 4],
            ],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // shell has 3 points + 1 (closing point)
        expect(result[0][1]).toHaveLength(4); // hole has 3 points + 1 (closing point)
    });
});

describe("POLYGON – Morton dictionary encoded, no holes", () => {
    it("creates a closed polygon shell via Morton encoding", () => {
        const gv = encodePolygonGeometryVectorWithMortonOffsets([
            [
                [0, 0],
                [5, 0],
                [0, 5],
            ],
        ]);
        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(4);
        expect(shell[0]).toEqual(new Point(0, 0));
        expect(shell[1]).toEqual(new Point(5, 0));
        expect(shell[2]).toEqual(new Point(0, 5));
        expect(shell[3]).toEqual(shell[0]); // closed
    });
});

describe("POLYGON – Morton dictionary encoded, with hole", () => {
    it("creates a polygon with one hole via Morton encoding", () => {
        const gv = encodePolygonGeometryVectorWithMortonOffsets([
            [
                [0, 0],
                [10, 0],
                [0, 10],
            ],
            [
                [1, 1],
                [3, 1],
                [1, 3],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // shell has 3 points + 1 (closing point)
        expect(result[0][1]).toHaveLength(4); // hole has 3 points + 1 (closing point)
    });
});

describe("MULTILINESTRING – sequential vertex buffer, no polygon context", () => {
    it("creates a multi-line string with two line strings", () => {
        const gv = encodeMultiLineStringGeometryVector([
            [
                [0, 0],
                [1, 1],
            ],
            [
                [2, 2],
                [3, 3],
                [4, 4],
            ],
            [
                [5, 5],
                [6, 6],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(3);
        expect(result[0][0]).toEqual([new Point(0, 0), new Point(1, 1)]);
        expect(result[0][1]).toEqual([new Point(2, 2), new Point(3, 3), new Point(4, 4)]);
        expect(result[0][2]).toEqual([new Point(5, 5), new Point(6, 6)]);
    });
});

describe("MULTILINESTRING – VEC_2 dictionary encoded", () => {
    it("decodes multi-line string via vertexOffsets VEC_2", () => {
        const gv = encodeMultiLineStringGeometryVectorWithOffsets([
            [
                [0, 1],
                [2, 3],
            ],
            [
                [4, 5],
                [6, 7],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toEqual([new Point(0, 1), new Point(2, 3)]);
        expect(result[0][1]).toEqual([new Point(4, 5), new Point(6, 7)]);
    });
});

describe("MULTILINESTRING – Morton dictionary encoded", () => {
    it("decodes multi-line string via Morton encoding", () => {
        const gv = encodeMultiLineStringGeometryVectorWithMortonOffsets([
            [
                [1, 2],
                [3, 4],
            ],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0][0]).toEqual([new Point(1, 2), new Point(3, 4)]);
    });
});

describe("MULTIPOLYGON – sequential vertex buffer, no holes", () => {
    it("creates a multi-polygon with two triangular polygons", () => {
        const gv = encodeMultiPolygonGeometryVector([
            [
                [
                    [0, 0],
                    [1, 0],
                    [0, 1],
                ],
            ],
            [
                [
                    [5, 5],
                    [6, 5],
                    [5, 6],
                ],
            ],
            [
                [
                    [10, 10],
                    [10, 0],
                    [0, 10],
                ],
            ],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(3);
        expect(result[0][0]).toHaveLength(4); // closed shell
        expect(result[0][1]).toHaveLength(4); // closed shell
        expect(result[0][2]).toHaveLength(4); // closed shell
    });
});

describe("MULTIPOLYGON – sequential vertex buffer, with holes", () => {
    it("creates a polygon that has a hole", () => {
        const gv = encodeMultiPolygonGeometryVector([
            [
                [
                    [0, 0],
                    [10, 0],
                    [0, 10],
                ],
                [
                    [1, 1],
                    [3, 1],
                    [1, 3],
                ],
            ],
            [
                [
                    [5, 5],
                    [25, 5],
                    [5, 25],
                ],
                [
                    [6, 6],
                    [7, 6],
                    [6, 7],
                ],
            ],
            [
                [
                    [10, 10],
                    [10, 0],
                    [0, 10],
                ],
            ],
        ]);

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(5);
        expect(result[0][0]).toHaveLength(4);
        expect(result[0][1]).toHaveLength(4);
        expect(result[0][2]).toHaveLength(4);
        expect(result[0][3]).toHaveLength(4);
        expect(result[0][4]).toHaveLength(4);
    });
});

describe("MULTIPOLYGON – VEC_2 dictionary encoded, no holes", () => {
    it("decodes multi-polygon via vertexOffsets VEC_2", () => {
        const gv = encodeMultiPolygonGeometryVectorWithOffsets([
            [
                [
                    [0, 0],
                    [2, 0],
                    [0, 2],
                ],
            ],
            [
                [
                    [5, 5],
                    [7, 5],
                    [5, 7],
                ],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // closed
    });
});

describe("MULTIPOLYGON – VEC_2 dictionary encoded, with holes", () => {
    it("creates a multi-polygon with hole via VEC_2", () => {
        const gv = encodeMultiPolygonGeometryVectorWithOffsets([
            [
                [
                    [0, 0],
                    [10, 0],
                    [0, 10],
                ],
                [
                    [1, 1],
                    [3, 1],
                    [1, 3],
                ],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
    });
});

describe("MULTIPOLYGON – Morton dictionary encoded, no holes", () => {
    it("decodes multi-polygon shells via Morton encoding", () => {
        const gv = encodeMultiPolygonGeometryVectorWithMortonOffsets([
            [
                [
                    [0, 0],
                    [1, 0],
                    [0, 1],
                ],
            ],
        ]);
        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(4); // closed
        expect(shell[0]).toEqual(new Point(0, 0));
        expect(shell[3]).toEqual(shell[0]);
    });
});

describe("MULTIPOLYGON – Morton dictionary encoded, with holes", () => {
    it("creates a multi-polygon with hole via Morton encoding", () => {
        const gv = encodeMultiPolygonGeometryVectorWithMortonOffsets([
            [
                [
                    [0, 0],
                    [10, 0],
                    [0, 10],
                ],
                [
                    [1, 1],
                    [3, 1],
                    [1, 3],
                ],
            ],
        ]);
        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
    });
});

describe("Vector with multiple geometries of different types", () => {
    it("converts geometries of different types in the same vector", () => {
        const pointGv = encodePointGeometryVector(1, 2);
        const multiPointGv = encodeMultiPointGeometryVector([
            [3, 4],
            [5, 6],
        ]);
        const lineStringGv = encodeLineStringGeometryVector([
            [7, 8],
            [9, 10],
        ]);
        const polygonGv = encodePolygonGeometryVector([
            [
                [11, 11],
                [11, 12],
                [12, 11],
            ],
        ]);
        const multiLineStringGv = encodeMultiLineStringGeometryVector([
            [
                [13, 14],
                [15, 16],
            ],
        ]);

        const gv = new FlatGeometryVector(
            VertexBufferType.VEC_2,
            new Uint32Array([
                GEOMETRY_TYPE.POINT,
                GEOMETRY_TYPE.MULTIPOINT,
                GEOMETRY_TYPE.LINESTRING,
                GEOMETRY_TYPE.POLYGON,
                GEOMETRY_TYPE.MULTILINESTRING,
            ]),
            {
                geometryOffsets: new Uint32Array([0, 1, 3, 5, 8, 9]),
                partOffsets: new Uint32Array([0, 1, 2, 3, 4, 5, 6]),
                ringOffsets: new Uint32Array([0, 1, 2, 3, 5, 8, 10]),
            },
            undefined,
            new Int32Array([
                ...pointGv.vertexBuffer,
                ...multiPointGv.vertexBuffer,
                ...lineStringGv.vertexBuffer,
                ...polygonGv.vertexBuffer,
                ...multiLineStringGv.vertexBuffer,
            ]),
        );

        const result = convertGeometryVector(gv);
        expect(result).toHaveLength(5);
        expect(result[0]).toEqual([[new Point(1, 2)]]);
        expect(result[1]).toEqual([[new Point(3, 4)], [new Point(5, 6)]]);
        expect(result[2]).toEqual([[new Point(7, 8), new Point(9, 10)]]);
        expect(result[3]).toEqual([
            [
                new Point(11, 11),
                new Point(11, 12),
                new Point(12, 11),
                new Point(11, 11), // closed
            ],
        ]);
        expect(result[4]).toEqual([[new Point(13, 14), new Point(15, 16)]]);
    });
});

describe("Error handling", () => {
    it("throws on unsupported geometry type", () => {
        const gv = {
            numGeometries: 1,
            topologyVector: {},
            geometryType: () => 999 as unknown as GEOMETRY_TYPE,
            containsPolygonGeometry: () => false,
        } as unknown as GeometryVector;

        expect(() => convertGeometryVector(gv)).toThrowError("The specified geometry type is currently not supported.");
    });
});

describe("Edge cases", () => {
    it("returns an empty array when numGeometries is 0", () => {
        const gv = {
            numGeometries: 0,
            topologyVector: {},
            containsPolygonGeometry: () => false,
        } as unknown as GeometryVector;
        expect(convertGeometryVector(gv)).toHaveLength(0);
    });

    it("treats vertexOffsets with length 0 as absent (sequential buffer path)", () => {
        const gv = new ConstGeometryVector(
            1,
            GEOMETRY_TYPE.POINT,
            VertexBufferType.VEC_2,
            {
                geometryOffsets: undefined,
                partOffsets: undefined,
                ringOffsets: undefined,
            },
            new Uint32Array([]), // length === 0 → same as undefined
            new Int32Array([3, 9]),
        );

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(3, 9)]]);
    });
});
