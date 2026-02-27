import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, it } from "vitest";
import decodeTile from "./mltDecoder";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import type { Geometry } from "./vector/geometry/geometryVector";
import type FeatureTable from "./vector/featureTable";

import { SyntheticTestRunner } from "synthetic-test-tool";

const UNIMPLEMENTED_SYNTHETICS = new Map([
    ["polygon_multi_fpf", "FastPFor not implemented"],
    ["polygon_fpf_tes", "Morton not implemented"],
    ["polygon_hole_fpf", "FastPFor not implemented"],
    ["polygon_fpf", "FastPFor not implemented"],
    ["props_mixed", "F32 very buggy and does not work"],
    ["prop_str_empty", "empty strings not supported"],
    ["prop_i64_min", "wraps to 0 despite it should have been negative"],
    ["prop_u32_max", "wraps to -1 despite it should have been positive"],
    ["prop_i64_max", "wraps to -1 despite it should have been positive"],
    ["prop_u64_max", "wraps to -1 despite it should have been positive"],
    ["mixed_all", "geometry diff"],
    ["mixed_line_mpt_line", "geometry diff"],
    ["mixed_mline_mpt_mline", "geometry diff"],
    ["mixed_mpoly_mpt_mpoly", "geometry diff"],
    ["mixed_mpt_line", "geometry diff"],
    ["mixed_mpt_line_mpt", "geometry diff"],
    ["mixed_mpt_mline", "geometry diff"],
    ["mixed_mpt_mline_mpt", "geometry diff"],
    ["mixed_mpt_mpoly", "geometry diff"],
    ["mixed_mpt_mpoly_mpt", "geometry diff"],
    ["mixed_mpt_poly", "geometry diff"],
    ["mixed_mpt_poly_mpt", "geometry diff"],
    ["mixed_poly_mpt_poly", "geometry diff"],
]);

const syntheticDir = resolve(__dirname, "../../test/synthetic/0x01");

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

describe("MLT Decoder - Synthetic tests", () => {
    it("should pass all synthetic tests", async () => {
        await new SyntheticTestRunnerJS().run(syntheticDir);
    });
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
            const id = safeNumber(feature.id);
            if (id != null) {
                geojsonFeature.id = id;
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
