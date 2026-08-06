import type Point from "@mapbox/point-geometry";

/**
 * Splits a multi-polygon's flat ring list into polygons by winding order: a ring wound like the
 * current polygon's exterior starts a new polygon, a ring wound the other way is one of its holes.
 *
 * This follows the same rule as `classifyRings` in the style-spec, with one difference: that one
 * skips any ring whose signed area is zero, silently dropping rings the decoder returned correctly.
 * Several synthetic fixtures are deliberately degenerate — self-intersecting "bow-ties" with zero
 * area — and discarding them hides real regressions. Here a degenerate ring starts a new polygon
 * instead, and no ring is ever dropped.
 *
 * Note that a degenerate exterior has no winding to compare against, so the rings that follow it
 * are treated as its holes. Grouping is really a property of the topology vector (partOffsets), not
 * of the coordinates, so for such rings this is a convention rather than something the geometry
 * implies.
 */
export function classifyRings(rings: Point[][]): Point[][][] {
    const polygons: Point[][][] = [];
    let polygon: Point[][] | undefined;
    let exteriorIsCcw: boolean | undefined;

    for (const ring of rings) {
        const area = signedArea(ring);
        // A degenerate ring has no winding of its own, so it always starts a new polygon.
        if (!polygon || area === 0 || (area < 0) === exteriorIsCcw) {
            if (polygon) polygons.push(polygon);
            polygon = [ring];
            exteriorIsCcw = area === 0 ? undefined : area < 0;
        } else {
            polygon.push(ring);
        }
    }

    if (polygon) polygons.push(polygon);
    return polygons;
}

function signedArea(ring: Point[]): number {
    let sum = 0;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        sum += (ring[j].x - ring[i].x) * (ring[i].y + ring[j].y);
    }
    return sum;
}
