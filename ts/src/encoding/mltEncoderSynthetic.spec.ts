import { readFile } from "node:fs/promises";
import * as path from "node:path";
import { describe, expect, it } from "vitest";
import { compareWithTolerance, getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import decodeTile from "../mltDecoder";
import { featureTablesToFeatureCollection, getGeometry } from "../vector/featureTablesToGeoJson";
import type FeatureTable from "../vector/featureTable";
import { encodeTile, type Feature, type FeatureGeometry, type Layer, type PropertyValue } from "./mltEncoder";

/**
 * Fixtures that cannot survive this round trip. Each is checked below to still *fail*, so an entry
 * that starts working fails the suite until it is removed from this list.
 */
/**
 * Fixtures that cannot survive this round trip because `encodeTile` emits Tag 0x01 layers,
 * which have no wire representation for v2 `Meshes`, per-vertex/per-triangle `m-values`, or 3D `Z`.
 * Each is checked below to still *fail*, so an entry that starts working fails the suite until
 * it is removed from this list.
 */
const UNSUPPORTED: string[] = [
    // v2 3D Meshes
    "0x02/mesh_mvalues_both",
    "0x02/mesh_mvalues_triangle",
    "0x02/mesh_mvalues_vertex",
    "0x02/mesh_relative_to_ground",
    "0x02/mesh_roof_wall",
    "0x02/mesh_triangle",
    // v2 M-value columns
    "0x02/mvalues",
    "0x02/mvalues_15props_7cols",
    "0x02/mvalues_16props",
    "0x02/mvalues_8cols",
    "0x02/mvalues_bool",
    "0x02/mvalues_delta_rle",
    "0x02/mvalues_f32",
    "0x02/mvalues_f32_dict",
    "0x02/mvalues_f64",
    "0x02/mvalues_f64_alp",
    "0x02/mvalues_fpf",
    "0x02/mvalues_i32",
    "0x02/mvalues_i64",
    "0x02/mvalues_i8",
    "0x02/mvalues_line_dict",
    "0x02/mvalues_mline",
    "0x02/mvalues_mline_dict",
    "0x02/mvalues_mpoint",
    "0x02/mvalues_mpoint_dict",
    "0x02/mvalues_mpoly",
    "0x02/mvalues_mpoly_dict",
    "0x02/mvalues_plain",
    "0x02/mvalues_poly",
    "0x02/mvalues_poly_dict",
    "0x02/mvalues_poly_hole",
    "0x02/mvalues_rle",
    "0x02/mvalues_sp",
    "0x02/mvalues_sp_prop",
    "0x02/mvalues_str_dict",
    "0x02/mvalues_str_fsst",
    "0x02/mvalues_str_plain",
    "0x02/mvalues_tes",
    "0x02/mvalues_u32",
    "0x02/mvalues_u64",
    "0x02/mvalues_u8",
    // v2 3D Z coordinates
    "0x02/z_height_absolute",
    "0x02/z_height_relative_to_ground",
    "0x02/z_line",
    "0x02/z_line_fpf",
    "0x02/z_mix",
    "0x02/z_mvalues",
    "0x02/z_point",
    "0x02/z_points",
    "0x02/z_points_hilbert",
    "0x02/z_poly_hole",
    "0x02/z_poly_hole_tes",
    "0x02/z_poly_hole_tri",
    "0x02/z_step_coarsest",
    "0x02/z_step_finest",
];

/**
 * Decodes each synthetic `.mlt`, re-encodes what came out, and checks the result still decodes to
 * the fixture's expected GeoJSON.
 *
 * The reference tile is the input rather than the expected JSON, so values reach the encoder with
 * the types they really have - 64-bit ids stay BigInt instead of being rounded through a JSON
 * double. Only the final comparison goes via GeoJSON. The byte layout is free to differ from the
 * reference: this encoder writes plain streams where the reference uses dictionaries, FastPFOR and
 * the like.
 */
describe("encodeTile - synthetic fixtures round trip", () => {
    expect.addEqualityTesters([compareWithTolerance]);
    const { active, skipped } = getTestCases(UNSUPPORTED.map((entry) => entry.replaceAll("/", path.sep)));

    for (const { name, content, fileName } of active) {
        it(name, async () => {
            const actual = await reEncode(fileName);
            expect(actual).toEqual(normalise(content as GeoJSON.FeatureCollection));
        });
    }

    for (const { name, content, fileName } of skipped) {
        it(`${name} (unsupported)`, async () => {
            let actual: GeoJSON.FeatureCollection | undefined;
            try {
                actual = await reEncode(fileName);
            } catch {
                return;
            }
            expect(actual, "round-tripped cleanly - remove it from the exclusion list").not.toEqual(
                normalise(content as GeoJSON.FeatureCollection),
            );
        });
    }
});

/** Reads a reference tile, re-encodes what it decodes to, and decodes that back to GeoJSON. */
async function reEncode(mltFile: string): Promise<GeoJSON.FeatureCollection> {
    const reference = decodeTile(new Uint8Array(await readFile(mltFile)), undefined, false);
    return featureTablesToFeatureCollection(decodeTile(encodeTile(toLayers(reference)), undefined, false));
}

/** Turns decoded feature tables back into the layer shape the encoder takes. */
function toLayers(featureTables: FeatureTable[]): Layer[] {
    return featureTables.map((table) => ({
        name: table.name,
        extent: table.extent,
        features: table.getFeatures().map((feature) => {
            const encoded: Feature = {
                geometry: getGeometry(feature.geometry) as FeatureGeometry,
                properties: feature.properties as Record<string, PropertyValue>,
            };
            if (feature.id !== undefined && feature.id !== null) encoded.id = feature.id;
            return encoded;
        }),
    }));
}

/** Property order and key order are not preserved by encoding, so compare a canonical form. */
function normalise(collection: GeoJSON.FeatureCollection): GeoJSON.FeatureCollection {
    return {
        type: "FeatureCollection",
        features: collection.features.map((feature) => {
            const normalised: GeoJSON.Feature = {
                type: "Feature",
                geometry: feature.geometry,
                properties: Object.fromEntries(
                    Object.entries(feature.properties ?? {}).filter(([, value]) => value !== null),
                ),
            };
            if (feature.id !== undefined) normalised.id = feature.id;
            return normalised;
        }),
    };
}
