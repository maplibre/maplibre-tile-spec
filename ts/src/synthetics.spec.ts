import { readdir, readFile } from "node:fs/promises";
import { join, parse, resolve } from "node:path";
import type { LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon } from "geojson";
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

describe("MLT Decoder - Synthetic tests", async () => {
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

    it.each(activeList)("should decode %s", async (testName) => {
        const [mltBuffer, jsonRaw] = await Promise.all([
            readFile(join(syntheticDir, `${testName}.mlt`)),
            readFile(join(syntheticDir, `${testName}.json`), "utf-8"),
        ]);

        const featureTables = decodeTile(mltBuffer);
        const expectedJson = wrapWithMatchers(JSON5.parse(jsonRaw));

        const actualFeatures = featureTables.flatMap((table) =>
            table.getFeatures().map((f) => ({
                type: "Feature",
                id: safeNumber(f.id) ?? 0,
                geometry: getGeometry(f.geometry),
                properties: {
                    _layer: table.name,
                    _extent: table.extent,
                    ...Object.fromEntries(Object.entries(f.properties).map(([k, v]) => [k, safeNumber(v)])),
                },
            })),
        );

        expect({ type: "FeatureCollection", features: actualFeatures }).toEqual(expectedJson);
    });
    it.skip.each(skippedTests)("should decode %s (skipped because: %s)", () => {});
});

function safeNumber<T>(val: bigint | T): T | number {
    return typeof val === "bigint" ? Number(val) : val;
}

// Custom geometry types with CRS support
const CRS = {
    type: "name",
    properties: {
        name: "EPSG:0",
    },
} as const;

type PointWithCRS = Point & { crs: typeof CRS };
type LineStringWithCRS = LineString & { crs: typeof CRS };
type PolygonWithCRS = Polygon & { crs: typeof CRS };
type MultiPointWithCRS = MultiPoint & { crs: typeof CRS };
type MultiLineStringWithCRS = MultiLineString & { crs: typeof CRS };
type MultiPolygonWithCRS = MultiPolygon & { crs: typeof CRS };

type GeometryWithCRS =
    | PointWithCRS
    | LineStringWithCRS
    | PolygonWithCRS
    | MultiPointWithCRS
    | MultiLineStringWithCRS
    | MultiPolygonWithCRS;

function getGeometry(geometry: Geometry): GeometryWithCRS {
    const coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));

    const mapping: Record<number, () => GeometryWithCRS> = {
        [GEOMETRY_TYPE.POINT]: () => ({ type: "Point", coordinates: coords[0][0] }) as PointWithCRS,
        [GEOMETRY_TYPE.LINESTRING]: () => ({ type: "LineString", coordinates: coords[0] }) as LineStringWithCRS,
        [GEOMETRY_TYPE.POLYGON]: () => ({ type: "Polygon", coordinates: coords }) as PolygonWithCRS,
        [GEOMETRY_TYPE.MULTIPOINT]: () =>
            ({ type: "MultiPoint", coordinates: coords.map((r) => r[0]) }) as MultiPointWithCRS,
        [GEOMETRY_TYPE.MULTILINESTRING]: () =>
            ({ type: "MultiLineString", coordinates: coords }) as MultiLineStringWithCRS,
        [GEOMETRY_TYPE.MULTIPOLYGON]: () =>
            ({ type: "MultiPolygon", coordinates: coords.map((r) => [r]) }) as MultiPolygonWithCRS,
    };

    const result = mapping[geometry.type]?.();
    if (!result) throw new Error(`Unsupported geometry type: ${geometry.type}`);

    return { ...result, crs: CRS };
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
