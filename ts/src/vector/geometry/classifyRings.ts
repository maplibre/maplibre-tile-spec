/**
 * Splits the flat ring list of a multi-polygon into its polygons by winding order, following the
 * same rule as `classifyRings` in the style-spec: a ring wound like the current polygon's exterior
 * starts a new polygon, a ring wound the other way is one of its holes.
 *
 * It differs from the style-spec version in how it treats degenerate rings. That one skips any ring
 * whose signed area is zero, which silently drops rings the decoder returned correctly. Several
 * synthetic fixtures are deliberately degenerate - self-intersecting "bow-ties" with zero area - so
 * a heuristic that discards them hides real regressions. Here a degenerate ring starts a new
 * polygon instead, and no ring is ever dropped.
 *
 * Note that a degenerate exterior has no winding to compare against, so the rings that follow it
 * are treated as its holes. Grouping is a property of the topology vector (partOffsets), not of the
 * coordinates, so for such rings this is a convention rather than something the geometry implies.
 */
export function classifyRings(rings: number[][][]): number[][][][] {
    const polygons: number[][][][] = [];
    let polygon: number[][][] | undefined;
    let exteriorIsCcw: boolean | undefined;

    for (const ring of rings) {
        const area = signedArea(ring);
        // A degenerate ring has no winding of its own, so it always starts a new polygon. Rings
        // following a degenerate exterior have nothing to match against and become its holes.
        if (!polygon || area === 0 || area < 0 === exteriorIsCcw) {
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

function signedArea(ring: number[][]): number {
    let sum = 0;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        sum += (ring[j][0] - ring[i][0]) * (ring[i][1] + ring[j][1]);
    }
    return sum;
}
