import { expect, describe, it } from "vitest";
import { readdirSync, readFileSync } from "fs";
import { parse, join } from "path";
import { VectorTile, type VectorTileFeature } from "@mapbox/vector-tile";
import Pbf from "pbf";

import { type FeatureTable, type Feature, decodeTile } from ".";
import path from "node:path";
import fs from "node:fs";

const ITERATOR_TILE = path.resolve(__dirname, "../../test/expected/tag0x01/simple/multiline-boolean.mlt");

describe("MLT Decoder - MVT comparison for SIMPLE tiles", () => {
    const simpleMltTileDir = "../test/expected/tag0x01/simple";
    const simpleMvtTileDir = "../test/fixtures/simple";
    testTiles(simpleMltTileDir, simpleMvtTileDir);
});

describe("MLT Decoder - MVT comparison for Amazon tiles", () => {
    const amazonMltTileDir = "../test/expected/tag0x01/amazon";
    const amazonMvtTileDir = "../test/fixtures/amazon";
    testTiles(amazonMltTileDir, amazonMvtTileDir);
});

describe("MLT Decoder - MVT comparison for OMT tiles", () => {
    const omtMltTileDir = "../test/expected/tag0x01/omt";
    const omtMvtTileDir = "../test/fixtures/omt";
    testTiles(omtMltTileDir, omtMvtTileDir);
}, 150000);

describe("FeatureTable", () => {
    it("should iterate through features correctly", () => {
        const bytes = new Uint8Array(fs.readFileSync(ITERATOR_TILE));
        const featureTables = decodeTile(bytes);

        const table = featureTables[0];

        expect(table.name).toBe("layer");
        expect(table.extent).toBe(4096);

        let featureCount = 0;
        for (const feature of table) {
            expect(feature.geometry).toBeTruthy();
            expect(feature.geometry.coordinates).toBeInstanceOf(Array);
            expect(feature.geometry.coordinates.length).toBeGreaterThan(0);
            expect(typeof feature.geometry.type).toBe("number");

            featureCount++;
        }
        expect(featureCount).toBe(table.numFeatures);
    });
});

function testTiles(mltSearchDir: string, mvtSearchDir: string) {
    const mltFileNames = readdirSync(mltSearchDir)
        .filter((file) => parse(file).ext === ".mlt")
        .map((file) => parse(file).name);
    for (const fileName of mltFileNames) {
        it(`should compare ${fileName} tile`, () => {
            const mltFileName = `${fileName}.mlt`;
            const mltPath = join(mltSearchDir, mltFileName);
            const mvtPath = join(mvtSearchDir, `${fileName}.mvt`);

            const encodedMvt = readFileSync(mvtPath);
            const encodedMlt = readFileSync(mltPath);
            const buf = new Pbf(encodedMvt);
            const decodedMvt = new VectorTile(buf);

            const decodedMlt = decodeTile(encodedMlt, undefined, true);
            comparePlainGeometryEncodedTile(decodedMlt, decodedMvt);
        });
    }
}

function removeEmptyStrings(mvtProperties: Record<string, any>) {
    for (const key of Object.keys(mvtProperties)) {
        const value = mvtProperties[key];
        if (typeof value === "string" && !value.length) {
            delete mvtProperties[key];
        }
    }
}

function comparePlainGeometryEncodedTile(mlt: FeatureTable[], mvt: VectorTile) {
    for (const featureTable of mlt) {
        const layer = mvt.layers[featureTable.name];

        // Use getFeatures() instead of iterator (like C++ and Java implementations)
        const mltFeatures = featureTable.getFeatures();

        expect(mltFeatures.length).toBe(layer.length);

        for (let j = 0; j < layer.length; j++) {
            const mvtFeature = layer.feature(j);
            const mltFeature = mltFeatures[j];

            compareId(mltFeature, mvtFeature, true);

            const mltGeometry = mltFeature.geometry?.coordinates;
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

function compareId(mltFeature: Feature, mvtFeature: VectorTileFeature, idWithinMaxSafeInteger: boolean) {
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
         * - The java-vector-tile library used in the Java converter returns negative integers for the
         *   unoptimized tileset in some tiles
         * - The vector-tile-js library is using number types for the id so there can only be stored
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

/* Change bigint to number for comparison with MVT */
function convertBigIntPropertyValues(mltProperties: Record<string, any>) {
    for (const key of Object.keys(mltProperties)) {
        if (typeof mltProperties[key] === "bigint") {
            mltProperties[key] = Number(mltProperties[key]);
        }
    }
}

function transformPropertyNames(properties: Record<string, any>) {
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
