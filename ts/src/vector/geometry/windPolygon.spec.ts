import Point from "@mapbox/point-geometry";
import { describe, it, expect } from "vitest";
import decodeTile from "../../mltDecoder";
import { encodeTile, type FeatureGeometry } from "../../encoding/mltEncoder";
import { classifyRings } from "./classifyRings";
import { windPolygon } from "./windPolygon";

type Position = [number, number];

const ring = (...positions: Position[]) => positions.map(([x, y]) => new Point(x, y));

// The surveyor's formula, not the one under test: positive is clockwise with the y axis pointing down
function area(r: Point[]): number {
    let sum = 0;
    for (let i = 0; i + 1 < r.length; i++) {
        sum += r[i].x * r[i + 1].y - r[i + 1].x * r[i].y;
    }
    return sum / 2;
}

describe("windPolygon", () => {
    const exterior = ring([0, 0], [10, 0], [10, 10], [0, 10], [0, 0]);
    const hole = ring([2, 2], [2, 8], [8, 8], [8, 2], [2, 2]);

    it("keeps rings wound as MVT", () => {
        const rings = windPolygon([exterior.slice(), hole.slice()]);
        expect(rings).toEqual([exterior, hole]);
    });

    it("reverses an exterior with a negative area, keeping it closed and its first vertex", () => {
        const [wound] = windPolygon([exterior.slice().reverse()]);
        expect(wound).toEqual(exterior);
        expect(wound[0]).toEqual(wound[wound.length - 1]);
    });

    it("reverses a hole with a positive area", () => {
        const [, wound] = windPolygon([exterior.slice(), hole.slice().reverse()]);
        expect(wound).toEqual(hole);
    });

    it("winds each ring on its own", () => {
        const rings = windPolygon([exterior.slice().reverse(), exterior.slice(), hole.slice().reverse()]);
        // the second ring is a hole here, so it is reversed as well
        expect(rings.map(area).map(Math.sign)).toEqual([1, -1, -1]);
    });

    it("winds in place and returns the same rings", () => {
        const rings = [exterior.slice().reverse()];
        expect(windPolygon(rings)).toBe(rings);
        expect(area(rings[0])).toBeGreaterThan(0);
    });

    it("leaves a ring without an area alone", () => {
        const line = ring([0, 0], [5, 0], [10, 0], [0, 0]);
        expect(windPolygon([line.slice()])).toEqual([line]);
    });
});

describe("decodeTile gives renderers rings they can group by winding", () => {
    const square = (x0: number, y0: number, x1: number, y1: number): Position[] => [
        [x0, y0],
        [x1, y0],
        [x1, y1],
        [x0, y1],
    ];
    // The format lets an encoder wind each ring either way, so the tiles below do
    const reversed = (r: Position[]): Position[] => [r[0], ...r.slice(1).reverse()];

    function decodeRings(geometry: FeatureGeometry): Point[][] {
        const tile = encodeTile([{ name: "layer", extent: 64, features: [{ geometry }] }]);
        return decodeTile(tile, undefined, false)[0].getFeatures()[0].geometry.coordinates;
    }

    /** What a renderer does with the flat list of rings: take the exteriors and holes from the winding. */
    function classify(rings: Point[][]): string[][] {
        const polygons = classifyRings(rings.map((r) => r.map((p): Position => [p.x, p.y])));
        return polygons.map((polygon) =>
            polygon.map((r) =>
                r
                    .slice(0, -1)
                    .map(([x, y]) => `${x},${y}`)
                    .sort()
                    .join(" "),
            ),
        );
    }
    const vertices = (r: Position[]) =>
        r
            .map(([x, y]) => `${x},${y}`)
            .sort()
            .join(" ");

    const outer = square(0, 0, 40, 40);
    const inner = square(10, 10, 20, 20);

    it("groups a polygon written with every ring reversed", () => {
        const rings = decodeRings({ type: "Polygon", coordinates: [reversed(outer), inner] });
        expect(classify(rings)).toEqual([[vertices(outer), vertices(inner)]]);
    });

    it("groups a hole wound like its exterior with it", () => {
        const rings = decodeRings({ type: "Polygon", coordinates: [outer, reversed(inner).reverse()] });
        expect(classify(rings)).toEqual([[vertices(outer), vertices(inner)]]);
    });

    it("keeps the polygons of a multi-polygon apart whichever way they are wound", () => {
        const other = square(30, 30, 60, 60);
        const otherHole = square(35, 35, 40, 40);
        const rings = decodeRings({
            type: "MultiPolygon",
            coordinates: [
                [outer, inner],
                [reversed(other), otherHole],
            ],
        });
        expect(classify(rings)).toEqual([
            [vertices(outer), vertices(inner)],
            [vertices(other), vertices(otherHole)],
        ]);
    });
});
