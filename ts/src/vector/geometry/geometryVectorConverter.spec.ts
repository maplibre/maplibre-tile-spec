import Point from "@mapbox/point-geometry";
import { describe, it, expect } from "vitest";
import { convertGeometryVector } from "./geometryVectorConverter";
import { GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import { encodeZOrderCurve } from "../../encoding/zOrderCurveEncoder";
import type { GeometryVector, MortonSettings } from "./geometryVector";

const DEFAULT_MORTON_SETTINGS: MortonSettings = { numBits: 16, coordinateShift: 0 } as MortonSettings;

function encode(x: number, y: number): number {
    return encodeZOrderCurve(x, y, DEFAULT_MORTON_SETTINGS.numBits, DEFAULT_MORTON_SETTINGS.coordinateShift);
}

function makeGeometryVector(overrides: Partial<GeometryVector> = {}): GeometryVector {
    return {
        numGeometries: 0,
        vertexBuffer: new Int32Array([]),
        vertexOffsets: undefined,
        vertexBufferType: VertexBufferType.VEC_2,
        mortonSettings: DEFAULT_MORTON_SETTINGS,
        topologyVector: {
            geometryOffsets: undefined,
            partOffsets: undefined,
            ringOffsets: undefined,
        },
        geometryType: (_i: number) => GEOMETRY_TYPE.POINT,
        containsPolygonGeometry: () => false,
        ...overrides,
    } as unknown as GeometryVector;
}

describe("POINT – sequential vertex buffer (no vertexOffsets)", () => {
    it("creates a single point from the vertex buffer", () => {
        const x = 5;
        const y = 7;
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([x, y]),
            vertexOffsets: undefined,
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

        const result = convertGeometryVector(gv);

        expect(result).toHaveLength(1);
        expect(result[0]).toEqual([[new Point(x, y)]]);
    });

    it("creates multiple points sequentially", () => {
        const gv = makeGeometryVector({
            numGeometries: 2,
            vertexBuffer: new Int32Array([1, 2, 3, 4]),
            vertexOffsets: undefined,
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

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
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([99, 99, x, y]),
            vertexOffsets: new Int32Array([1]),
            vertexBufferType: VertexBufferType.VEC_2,
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(x, y)]]);
    });
});

describe("POINT – Morton dictionary encoded", () => {
    it("decodes a point via morton encoding", () => {
        const x = 3;
        const y = 7;
        const code = encode(x, y);

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([code]),
            vertexOffsets: new Int32Array([0]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(x, y)]]);
    });

    it("decodes a point with a non-zero coordinateShift", () => {
        const x = 50;
        const y = 80;
        const settings: MortonSettings = { numBits: 16, coordinateShift: 100 } as MortonSettings;
        const code = encodeZOrderCurve(x, y, settings.numBits, settings.coordinateShift);

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([code]),
            vertexOffsets: new Int32Array([0]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: settings,
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(x, y)]]);
    });
});

describe("MULTIPOINT – sequential vertex buffer", () => {
    it("creates a multi-point geometry", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([1, 2, 3, 4, 5, 6]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 3]),
                partOffsets: undefined,
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOINT,
        });

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(1, 2)], [new Point(3, 4)], [new Point(5, 6)]]);
    });
});

describe("MULTIPOINT – VEC_2 dictionary encoded", () => {
    it("reads multi-point via vertexOffsets", () => {
        // offsets [0, 2] → vertexBuffer[0,1] and vertexBuffer[4,5]
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([10, 20, 99, 99, 30, 40]),
            vertexOffsets: new Int32Array([0, 2]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: undefined,
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOINT,
        });

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(10, 20)], [new Point(30, 40)]]);
    });
});

describe("LINESTRING – sequential vertex buffer, no polygon context", () => {
    it("creates a line string from sequential vertices", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 1, 1, 2, 2]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 3]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.LINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);

        expect(result[0]).toEqual([[new Point(0, 0), new Point(1, 1), new Point(2, 2)]]);
    });
});

describe("LINESTRING – sequential vertex buffer, polygon context (uses ringOffsets)", () => {
    it("creates a line string using ringOffsets when containsPolygon is true", () => {
        const gv = makeGeometryVector({
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
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(1, 2), new Point(3, 4)]]);
    });
});

describe("LINESTRING – VEC_2 dictionary encoded", () => {
    it("decodes a line string via vertexOffsets VEC_2", () => {
        // offsets [0, 1] → vertexBuffer[0,1] and [2,3]
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([5, 10, 15, 20]),
            vertexOffsets: new Int32Array([0, 1]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.LINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(5, 10), new Point(15, 20)]]);
    });
});

describe("LINESTRING – Morton dictionary encoded", () => {
    it("decodes a line string via Morton encoding", () => {
        const code0 = encode(1, 2);
        const code1 = encode(3, 4);

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([code0, code1]),
            vertexOffsets: new Int32Array([0, 1]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.LINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(1, 2), new Point(3, 4)]]);
    });
});

describe("POLYGON – sequential vertex buffer, no holes", () => {
    it("creates a polygon shell that is closed", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 0, 1, 1, 1, 1, 0]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 4]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(5); // 4 + closed
        expect(shell[0]).toEqual(shell[4]);
    });
});

describe("POLYGON – sequential vertex buffer, with hole", () => {
    it("creates a polygon with one hole", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 10, 0, 0, 10, 2, 2, 4, 2, 2, 4]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2); // [shell, hole]
    });
});

describe("POLYGON – VEC_2 dictionary encoded, no holes", () => {
    it("creates a closed polygon shell via VEC_2 vertex offsets", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 5, 5, 10, 0]),
            vertexOffsets: new Int32Array([0, 1, 2]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 3]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(4); // closed
        expect(shell[0]).toEqual(shell[3]);
    });
});

describe("POLYGON – VEC_2 dictionary encoded, with hole", () => {
    it("creates a polygon with one hole via VEC_2 vertex offsets", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 10, 0, 0, 10, 2, 2, 4, 2, 2, 4]),
            vertexOffsets: new Int32Array([0, 1, 2, 3, 4, 5]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2); // [shell, hole]
    });
});

describe("POLYGON – Morton dictionary encoded, no holes", () => {
    it("creates a closed polygon shell via Morton encoding", () => {
        const pts = [
            [0, 0],
            [5, 0],
            [0, 5],
        ] as const;
        const codes = pts.map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1, 2]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 3]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

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
        const shellPts = [
            [0, 0],
            [10, 0],
            [0, 10],
        ] as const;
        const holePts = [
            [1, 1],
            [3, 1],
            [1, 3],
        ] as const;
        const codes = [...shellPts, ...holePts].map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1, 2, 3, 4, 5]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: undefined,
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
    });
});

describe("POLYGON – geometryOffsets counter incremented", () => {
    it("does not throw and returns correct polygon when geometryOffsets is present", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 1, 0, 0, 1]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 3]),
            },
            geometryType: () => GEOMETRY_TYPE.POLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0][0]).toHaveLength(4);
    });
});

describe("MULTILINESTRING – sequential vertex buffer, no polygon context", () => {
    it("creates a multi-line string with two line strings", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 1, 1, 2, 2, 3, 3]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 2, 4]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.MULTILINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toEqual([new Point(0, 0), new Point(1, 1)]);
        expect(result[0][1]).toEqual([new Point(2, 2), new Point(3, 3)]);
    });
});

describe("MULTILINESTRING – sequential vertex buffer, polygon context (uses ringOffsets)", () => {
    it("uses ringOffsets for vertex counts when containsPolygon is true", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 1, 1]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 2]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTILINESTRING,
            containsPolygonGeometry: () => true,
        });

        const result = convertGeometryVector(gv);
        expect(result[0][0]).toEqual([new Point(0, 0), new Point(1, 1)]);
    });
});

describe("MULTILINESTRING – VEC_2 dictionary encoded", () => {
    it("decodes multi-line string via vertexOffsets VEC_2", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 1, 2, 3, 4, 5, 6, 7]),
            vertexOffsets: new Int32Array([0, 1, 2, 3]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 2, 4]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.MULTILINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toEqual([new Point(0, 1), new Point(2, 3)]);
        expect(result[0][1]).toEqual([new Point(4, 5), new Point(6, 7)]);
    });
});

describe("MULTILINESTRING – Morton dictionary encoded", () => {
    it("decodes multi-line string via Morton encoding", () => {
        const pts = [
            [1, 2],
            [3, 4],
        ] as const;
        const codes = pts.map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 1, 2]),
                ringOffsets: undefined,
            },
            geometryType: () => GEOMETRY_TYPE.MULTILINESTRING,
            containsPolygonGeometry: () => false,
        });

        const result = convertGeometryVector(gv);
        expect(result[0][0]).toEqual([new Point(1, 2)]);
        expect(result[0][1]).toEqual([new Point(3, 4)]);
    });
});

describe("MULTILINESTRING – Morton encoded, polygon context", () => {
    it("uses ringOffsets for vertex counts when containsPolygon is true", () => {
        const pts = [
            [5, 6],
            [7, 8],
        ] as const;
        const codes = pts.map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 1, 2]),
                ringOffsets: new Uint32Array([0, 1, 2]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTILINESTRING,
            containsPolygonGeometry: () => true,
        });

        const result = convertGeometryVector(gv);
        expect(result[0][0]).toEqual([new Point(5, 6)]);
        expect(result[0][1]).toEqual([new Point(7, 8)]);
    });
});

describe("MULTIPOLYGON – sequential vertex buffer, no holes", () => {
    it("creates a multi-polygon with two triangular polygons", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 1, 0, 0, 1, 5, 5, 6, 5, 5, 6]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 1, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // closed shell
        expect(result[0][1]).toHaveLength(4); // closed shell
    });
});

describe("MULTIPOLYGON – sequential vertex buffer, with holes", () => {
    it("creates a polygon that has a hole", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 10, 0, 0, 10, 1, 1, 3, 1, 1, 3]),
            vertexOffsets: undefined,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2); // [shell, hole]
    });
});

describe("MULTIPOLYGON – VEC_2 dictionary encoded, no holes", () => {
    it("decodes multi-polygon via vertexOffsets VEC_2", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 2, 0, 0, 2, 5, 5, 7, 5, 5, 7]),
            vertexOffsets: new Int32Array([0, 1, 2, 3, 4, 5]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 2]),
                partOffsets: new Uint32Array([0, 1, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
        expect(result[0][0]).toHaveLength(4); // closed
    });
});

describe("MULTIPOLYGON – VEC_2 dictionary encoded, with holes", () => {
    it("creates a multi-polygon with hole via VEC_2", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([0, 0, 10, 0, 0, 10, 1, 1, 3, 1, 1, 3]),
            vertexOffsets: new Int32Array([0, 1, 2, 3, 4, 5]),
            vertexBufferType: VertexBufferType.VEC_2,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
    });
});

describe("MULTIPOLYGON – Morton dictionary encoded, no holes", () => {
    it("decodes multi-polygon shells via Morton encoding", () => {
        const shellPts = [
            [0, 0],
            [1, 0],
            [0, 1],
        ] as const;
        const codes = shellPts.map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1, 2]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 1]),
                ringOffsets: new Uint32Array([0, 3]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        const shell = result[0][0];
        expect(shell).toHaveLength(4); // closed
        expect(shell[0]).toEqual(new Point(0, 0));
        expect(shell[3]).toEqual(shell[0]);
    });
});

describe("MULTIPOLYGON – Morton dictionary encoded, with holes", () => {
    it("creates a multi-polygon with hole via Morton encoding", () => {
        const shellPts = [
            [0, 0],
            [10, 0],
            [0, 10],
        ] as const;
        const holePts = [
            [1, 1],
            [3, 1],
            [1, 3],
        ] as const;
        const codes = [...shellPts, ...holePts].map(([x, y]) => encode(x, y));

        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array(codes),
            vertexOffsets: new Int32Array([0, 1, 2, 3, 4, 5]),
            vertexBufferType: VertexBufferType.MORTON,
            mortonSettings: DEFAULT_MORTON_SETTINGS,
            topologyVector: {
                geometryOffsets: new Uint32Array([0, 1]),
                partOffsets: new Uint32Array([0, 2]),
                ringOffsets: new Uint32Array([0, 3, 6]),
            },
            geometryType: () => GEOMETRY_TYPE.MULTIPOLYGON,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toHaveLength(2);
    });
});

describe("Error handling", () => {
    it("throws on unsupported geometry type", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            geometryType: () => 999 as unknown as GEOMETRY_TYPE,
        });

        expect(() => convertGeometryVector(gv)).toThrowError("The specified geometry type is currently not supported.");
    });
});

describe("Edge cases", () => {
    it("returns an empty array when numGeometries is 0", () => {
        const gv = makeGeometryVector({ numGeometries: 0 });
        expect(convertGeometryVector(gv)).toHaveLength(0);
    });

    it("treats vertexOffsets with length 0 as absent (sequential buffer path)", () => {
        const gv = makeGeometryVector({
            numGeometries: 1,
            vertexBuffer: new Int32Array([3, 9]),
            vertexOffsets: new Int32Array([]), // length === 0 → same as undefined
            geometryType: () => GEOMETRY_TYPE.POINT,
        });

        const result = convertGeometryVector(gv);
        expect(result[0]).toEqual([[new Point(3, 9)]]);
    });
});
