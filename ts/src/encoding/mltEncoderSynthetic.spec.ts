import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { compareWithTolerance, getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import decodeTile from "../mltDecoder";
import { GEOMETRY_TYPE } from "../vector/geometry/geometryType";
import { classifyRings } from "../vector/geometry/classifyRings";
import type { Geometry } from "../vector/geometry/geometryVector";
import type FeatureTable from "../vector/featureTable";
import { encodeTile, type Feature, type FeatureGeometry, type Layer, type PropertyValue } from "./mltEncoder";

/**
 * Fixtures that cannot survive this round trip. Each is checked below to still *fail*, so an entry
 * that starts working fails the suite until it is removed from this list.
 */
const UNSUPPORTED: string[] = [
    // Nested (MAP) property columns, which neither this encoder nor the decoder supports yet.
    "0x02/prop_nested_big",
    "0x02/prop_nested_ints",
    "0x02/prop_nested_json",
    "0x02/prop_nested_list",
    "0x02/prop_nested_list_root",
    "0x02/prop_nested_mixed_root",
    "0x02/prop_nested_null",
    "0x02/prop_nested_shared",
    "0x02/prop_nested_specials",
];

/**
 * Decodes each synthetic `.mlt`, re-encodes what came out, and checks the result still decodes to
 * the fixture's expected GeoJSON.
 *
 * The reference tile is the input rather than the expected JSON, so values reach the encoder with
 * the types they really have — 64-bit ids stay BigInt instead of being rounded through a JSON
 * double. Only the final comparison goes via GeoJSON. The byte layout is free to differ from the
 * reference: this encoder writes plain streams where the reference uses dictionaries, FastPFOR and
 * the like.
 */
describe("encodeTile - synthetic fixtures round trip", () => {
    expect.addEqualityTesters([compareWithTolerance]);
    const { active, skipped } = getTestCases(UNSUPPORTED);

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
            expect(actual, "round-tripped cleanly — remove it from the exclusion list").not.toEqual(
                normalise(content as GeoJSON.FeatureCollection),
            );
        });
    }
});

/** Reads a reference tile, re-encodes what it decodes to, and decodes that back to GeoJSON. */
async function reEncode(mltFile: string): Promise<GeoJSON.FeatureCollection> {
    const reference = decodeTile(new Uint8Array(await readFile(mltFile)), undefined, false);
    return toFeatureCollection(decodeTile(encodeTile(toLayers(reference)), undefined, false));
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

/** The same conversion the decoder's synthetic harness uses, so both sides are comparable. */
function toFeatureCollection(featureTables: FeatureTable[]): GeoJSON.FeatureCollection {
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
            if (id !== null && id !== undefined) geojsonFeature.id = id;
            features.push(geojsonFeature);
        }
    }
    return { type: "FeatureCollection", features };
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
