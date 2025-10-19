import { expect, describe, it } from "vitest";
import { readdirSync, readFileSync } from "fs";
import { parse, join } from "path";
import { classifyRings } from "@maplibre/maplibre-gl-style-spec";
import { VectorTile } from "@mapbox/vector-tile";
import Pbf from "pbf";
import earcut from "earcut";

import { type FeatureTable, decodeTile } from ".";

describe("MLT Decoder - MVT comparison for OMT tiles", () => {
    const omtMltTileDir = "../test/expected/tag0x01/omt";
    const omtMvtTileDir = "../test/fixtures/omt";
    testTiles(omtMltTileDir, omtMvtTileDir);
});

function testTiles(mltSearchDir: string, mvtSearchDir: string, isSorted = false, idWithinMaxSafeInteger = true) {
    let mltFileNames = readdirSync(mltSearchDir)
        .filter((file) => parse(file).ext === ".mlt")
        .map((file) => parse(file).name);
    mltFileNames = [mltFileNames[0]]; // TODO: remove this!
    for (const fileName of mltFileNames) {
        it.skip(`should compare ${fileName} tile`, () => {
            const mltFileName = `${fileName}.mlt`;
            const mltPath = join(mltSearchDir, mltFileName);
            const mvtPath = join(mvtSearchDir, `${fileName}.mvt`);

            const encodedMvt = readFileSync(mvtPath);
            const encodedMlt = readFileSync(mltPath);
            const buf = new Pbf(encodedMvt);
            const decodedMvt = new VectorTile(buf);

            const decodedMlt = decodeTile(encodedMlt, undefined, idWithinMaxSafeInteger);
            comparePlainGeometryEncodedTile(decodedMlt, decodedMvt, isSorted, idWithinMaxSafeInteger);
        });
    }
}

function removeEmptyStrings(mvtProperties) {
    for (const key of Object.keys(mvtProperties)) {
        const value = mvtProperties[key];
        if (typeof value === "string" && !value.length) {
            delete mvtProperties[key];
        }
    }
}

function comparePlainGeometryEncodedTile(
    mlt: FeatureTable[],
    mvt: VectorTile,
    isSorted = false,
    idWithinMaxSafeInteger = true,
) {
    for (const featureTable of mlt) {
        const layer = mvt.layers[featureTable.name];

        let j = 0;
        for (const mltFeature of featureTable) {
            const mvtFeature = isSorted ? getMvtFeatureById(layer, mltFeature.id) : layer.feature(j++);

            compareId(mltFeature, mvtFeature, idWithinMaxSafeInteger);

            const mltGeometry = mltFeature.geometry;
            const mvtGeometry = mvtFeature.loadGeometry();
            expect(mltGeometry).toEqual(mvtGeometry);

            const mltProperties = mltFeature.properties;
            const mvtProperties = mvtFeature.properties;
            transformPropertyNames(mltProperties);
            transformPropertyNames(mvtProperties);
            convertBigIntPropertyValues(mltProperties);
            //TODO: fix -> since a change in the java converter shared dictionary encoding empty strings are not
            //encoded anymore
            removeEmptyStrings(mvtProperties);
            removeEmptyStrings(mltProperties);

            expect(Object.keys(mltProperties).length).toEqual(Object.keys(mvtProperties).length);
            expect(mltProperties).toEqual(mvtProperties);
        }
    }
}

function compareId(mltFeature, mvtFeature, idWithinMaxSafeInteger) {
    if (!mvtFeature.id) {
        /* Java MVT library in the MVT converter decodes zero for undefined ids */
        expect([0, null, 0n]).toContain(mltFeature.id);
    } else {
        const mltFeatureId = mltFeature.id;
        /* For const and sequence vectors the decoder can return bigint compared to the vector-tile-js library */
        const actualId =
            idWithinMaxSafeInteger && typeof mltFeatureId !== "bigint" ? mltFeatureId : Number(mltFeatureId);
        /*
         * The id check can fail for two known reasons:
         * - The java-vector-tile library used in the Java converter returns negative integers  for the
         *   unoptimized tileset in some tiles
         * - The vector-tile-js is library is using number types for the id so there can only be stored
         *   values up to 53 bits without loss of precision
         **/
        if (mltFeatureId < 0 || mltFeatureId > Number.MAX_SAFE_INTEGER) {
            /* Expected to fail in some/most cases */
            try {
                expect(actualId).toEqual(mvtFeature.id);
            } catch (e) {
                //console.info("id mismatch", featureTableName, mltFeatureId, mvtFeature.id);
            }
            return;
        }

        expect(actualId).toEqual(mvtFeature.id);
    }
}

function getMvtFeatureById(layer, id) {
    for (let i = 0; i < layer.length; i++) {
        const mvtFeature = layer.feature(i);
        if (mvtFeature.id === id || mvtFeature.properties["id"] === id) {
            return mvtFeature;
        }
    }
    return null;
}

/* Change bigint to number for comparison with MVT */
function convertBigIntPropertyValues(mltProperties) {
    for (const key of Object.keys(mltProperties)) {
        if (typeof mltProperties[key] === "bigint") {
            mltProperties[key] = Number(mltProperties[key]);
        }
    }
}

function transformPropertyNames(properties: { [p: string]: any }) {
    const propertyNames = Object.keys(properties);
    for (let k = 0; k < propertyNames.length; k++) {
        const key = propertyNames[k];

        let newKey = key;
        /* rename the property names which are separated with : in mlt to match _ in omt mvts */
        if (key.startsWith("name") && key.includes(":")) {
            newKey = (key as any).replaceAll(":", "_");
            properties[newKey] = properties[key];
            delete properties[key];
        }

        /* Currently id is not supported as a property name in a FeatureTable,
         *  so this quick workaround is implemented */
        if (newKey === "_id") {
            properties["id"] = properties[newKey];
            delete properties[newKey];
        }
    }
}

/* Polygon tessellation code from the FillBucket class of MapLibre GL JS
 *  Slightly modified to tessellate without the closing point to match the MLT approach
 * */
function tessellatePolygon(geometry: Array<Array<{ x: number; y: number }>>) {
    const EARCUT_MAX_RINGS = 500;
    const polygonIndices = [];
    const vertexBuffer = [];
    for (const polygon of classifyRings(geometry, EARCUT_MAX_RINGS)) {
        const flattened = [];
        const holeIndices = [];
        for (const ring of polygon) {
            if (ring.length === 0) {
                continue;
            }

            if (ring !== polygon[0]) {
                holeIndices.push(flattened.length / 2);
            }

            flattened.push(ring[0].x);
            flattened.push(ring[0].y);
            for (let i = 1; i < ring.length - 1; i++) {
                flattened.push(ring[i].x);
                flattened.push(ring[i].y);
            }
        }

        const indices = earcut(flattened, holeIndices);
        polygonIndices.push(...indices);
        vertexBuffer.push(...flattened);
    }

    return { indices: polygonIndices, vertexBuffer };
}
