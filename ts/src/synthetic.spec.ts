import { readdir, readFile } from "node:fs/promises";
import { join, parse, resolve } from "node:path";
import type {
    LineString,
    MultiLineString,
    MultiPoint,
    MultiPolygon,
    Point,
    Polygon,
    Geometry as GeoJSONGeometry,
} from "geojson";
import JSON5 from "json5";
import { describe, expect, it } from "vitest";
import decodeTile from "./mltDecoder";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import type { Geometry } from "./vector/geometry/geometryVector";

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
    it.each(activeList)("should decode %s", async (testName) => {
        const [mltBuffer, jsonRaw] = await Promise.all([
            readFile(join(syntheticDir, `${testName}.mlt`)),
            readFile(join(syntheticDir, `${testName}.json`), "utf-8"),
        ]);

        const featureTables = decodeTile(mltBuffer, null, false);
        const expectedJson = wrapWithMatchers(JSON5.parse(jsonRaw));

        const actualFeatures = featureTables.flatMap((table) =>
            table.getFeatures().map((f) => {
                const feature: GeoJSON.Feature = {
                    type: "Feature",
                    geometry: getGeometry(f.geometry),
                    properties: {
                        _layer: table.name,
                        _extent: table.extent,
                        ...Object.fromEntries(Object.entries(f.properties).map(([k, v]) => [k, safeNumber(v)])),
                    },
                };
                if (safeNumber(f.id) !== null) {
                    feature.id = safeNumber(f.id);
                }
                return feature;
            }),
        );
        //console.log("From file:\n" + JSON.stringify(expectedJson, null, 2));
        //console.log("From test:\n" + JSON.stringify({ type: "FeatureCollection", features: actualFeatures }, null, 2));
        expect({ type: "FeatureCollection", features: actualFeatures }).toEqual(expectedJson);
    });
    it.skip.each(skippedTests)("should decode %s (skipped because: %s)", () => {});
});

function safeNumber<T>(val: bigint | T): T | number {
    return typeof val === "bigint" ? Number(val) : val;
}

function getGeometry(geometry: Geometry): GeoJSONGeometry {
    const coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));

    const mapping: Record<number, () => GeoJSONGeometry> = {
        [GEOMETRY_TYPE.POINT]: () => ({ type: "Point", coordinates: coords[0][0] }) as Point,
        [GEOMETRY_TYPE.LINESTRING]: () => ({ type: "LineString", coordinates: coords[0] }) as LineString,
        [GEOMETRY_TYPE.POLYGON]: () => ({ type: "Polygon", coordinates: coords }) as Polygon,
        [GEOMETRY_TYPE.MULTIPOINT]: () => ({ type: "MultiPoint", coordinates: coords.map((r) => r[0]) }) as MultiPoint,
        [GEOMETRY_TYPE.MULTILINESTRING]: () => ({ type: "MultiLineString", coordinates: coords }) as MultiLineString,
        [GEOMETRY_TYPE.MULTIPOLYGON]: () =>
            ({ type: "MultiPolygon", coordinates: coords.map((r) => [r]) }) as MultiPolygon,
    };

    const result = mapping[geometry.type]?.();
    if (!result) throw new Error(`Unsupported geometry type: ${geometry.type}`);

    return result;
}

type MatchableValue = string | number | boolean | null | MatchableValue[] | { [key: string]: MatchableValue };
function wrapWithMatchers<T extends MatchableValue>(value: T): T {
    if (typeof value === "number" && Number.isFinite(value) && !Number.isSafeInteger(value)) {
        // We cast to T because Vitest matchers are designed to be
        return (expect as any).toBeRelativelyClose(value) as T;
    }
    if (Array.isArray(value)) {
        return value.map(wrapWithMatchers) as unknown as T;
    }
    if (value !== null && typeof value === "object") {
        return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, wrapWithMatchers(v)])) as unknown as T;
    }
    return value;
}

/** Custom Vitest Matcher */
expect.extend({
    toBeRelativelyClose(received: number, expected: number) {
        // Handle Infinity, -Infinity, and NaN
        if (!Number.isFinite(expected)) {
            const pass = Object.is(received, expected); // Object.is handles NaN correctly
            return {
                pass,
                message: () => `expected ${received} to be exactly ${expected}`,
            };
        }

        const isCloseToZero = Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE;
        if (isCloseToZero) {
            const pass = Math.abs(received) <= ABSOLUTE_FLOAT_TOLERANCE;
            return {
                pass,
                message: () => `expected ${received} to be close to 0 (precision: ${ABSOLUTE_FLOAT_TOLERANCE})`,
            };
        }

        const relativeError = Math.abs(received - expected) / Math.abs(expected);
        const pass = relativeError <= RELATIVE_FLOAT_TOLERANCE;

        return {
            pass,
            message: () =>
                `expected ${received} to be close to ${expected} (error: ${(relativeError * 100).toFixed(6)}%)`,
        };
    },
});