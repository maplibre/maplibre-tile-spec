import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { classifyRings } from "@maplibre/maplibre-gl-style-spec";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import { compareWithTolerance, getTestCases, writeActualOutput } from "../../test/synthetic/synthetic-test-utils";
import decodeTile from "./mltDecoder";
import type { Geometry } from "./vector/geometry/geometryVector";
import type FeatureTable from "./vector/featureTable";

const EARCUT_MAX_RINGS = 500;

const UNIMPLEMENTED_SYNTHETICS = new Map([
    //["prop_u32_max", "wraps to -1 despite it should have been positive"],
    ["prop_u64_max", "wraps to -1 despite it should have been positive"],
    ["poly_multi_morton_ring_morton", "decodes to zero coordinates"],
    ["poly_multi_morton_ring_no_morton", "decodes to zero coordinates"],
]);

describe("MLT Decoder - Synthetic tests", () => {
    expect.addEqualityTesters([compareWithTolerance]);
    const testCases = getTestCases(Array.from(UNIMPLEMENTED_SYNTHETICS.keys()));
    for (const { name, content, fileName } of testCases.active) {
        if (name !== "prop_u32_max") continue;
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
        case GEOMETRY_TYPE.MULTIPOLYGON: {
            const polygons = classifyRings(geometry.coordinates, EARCUT_MAX_RINGS);
            return {
                type: "MultiPolygon",
                coordinates: polygons.map((polygon) => polygon.map((ring) => ring.map((p) => [p.x, p.y]))),
            };
        }
        default:
            throw new Error(`Unsupported geometry type: ${geometry.type}`);
    }
}
