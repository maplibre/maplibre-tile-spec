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
    ["polygon_fpf_tes", "FastPFor not implemented"],
    ["polygon_hole_fpf", "FastPFor not implemented"],
    ["polygon_fpf", "FastPFor not implemented"],
    ["props_mixed", "F32 very buggy and does not work"],
    ["prop_str_empty", "empty strings not supported"],
    ["prop_i64_min", "wraps to 0 despite it should have been negative"],
    ["prop_u32_max", "wraps to -1 despite it should have been positive"],
    ["prop_i64_max", "wraps to -1 despite it should have been positive"],
    ["prop_u64_max", "wraps to -1 despite it should have been positive"],
    ["mix_2_line_mpoly", "geometry diff"],
    ["mix_2_mline_mpoly", "geometry diff"],
    ["mix_2_mpoly_mpoly", "geometry diff"],
    ["mix_2_mpt_mline", "geometry diff"],
    ["mix_2_mpt_mpoly", "geometry diff"],
    ["mix_2_poly_mpoly", "geometry diff"],
    ["mix_2_polyh_mpoly", "geometry diff"],
    ["mix_2_pt_mpoly", "geometry diff"],
    ["mix_3_line_mline_mpoly", "geometry diff"],
    ["mix_3_line_mpoly_line", "geometry diff"],
    ["mix_3_line_mpt_line", "geometry diff"],
    ["mix_3_line_mpt_mline", "geometry diff"],
    ["mix_3_line_mpt_mpoly", "geometry diff"],
    ["mix_3_line_poly_mpoly", "geometry diff"],
    ["mix_3_line_polyh_mpoly", "geometry diff"],
    ["mix_3_mline_mpoly_mline", "geometry diff"],
    ["mix_3_mline_mpt_mline", "geometry diff"],
    ["mix_3_mpoly_line_mpoly", "geometry diff"],
    ["mix_3_mpoly_mline_mpoly", "geometry diff"],
    ["mix_3_mpoly_mpt_mpoly", "geometry diff"],
    ["mix_3_mpoly_poly_mpoly", "geometry diff"],
    ["mix_3_mpoly_polyh_mpoly", "geometry diff"],
    ["mix_3_mpoly_pt_mpoly", "geometry diff"],
    ["mix_3_mpt_line_mpt", "geometry diff"],
    ["mix_3_mpt_mline_mpt", "geometry diff"],
    ["mix_3_mpt_mpoly_mpt", "geometry diff"],
    ["mix_3_mpt_poly_mpt", "geometry diff"],
    ["mix_3_mpt_polyh_mpt", "geometry diff"],
    ["mix_3_mpt_mline_mpoly", "geometry diff"],
    ["mix_3_poly_mline_mpoly", "geometry diff"],
    ["mix_3_poly_mpoly_poly", "geometry diff"],
    ["mix_3_poly_mpt_poly", "geometry diff"],
    ["mix_3_poly_mpt_mline", "geometry diff"],
    ["mix_3_poly_mpt_mpoly", "geometry diff"],
    ["mix_3_poly_polyh_mpoly", "geometry diff"],
    ["mix_3_polyh_mline_mpoly", "geometry diff"],
    ["mix_3_polyh_mpt_mline", "geometry diff"],
    ["mix_3_polyh_mpt_mpoly", "geometry diff"],
    ["mix_3_polyh_mpt_polyh", "geometry diff"],
    ["mix_3_polyh_mpoly_polyh", "geometry diff"],
    ["should decode mix_3_polyh_mpt_polyh", "geometry diff"],
    ["mix_3_pt_line_mpoly", "geometry diff"],
    ["mix_3_pt_mline_mpoly", "geometry diff"],
    ["mix_3_pt_mpt_mline", "geometry diff"],
    ["mix_3_pt_mpoly_pt", "geometry diff"],
    ["mix_3_pt_mpt_mpoly", "geometry diff"],
    ["mix_3_pt_poly_mpoly", "geometry diff"],
    ["mix_3_pt_polyh_mpoly", "geometry diff"],
    ["mix_4_line_mpt_mline_mpoly", "geometry diff"],
    ["mix_4_line_poly_mline_mpoly", "geometry diff"],
    ["mix_4_line_poly_mpt_mline", "geometry diff"],
    ["mix_4_line_poly_mpt_mpoly", "geometry diff"],
    ["mix_4_line_poly_polyh_mpoly", "geometry diff"],
    ["mix_4_line_polyh_mline_mpoly", "geometry diff"],
    ["mix_4_line_polyh_mpt_mline", "geometry diff"],
    ["mix_4_line_polyh_mpt_mpoly", "geometry diff"],
    ["mix_4_poly_mpt_mline_mpoly", "geometry diff"],
    ["mix_4_poly_polyh_mline_mpoly", "geometry diff"],
    ["mix_4_poly_polyh_mpt_mline", "geometry diff"],
    ["mix_4_poly_polyh_mpt_mpoly", "geometry diff"],
    ["mix_4_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_4_pt_line_mline_mpoly", "geometry diff"],
    ["mix_4_pt_line_mpt_mline", "geometry diff"],
    ["mix_4_pt_line_mpt_mpoly", "geometry diff"],
    ["mix_4_pt_line_poly_mpoly", "geometry diff"],
    ["mix_4_pt_line_polyh_mpoly", "geometry diff"],
    ["mix_4_pt_mpt_mline_mpoly", "geometry diff"],
    ["mix_4_pt_poly_mline_mpoly", "geometry diff"],
    ["mix_4_pt_poly_mpt_mline", "geometry diff"],
    ["mix_4_pt_poly_mpt_mpoly", "geometry diff"],
    ["mix_4_pt_poly_polyh_mpoly", "geometry diff"],
    ["mix_4_pt_polyh_mline_mpoly", "geometry diff"],
    ["mix_4_pt_polyh_mpt_mline", "geometry diff"],
    ["mix_4_pt_polyh_mpt_mpoly", "geometry diff"],
    ["mix_5_line_poly_mpt_mline_mpoly", "geometry diff"],
    ["mix_5_line_poly_polyh_mline_mpoly", "geometry diff"],
    ["mix_5_line_poly_polyh_mpt_mline", "geometry diff"],
    ["mix_5_line_poly_polyh_mpt_mpoly", "geometry diff"],
    ["mix_5_line_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_5_poly_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_5_pt_line_mpt_mline_mpoly", "geometry diff"],
    ["mix_5_pt_line_poly_mline_mpoly", "geometry diff"],
    ["mix_5_pt_line_poly_mpt_mline", "geometry diff"],
    ["mix_5_pt_line_poly_mpt_mpoly", "geometry diff"],
    ["mix_5_pt_line_poly_polyh_mpoly", "geometry diff"],
    ["mix_5_pt_line_polyh_mline_mpoly", "geometry diff"],
    ["mix_5_pt_line_polyh_mpt_mline", "geometry diff"],
    ["mix_5_pt_line_polyh_mpt_mpoly", "geometry diff"],
    ["mix_5_pt_poly_mpt_mline_mpoly", "geometry diff"],
    ["mix_5_pt_poly_polyh_mline_mpoly", "geometry diff"],
    ["mix_5_pt_poly_polyh_mpt_mline", "geometry diff"],
    ["mix_5_pt_poly_polyh_mpt_mpoly", "geometry diff"],
    ["mix_5_pt_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_6_line_poly_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_6_pt_line_poly_mpt_mline_mpoly", "geometry diff"],
    ["mix_6_pt_line_poly_polyh_mline_mpoly", "geometry diff"],
    ["mix_6_pt_line_poly_polyh_mpt_mline", "geometry diff"],
    ["mix_6_pt_line_poly_polyh_mpt_mpoly", "geometry diff"],
    ["mix_6_pt_line_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_6_pt_poly_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_7_pt_line_poly_polyh_mpt_mline_mpoly", "geometry diff"],
    ["mix_dup_mpoly", "geometry diff"],
    ["mix_line_mpoly_line", "geometry diff"],
    ["mix_line_mpt_line", "geometry diff"],
    ["mix_mline_mpoly_mline", "geometry diff"],
    ["mix_mline_mpt_mline", "geometry diff"],
    ["mix_mpoly_line_mpoly", "geometry diff"],
    ["mix_mpoly_mline_mpoly", "geometry diff"],
    ["mix_mpoly_mpt_mpoly", "geometry diff"],
    ["mix_mpoly_poly_mpoly", "geometry diff"],
    ["mix_mpoly_polyh_mpoly", "geometry diff"],
    ["mix_mpoly_pt_mpoly", "geometry diff"],
    ["mix_mpt_line_mpt", "geometry diff"],
    ["mix_mpt_mline_mpt", "geometry diff"],
    ["mix_mpt_mpoly_mpt", "geometry diff"],
    ["mix_mpt_poly_mpt", "geometry diff"],
    ["mix_mpt_polyh_mpt", "geometry diff"],
    ["mix_poly_mpoly_poly", "geometry diff"],
    ["mix_poly_mpt_poly", "geometry diff"],
    ["mix_polyh_mpoly_polyh", "geometry diff"],
    ["mix_polyh_mpt_polyh", "geometry diff"],
    ["mix_pt_mpoly_pt", "geometry diff"],
    ["prop_f32_nan", "geometry diff"],
    ["prop_f32_neg_inf", "geometry diff"],
    ["prop_f32_pos_inf", "geometry diff"],
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
