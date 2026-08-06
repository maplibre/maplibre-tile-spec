import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import { compareWithTolerance, getTestCases, writeActualOutput } from "../../test/synthetic/synthetic-test-utils";
import decodeTile from "./mltDecoder";
import type { Geometry } from "./vector/geometry/geometryVector";
import type FeatureTable from "./vector/featureTable";

/** Synthetics that cannot be decoded yet. Keep empty: prefer fixing the decoder over skipping. */
const UNIMPLEMENTED_SYNTHETICS: string[] = [];

describe("MLT Decoder - Synthetic tests", () => {
    expect.addEqualityTesters([compareWithTolerance]);
    const testCases = getTestCases(UNIMPLEMENTED_SYNTHETICS);
    for (const { name, content, fileName } of testCases.active) {
        it(name, async () => {
            const actual = await decodeMLT(fileName);
            writeActualOutput(fileName, actual);
            expect(actual).toEqual(content);
        });
    }

    for (const skippedTest of testCases.skipped) {
        it.skip(skippedTest, () => {
            // Test is skipped since it is not supported yet
        });
    }
});

async function decodeMLT(mltFilePath: string) {
    const mltBuffer = await readFile(mltFilePath);
    const featureTables = decodeTile(mltBuffer, undefined, false);
    return featureTablesToFeatureCollection(featureTables) as unknown as Record<string, unknown>;
}

function featureTablesToFeatureCollection(featureTables: FeatureTable[]): GeoJSON.FeatureCollection {
    const features: GeoJSON.Feature[] = [];
    for (const table of featureTables) {
        for (const feature of table.getFeatures()) {
            const geojsonFeature: GeoJSON.Feature = {
                type: "Feature",
                geometry: getGeometry(feature.geometry),
                properties: {
                    _layer: table.name,
                    _extent: table.extent,
                    ...Object.fromEntries(Object.entries(feature.properties).map(([k, v]) => [k, safeNumber(v)])),
                },
            };
            const safeId = safeNumber(feature.id);
            if (safeId !== null && safeId !== undefined) {
                geojsonFeature.id = safeId;
            }
            features.push(geojsonFeature);
        }
    }
    return { type: "FeatureCollection", features };
}

function safeNumber<T>(val: bigint | T): T | number {
    return typeof val === "bigint" ? Number(val) : val;
}

function getGeometry(geometry: Geometry): GeoJSON.Geometry {
    const coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));
    switch (geometry.type) {
        case GEOMETRY_TYPE.POINT:
            return { type: "Point", coordinates: coords[0][0] };
        case GEOMETRY_TYPE.LINESTRING:
            return { type: "LineString", coordinates: coords[0] };
        case GEOMETRY_TYPE.POLYGON:
            return { type: "Polygon", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOINT:
            return { type: "MultiPoint", coordinates: coords.map((r) => r[0]) };
        case GEOMETRY_TYPE.MULTILINESTRING:
            return { type: "MultiLineString", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOLYGON:
            return { type: "MultiPolygon", coordinates: classifyRings(coords) };
        default:
            throw new Error(`Unsupported geometry type: ${geometry.type}`);
    }
}

/**
 * Splits the flat ring list of a multi-polygon into its polygons by winding order, following the
 * same rule as `classifyRings` in the style-spec: a ring wound like the current polygon's exterior
 * starts a new polygon, a ring wound the other way is one of its holes.
 *
 * It differs from the style-spec version in how it treats degenerate rings. That one skips any ring
 * whose signed area is zero, which silently drops rings the decoder returned correctly
 *
 * Note that a degenerate exterior has no winding to compare against, so the rings that follow it are
 * treated as its holes. Grouping is a property of the topology vector (partOffsets), not of the
 * coordinates, so for such rings this is a convention rather than something the geometry implies.
 */
function classifyRings(rings: number[][][]): number[][][][] {
    const polygons: number[][][][] = [];
    let polygon: number[][][] | undefined;
    let exteriorIsCcw: boolean | undefined;

    for (const ring of rings) {
        const area = signedArea(ring);
        // A degenerate ring has no winding of its own, so it always starts a new polygon. Rings
        // following a degenerate exterior have nothing to match against and become its holes.
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

function signedArea(ring: number[][]): number {
    let sum = 0;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        sum += (ring[j][0] - ring[i][0]) * (ring[i][1] + ring[j][1]);
    }
    return sum;
}
