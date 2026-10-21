import type Point from "@mapbox/point-geometry";

/**
 * Winds the rings of one polygon as MVT does, whatever the encoder wrote: the exterior has a positive
 * area by the surveyor's formula in tile coordinates (clockwise with the y axis pointing down) and
 * the holes a negative one. The rings are reversed in place, a closed ring stays closed.
 *
 * The first ring is the exterior and the others are its holes, as the topology streams say. The
 * format lets an encoder wind the rings either way, but a renderer that reads the flat list of rings
 * of a multi-polygon can only tell the polygons and holes apart by their winding, and the walls of
 * extruded polygons only face outward when the exterior is wound this way. Rings without an area
 * are left alone.
 */
export function windPolygon(rings: Point[][]): Point[][] {
    for (let i = 0; i < rings.length; i++) {
        const area = signedArea(rings[i]);
        if (area !== 0 && area > 0 !== (i === 0)) {
            rings[i].reverse();
        }
    }
    return rings;
}

function signedArea(ring: Point[]): number {
    let sum = 0;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        sum += (ring[j].x - ring[i].x) * (ring[i].y + ring[j].y);
    }
    return sum;
}
