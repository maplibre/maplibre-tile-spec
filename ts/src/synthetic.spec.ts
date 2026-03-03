import { readFile } from "node:fs/promises";
import { describe, it } from "vitest";
import decodeTile from "./mltDecoder";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import type { Geometry } from "./vector/geometry/geometryVector";
import type FeatureTable from "./vector/featureTable";
import { classifyRings } from "@maplibre/maplibre-gl-style-spec";

import {
    SyntheticTestRunner,
    type SyntheticCaseResult,
} from "synthetic-test-tool";

const EARCUT_MAX_RINGS = 500;

const UNIMPLEMENTED_SYNTHETICS = new Map([
    ["props_mixed", "F32 very buggy and does not work"],
    ["prop_str_empty", "empty strings not supported"],
    ["prop_str_empty_val", "empty strings not supported"],
    ["prop_str_val_empty", "empty strings not supported"],
    ["prop_i64_min", "wraps to 0 despite it should have been negative"],
    ["prop_u32_max", "wraps to -1 despite it should have been positive"],
    ["prop_i64_max", "wraps to -1 despite it should have been positive"],
    ["prop_u64_max", "wraps to -1 despite it should have been positive"],
    ["prop_f32_nan", "geometry diff"],
    ["prop_f32_neg_inf", "geometry diff"],
    ["prop_f32_pos_inf", "geometry diff"],
    ["prop_f64", "does not parse f64 as f32"],
    ["prop_f64_max", "does not parse f64 as f32"],
    ["prop_f64_min_norm", "does not parse f64 as f32"],
    ["prop_f64_min_val", "does not parse f64 as f32"],
    ["prop_f64_nan", "does not parse f64 as f32"],
    ["prop_f64_neg_inf", "does not parse f64 as f32"],
    ["prop_f64_neg_zero", "does not parse f64 as f32"],
    ["prop_f64_pos_inf", "does not parse f64 as f32"],
    ["prop_f64_zero", "does not parse f64 as f32"],
    ["prop_f64_null_val", "does not parse f64 as f32"],
    ["prop_f64_val_null", "does not parse f64 as f32"],
]);

class SyntheticTestRunnerJS extends SyntheticTestRunner {
    shouldSkip(testName: string) {
        return UNIMPLEMENTED_SYNTHETICS.get(testName) ?? false;
    }

    async decodeMLT(mltFilePath: string) {
        const mltBuffer = await readFile(mltFilePath);
        const featureTables = decodeTile(mltBuffer, undefined, false);
        return featureTablesToFeatureCollection(featureTables) as unknown as Record<string, unknown>;
    }
}

describe("MLT Decoder - Synthetic tests", async () => {
    const runner = new SyntheticTestRunnerJS();

    for await (const result of runner) {
        switch (result.status) {
            case "skip":
                it.skip(`${result.name} (${result.reason})`, () => undefined);
                break;
            case "ok":
                it(result.name, () => undefined);
                break;
            case "fail":
                it(result.name, () => {
                    throw result.error;
                });
                break;
            case "crash":
                it(result.name, () => {
                    throw new Error(`Crash for ${result.name}: ${result.reason}`);
                });
                break;
        }
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
