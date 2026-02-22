import { readdir, readFile } from "node:fs/promises";
import { join, parse, resolve } from "node:path";
import JSON5 from "json5";
import { describe, expect, it } from "vitest";
import decodeTile from "./mltDecoder";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import type { Geometry } from "./vector/geometry/geometryVector";
import type FeatureTable from "./vector/featureTable";

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

const UNIMPLEMENTED_SYNTHETICS = new Map([
    ["polygon_multi_fpf", "FastPFor not implemented"],
    ["polygon_morton_tes", "Morton not implemented"],
    ["polygon_hole_fpf", "FastPFor not implemented"],
    ["polygon_fpf", "FastPFor not implemented"],
    ["props_mixed", "F32 very buggy and does not work"],
    ["prop_f64", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_f64_min", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_f64_max", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_f64_nan", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_f64_neg_inf", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_f64_neg_zero", "throws error 'byte length of Float64Array should be a multiple of 8'"],
    ["prop_str_empty", "empty strings not supported"],
    ["prop_i64_min", "wraps to 0 despite it should have been negative"],
    ["prop_i64_max", "wraps to -1 despite it should have been positive"],
]);

const syntheticDir = resolve(__dirname, "../../test/synthetic/0x01");
const files = await readdir(syntheticDir);
const testNames = files
    .filter((f) => f.endsWith(".mlt"))
    .map((f) => parse(f).name)
    .sort();
// Separate tests into two groups
const activeList = testNames.filter((name) => !UNIMPLEMENTED_SYNTHETICS.has(name));
const skippedTests = testNames
    .filter((name) => UNIMPLEMENTED_SYNTHETICS.has(name))
    .map((name) => [name, UNIMPLEMENTED_SYNTHETICS.get(name)] as const);

describe("MLT Decoder - Synthetic tests", () => {
    /**
     * This adds number handing comparison logic to vitest for number which are close enough, but would fail the regular comparison.
     */
    expect.addEqualityTesters([
        (received, expected) => {
            if (typeof received === "number" && typeof expected === "number") {
                // Handle Infinity/NaN
                if (!Number.isFinite(expected)) return Object.is(received, expected);

                // Handle Close to Zero
                if (Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
                    return Math.abs(received) <= ABSOLUTE_FLOAT_TOLERANCE;
                }

                // Handle Relative Tolerance
                const relativeError = Math.abs(received - expected) / Math.abs(expected);
                return relativeError <= RELATIVE_FLOAT_TOLERANCE;
            }
            // Return undefined to let Vitest handle non-number types normally
            return undefined;
        },
    ]);

    for (const testName of activeList) {
        it(`should decode ${testName}`, async () => {
            const [mltBuffer, jsonRaw] = await Promise.all([
                readFile(join(syntheticDir, `${testName}.mlt`)),
                readFile(join(syntheticDir, `${testName}.json`), "utf-8"),
            ]);

            const featureTables = decodeTile(mltBuffer, null, false);
            const actualJson = featureTablesToFeatureCollection(featureTables);
            const expectedJson = JSON5.parse(jsonRaw);
            expect(actualJson).toEqual(expectedJson);
        });
    }

    for (const skipTest of skippedTests) {
        it.skip(`should decode ${skipTest[0]} (skipped because: ${skipTest[1]})`, () => {});
    }
});

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
            if (safeNumber(feature.id) !== null) {
                geojsonFeature.id = safeNumber(feature.id);
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
            return { type: "MultiPolygon", coordinates: coords.map((r) => [r]) };
        default:
            throw new Error(`Unsupported geometry type: ${geometry.type}`);
    }
}
