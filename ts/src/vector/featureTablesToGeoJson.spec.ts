import { describe, expect, it } from "vitest";
import decodeTile from "../mltDecoder";
import { encodeTile, type FeatureGeometry } from "../encoding/mltEncoder";
import { featureTablesToFeatureCollection } from "./featureTablesToGeoJson";

function toGeoJsonGeometry(geometry: FeatureGeometry): GeoJSON.Geometry {
    const tile = encodeTile([{ name: "layer", features: [{ geometry }] }]);
    const collection = featureTablesToFeatureCollection(decodeTile(tile, undefined, false));
    expect(collection.features).toHaveLength(1);
    return collection.features[0].geometry;
}

/** A closed square ring, wound one way or the other. */
function square(min: number, size: number, clockwise: boolean): number[][] {
    const max = min + size;
    const ring = [
        [min, min],
        [max, min],
        [max, max],
        [min, max],
    ];
    if (clockwise) ring.reverse();
    return [...ring, ring[0]];
}

// MLT does not constrain ring winding, so the grouping must come from the encoded topology, not be
// guessed from the winding order.
describe("featureTablesToFeatureCollection - MultiPolygon", () => {
    it("keeps a hole wound like its exterior inside its polygon", () => {
        const geometry: FeatureGeometry = {
            type: "MultiPolygon",
            coordinates: [[square(0, 10, false), square(2, 2, true)], [square(20, 10, true)]],
        };
        expect(toGeoJsonGeometry(geometry)).toEqual(geometry);
    })
    it("keeps an exterior wound like the previous hole as a new polygon", () => {
        const geometry: FeatureGeometry = {
            type: "MultiPolygon",
            coordinates: [[square(0, 10, false), square(2, 2, false)], [square(20, 10, false)]],
        };
        expect(toGeoJsonGeometry(geometry)).toEqual(geometry);
    })
});
