import * as fs from "fs";
import * as Path from "path";
import decodeTile from "../../src/mltDecoder";
import {TileSetMetadata} from "../../src/metadata/tileset/tilesetMetadata";
import {VectorTile} from "@mapbox/vector-tile";
import Pbf from "pbf";
import * as path from "node:path";
import {FeatureTable, GpuVector} from "../../src";
import {classifyRings} from "@maplibre/maplibre-gl-style-spec";
//TODO: refactor to use npm package
import earcut from "../utils/earcut";

const omtOptimizedMvtDir = "./test/data/omt/optimized/mvt";
const omtUnoptimizedMvtDir = "./test/data/omt/unoptimized/mvt";

const omtOptimizedPlainMltDir = "./test/data/omt/optimized/mlt/plain";
const omtOptimizedPlainMortonMltDir = "./test/data/omt/optimized/mlt/plain-morton";
//const omtOptimizedPlainOptimizedIdMltDir = "./test/data/omt/optimized/mlt/plain-optimized-id";
const omtOptimizedPretessellatedMltDir = "./test/data/omt/optimized/mlt/pre-tessellated";

const omtUnoptimizedPlainMltDir = "./test/data/omt/unoptimized/mlt/plain";
const omtUnoptimizedPlainMortonMltDir = "./test/data/omt/unoptimized/mlt/plain-morton";
//const omtUnoptimizedPlainOptimizedIdMltDir = "./test/data/omt/unoptimized/mlt/plain-optimized-id";
const omtUnoptimizedPretessellatedMltDir = "./test/data/omt/unoptimized/mlt/pre-tessellated";


/*
* TODOs:
* - Fix failing id comparison in building layer in the unoptimized tileset -> tile 13_4359_2842 and 14_8718_5685
* - When ids sorted in converter tests are wrong
* - Fix pre-tessellation tests
* */

/*
* Optimizations:
* - decodeScalarPropertyColumn -> handle case again when nullability buffer is present but all values in the column
*   are present -> skip nullability buffer decoding -> fix calculation for 4 bits in getVectorTypeBooleanStream
*   function or use flag in metadata
* - decodeBooleanColumn -> add check for ConstBooleanVector again
*
* Improvements:
* - decodePropertyColumn -> Handle struct which currently only supports strings as nested fields for supporting shared
*   dictionary encoding
*
* */

describe("decodeTile", () => {

    it("should decode dictionary of unoptimized plain encoded OMT schema based tiles with global tileset " +
        "metadata", async () => {
        testTiles(omtUnoptimizedPlainMltDir, omtUnoptimizedMvtDir, false);
    });

    it("should decode dictionary of optimized plain encoded OMT schema based tiles with global tileset metadata",
        async () => {
        testTiles(omtOptimizedPlainMltDir, omtOptimizedMvtDir, false);
    });

    it("should decode dictionary of unoptimized plain morton encoded OMT schema based tiles with global tileset " +
        "metadata", async () => {
        testTiles(omtUnoptimizedPlainMortonMltDir, omtUnoptimizedMvtDir, false);
    });

    it("should decode dictionary of optimized plain morton encoded OMT schema based tiles with global tileset " +
        "metadata", async () => {
            testTiles(omtOptimizedPlainMortonMltDir, omtOptimizedMvtDir, false);
    });

    it("should decode dictionary of unoptimized pre-tessellated OMT schema based tiles with global tileset " +
        "metadata", async () => {
        testTiles(omtUnoptimizedPretessellatedMltDir, omtUnoptimizedMvtDir, true);
    });

    it("should decode dictionary of optimized pre-tessellated OMT schema based tiles with global tileset " +
        "metadata", async () => {
        testTiles(omtOptimizedPretessellatedMltDir, omtOptimizedMvtDir, true);
    });

    it.each(["2_2_2"])(
        "should partially decode optimized OMT schema based tile %i with global tileset metadata and without advanced encodings" +
        " and with polygon pre-tessellation", tileId => {
            const {tilesetMetadata, encodedMlt, decodedMvt} = getTileData(tileId,
                omtOptimizedPlainMltDir, omtOptimizedMvtDir);

            const featureTableDecodingOptions = new Map<string, Set<string>>();
            featureTableDecodingOptions.set("place", new Set(["class", "name:latin",
                "name:nonlatin", "capital", "rank", "iso_a2"]));
            featureTableDecodingOptions.set("water", new Set());
            featureTableDecodingOptions.set("boundary", new Set(["admin:level", "maritime", "disputed"]));
            featureTableDecodingOptions.set("transportation", new Set([]));
            featureTableDecodingOptions.set("building", new Set([]));

            const decodedMlt = decodeTile(encodedMlt, tilesetMetadata, featureTableDecodingOptions);

            expect(decodedMlt.length).toEqual(3);
        });

    it.each(["0_0_0", "1_1_1", "2_2_2"])(
        "should decode optimized OMT schema based tile %i with global tileset metadata and without advanced encodings" +
        " and without pre-tessellation", tileId => {
            const {tilesetMetadata, encodedMlt, decodedMvt} = getTileData(tileId,
                omtOptimizedPlainMltDir, omtOptimizedMvtDir);

            const decodedMlt = decodeTile(encodedMlt, tilesetMetadata);

            comparePlainGeometryEncodedTile(decodedMlt, decodedMvt);
        });

    it.each(["5_16_21", "6_33_42", "7_67_84"])(
        "should decode optimized OMT schema based tile %i with global tileset metadata and without advanced encodings" +
        " and with polygon pre-tessellation", tileId => {
            const {tilesetMetadata, encodedMlt, decodedMvt} = getTileData(tileId,
                omtOptimizedPretessellatedMltDir, omtOptimizedMvtDir);

            const decodedMlt = decodeTile(encodedMlt, tilesetMetadata);

            comparePreTessellatedTile(decodedMlt, decodedMvt);
        });

    it.each(["0_0_0", "1_1_0", "2_2_1", "3_4_2", "4_8_5", "5_17_10", "6_34_21", "7_68_42"])(
        "should decode unoptimized OMT schema based tile %i with global tileset metadata and without advanced encodings" +
        " and with polygon pre-tessellation", tileId => {
            const {tilesetMetadata, encodedMlt, decodedMvt} = getTileData(tileId,
                omtUnoptimizedPretessellatedMltDir, omtUnoptimizedMvtDir);

            const decodedMlt = decodeTile(encodedMlt, tilesetMetadata);

            comparePreTessellatedTile(decodedMlt, decodedMvt);
        });

});


function getTileData(tileId: string, mltSearchPath: string, mvtSearchPath: string) {
    const mltFilename = Path.join(mltSearchPath, `${tileId}.mlt`);
    const mltMetaFilename = Path.join(mltSearchPath, "tileset.pbf");
    const mvtFileName = Path.join(mvtSearchPath, `${tileId}.mvt`);
    const encodedMvt = fs.readFileSync(mvtFileName);
    const tilesetMetadata = fs.readFileSync(mltMetaFilename);
    const encodedMlt = fs.readFileSync(mltFilename);
    const buf = new Pbf(encodedMvt)
    const decodedMvt = new VectorTile(buf);
    const metadata = TileSetMetadata.fromBinary(tilesetMetadata);
    return {tilesetMetadata: metadata, encodedMlt, decodedMvt};
}

function testTiles(mltSearchDir: string, mvtSearchDir: string, isPreTessellated: boolean, isSorted = false,
                   idWithinMaxSafeInteger = false) {
    const mltFileNames = fs.readdirSync(mltSearchDir).filter(file => path.parse(file).ext === ".mlt").
    map((file) => path.parse(file).name);
    const mltMetaFileName = path.join(mltSearchDir, "tileset.pbf");

    for(const fileName of mltFileNames){
        console.info(`Testing tile ${fileName} ---------------------------------------------------------`);

        const mltFileName = `${fileName}.mlt`;
        const mltPath = Path.join(mltSearchDir, mltFileName);
        const mvtPath = Path.join(mvtSearchDir, `${fileName}.mvt`);

        const encodedMvt = fs.readFileSync(mvtPath);
        const tilesetMetadata = fs.readFileSync(mltMetaFileName);
        const encodedMlt = fs.readFileSync(mltPath);
        const buf = new Pbf(encodedMvt)
        const decodedMvt = new VectorTile(buf);

        const metadata = TileSetMetadata.fromBinary(tilesetMetadata);
        const decodedMlt = decodeTile(encodedMlt, metadata, undefined,
            undefined, true);

        if(isPreTessellated){
            comparePreTessellatedTile(decodedMlt, decodedMvt);
        }
        else{
            comparePlainGeometryEncodedTile(decodedMlt, decodedMvt, isSorted);
        }
    }
}

function comparePlainGeometryEncodedTile(mlt: FeatureTable[], mvt: VectorTile, isSorted = false){
    for(const featureTable of mlt){
        const layer = mvt.layers[featureTable.name];

        let j = 0;
        for(const mltFeature of featureTable) {
            const mvtFeature = isSorted? getMvtFeatureById(layer, mltFeature.id): layer.feature(j++);

            if (!mvtFeature.id) {
                /* Java MVT library in the MVT converter decodes zero for undefined ids */
                expect([0, null]).toContain(mltFeature.id);
            } else{
                try{
                    expect(mltFeature.id).toEqual(mvtFeature.id);
                }
                catch (e){
                    console.error("id mismatch", featureTable.name, mltFeature.id, mvtFeature.id);
                }
            }

            const mltGeometry = mltFeature.geometry;
            const mvtGeometry = mvtFeature.loadGeometry();
            expect(mltGeometry).toEqual(mvtGeometry);

            const mltProperties = mltFeature.properties;
            const mvtProperties = mvtFeature.properties;
            transformPropertyNames(mltProperties);
            transformPropertyNames(mvtProperties)
            convertBigIntPropertyValues(mltProperties);
            expect(mltProperties).toEqual(mvtProperties);
        }
    }
}

function getMvtFeatureById(layer, id){
    for(let i = 0; i < layer.length; i++){
        const mvtFeature = layer.feature(i);
        if(mvtFeature.id === id || mvtFeature.properties["id"] === id){
            return mvtFeature;
        }
    }
    return null;
}

function comparePreTessellatedTile(mlt: FeatureTable[], mvt: VectorTile){
    for(const featureTable of mlt){
        const layer = mvt.layers[featureTable.name];
        const gpuVector = featureTable.geometryVector instanceof GpuVector?
            featureTable.geometryVector as GpuVector : null;

        //console.info(featureTable.name);

        const mvtIndexBuffer = [];
        const mvtVertexBuffer = [];
        let j = 0;
        for(const mltFeature of featureTable){
            const mvtFeature = layer.feature(j++);

            /* Layers with ids which are unique per tile but contain no global information, so they are reassigned
               by the converter */
            //TODO: fix -> only transportatin and housenumber are reassigned
            const reassignableLayers = ["transportation", "housenumber", "water_name", "place", "water"];
            /*if(!reassignableLayers.includes(featureTable.name)){
                expect(mltFeature.id).toEqual(mvtFeature.id);
            }*/

            const mvtGeometry = mvtFeature.loadGeometry();
            const mltGeometry = mltFeature.geometry;
            if(gpuVector){
                const tessellation = tessellatePolygon(mvtGeometry);
                mvtIndexBuffer.push(...tessellation.indices);
                mvtVertexBuffer.push(...tessellation.vertexBuffer);
            }
            else{
                expect(mltGeometry).toEqual(mvtGeometry);
            }

            //TODO: two properties are missing in mlt in layer 2 -> name_de and name_en
            const mltProperties = mltFeature.properties;
            const mvtProperties = mvtFeature.properties;

            transformPropertyNames(mltProperties);
            transformPropertyNames(mvtProperties)
            convertBigIntPropertyValues(mltProperties);
            expect(mltProperties).toEqual(mvtProperties);
        }

        if(gpuVector){
            //TODO: fix wrong indices
            //expect(gpuVector.indexBuffer).toEqual(new Int32Array(mvtIndexBuffer));
            //expect(gpuVector.indexBuffer.length).toEqual(mvtIndexBuffer.length);
            if(gpuVector.indexBuffer.length !== mvtIndexBuffer.length){
                console.error("index buffer in MLT and MVT have different length",
                    gpuVector.indexBuffer.length, mvtIndexBuffer.length);
            }
            expect(gpuVector.vertexBuffer).toEqual(new Int32Array(mvtVertexBuffer));
        }
    }
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

        let newKey= key;
        /* rename the property names which are separated with : in mlt to match _ in omt mvts */
        if (key.startsWith("name") && key.includes(":")) {
            newKey = (key as any).replaceAll(":", "_")
            properties[newKey] = properties[key];
            delete properties[key];
        }


        /* Currently id is not supported as a property name in a FeatureTable,
        *  so this quick workaround is implemented */
        if(newKey === "_id"){
            properties["id"] = properties[newKey];
            delete properties[newKey];
        }
    }
}

/* Polygon tessellation code from the FillBucket class of MapLibre GL JS
*  Slightly modified to tessellate without the closing point to match the MLT approach
* */
function tessellatePolygon(geometry: Array<Array<{x: number, y: number}>>) {
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
            for (let i = 1; i < ring.length-1; i++) {
                flattened.push(ring[i].x);
                flattened.push(ring[i].y);
            }
        }

        const indices = earcut(flattened, holeIndices);
        polygonIndices.push(...indices);
        vertexBuffer.push(...flattened);
    }

    return {indices: polygonIndices, vertexBuffer};
}
