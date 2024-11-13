import * as fs from "fs";
import * as Path from "path";
import { MltDecoder, TileSetMetadata } from "../../../src/index";
import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';

const tilesDir = "../test/fixtures";

function printValue(key, value) {
  return typeof value === 'bigint'
  ? value.toString()
  : value;
}

describe("MltDecoder", () => {
    it("should decode one tile with one point", async () => {
        const tiles = getTiles("simple/point-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(feature.extent).toEqual(mvtFeature.extent);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
        expect(feature.toGeoJSON(1,2,3)).toEqual(mvtFeature.toGeoJSON(1,2,3));
    });

    it("should decode one tile with one line", async () => {
        const tiles = getTiles("simple/line-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one polygon", async () => {
        const tiles = getTiles("simple/polygon-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-point", async () => {
        const tiles = getTiles("simple/multipoint-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    // TODO: this is not actually parsing as a multi-line
    it("should decode one tile with one multi-line", async () => {
        const tiles = getTiles("simple/multiline-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-polygon", async () => {
        const tiles = getTiles("simple/multipolygon-boolean")[0];
        const mltLayer = tiles.mlt.layers['layer'];
        const mvtLayer = tiles.mvt.layers['layer'];
        expect(mltLayer.name).toEqual(mvtLayer.name);
        expect(mltLayer.length).toEqual(mvtLayer.length);
        expect(mltLayer.version).toEqual(mvtLayer.version);
        const feature = mltLayer.feature(0);
        const mvtFeature = mvtLayer.feature(0);
        expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
        expect(feature.toGeoJSON(1,2,3)).toEqual(mvtFeature.toGeoJSON(1,2,3));
    });

    it("should decode one Bing Map based tile", async () => {
        const tiles = getTiles("bing/4-13-6")[0];
        expect(Object.keys(tiles.mlt.layers).length).toEqual(9);
        let numGeomErrors = 0;
        let numFeaturesErrors = 0;
        for (const layerName of Object.keys(tiles.mlt.layers)) {
            const layer = tiles.mlt.layers[layerName];
            const mvtLayer = tiles.mvt.layers[layerName];
            expect(layer.name).toEqual(mvtLayer.name);
            expect(layer.length).toEqual(mvtLayer.length);
            for (let i = 0; i < layer.length; i++) {
                const feature = layer.feature(i);
                const mvtFeature = mvtLayer.feature(i);
                if (layer.name === 'vector_background' && i === 0) {
                    // TODO: known bug:
                    // vector_background feature has a different geometry
                    // Actual is:
                    // [[{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0}]]
                    // Expected is:
                    // [[{"x":0,"y":0},{"x":4096,"y":0},{"x":4096,"y":4096},{"x":0,"y":4096},{"x":0,"y":0}]]
                    numGeomErrors++;
                    continue;
                } else {
                    expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
                }
                const featureKeys = Object.keys(feature.properties).sort();
                const mvtFeatureKeys = Object.keys(mvtFeature.properties).sort();
                // workaround https://github.com/maplibre/maplibre-tile-spec/issues/181
                if (mvtFeatureKeys.indexOf('id') !== -1) {
                  mvtFeatureKeys.splice(mvtFeatureKeys.indexOf('id'), 1);
                }
                const featureKeysString = JSON.stringify(featureKeys);
                const mvtFeatureKeysString = JSON.stringify(mvtFeatureKeys);
                // For Bing tiles, if we remove the missing id key, then keys should match
                expect(featureKeysString).toEqual(mvtFeatureKeysString);
                const featStringJSON = JSON.stringify(Object.entries(feature.properties),printValue);
                const mvtFeatStringJSON = JSON.stringify(Object.entries(mvtFeature.properties),printValue);
                if (featStringJSON !== mvtFeatStringJSON) {
                    // console.log(feature.id + ' for layer '  + layer.name + ' feature.properties', feature.properties);
                    // console.log(mvtFeature.id + ' for layer '  + layer.name + ' mvtFeature.properties', mvtFeature.properties);
                    numFeaturesErrors++;
                } else {
                    expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
                }
            }
        }
        // Currently expect 1 difference in geometry in vector_background layer
        expect(numGeomErrors).toEqual(1);
        // Currently major differences in properties
        expect(numFeaturesErrors).toEqual(86);
    });

    it("should decode one OMT based tile", async () => {
        const tiles = getTiles("omt/2_2_2")[0];
        expect(Object.keys(tiles.mlt.layers).length).toEqual(5);
        let numErrors = 0;
        let numFeaturesErrors = 0;
        for (const layerName of Object.keys(tiles.mlt.layers)) {
            const layer = tiles.mlt.layers[layerName];
            const mvtLayer = tiles.mvt.layers[layerName];
            expect(layer.name).toEqual(mvtLayer.name);
            expect(layer.length).toEqual(mvtLayer.length);
            for (let i = 0; i < layer.length; i++) {
                const feature = layer.feature(i);
                const mvtFeature = mvtLayer.feature(i);
                expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
                if (layer.name === 'water') {
                    // TODO: Known multipolygon vs polygon bugs in water, so we skip for now
                    const featStringJSON = JSON.stringify(feature.toGeoJSON(0,0,0).geometry);
                    const mvtFeatStringJSON = JSON.stringify(mvtFeature.toGeoJSON(0,0,0).geometry);
                    if (featStringJSON !== mvtFeatStringJSON) {
                        numErrors++;
                        // console.log('geometry is NOT equal ' + i + ' ' + layer.name);
                        // fs.writeFileSync(i+'feature.geojson', featStringJSON);
                        // fs.writeFileSync(i+'mvtFeature.geojson', mvtFeatStringJSON);
                        // console.log('mvtFeature.geometry', mvtFeatStringJSON);
                        // console.log('feature.geometry', featStringJSON);
                        continue;
                    } else {
                        expect(feature.toGeoJSON(0,0,0).geometry).toEqual(mvtFeature.toGeoJSON(0,0,0).geometry);
                    }
                } else {
                    expect(feature.toGeoJSON(0,0,0).geometry).toEqual(mvtFeature.toGeoJSON(0,0,0).geometry);
                }

                const featProperties = JSON.stringify(Object.entries(feature.properties),printValue);
                const mvtFeatProperties = JSON.stringify(Object.entries(mvtFeature.properties),printValue);
                if (featProperties !== mvtFeatProperties) {
                    // console.log(feature.id + ' for layer '  + layer.name + ' feature.properties', feature.properties);
                    // console.log(mvtFeature.id + ' for layer '  + layer.name + ' mvtFeature.properties', mvtFeature.properties);
                    numFeaturesErrors++;
                } else {
                    expect(Object.entries(feature.properties)).toEqual(Object.entries(mvtFeature.properties));
                }
            }
        }
        // Currently expect 4 differences in geometry in water layer
        expect(numErrors).toEqual(4);
        // Currently major differences in properties
        expect(numFeaturesErrors).toEqual(5228);
    });

});

function getTiles(pathname: string) {
    const fixtureDirname = Path.dirname(pathname);
    const searchDir = Path.join(tilesDir, fixtureDirname);
    return fs.readdirSync(searchDir)
        .filter(file => {
            const nameWithoutExt = Path.basename(file, Path.extname(file));
            if (!file.endsWith('.mvt') && !file.endsWith('.pbf')) {
                return false;
            }
            if (pathname.endsWith(nameWithoutExt)) {
                return true;
            }
            return false;
        })
        .map((mvtFilename) => {
            const mvtPath = Path.join(tilesDir, fixtureDirname, mvtFilename);
            const expectedDir = tilesDir.replace('fixtures','expected');
            const mltPath = Path.join(expectedDir, fixtureDirname, mvtFilename.replace(/\.(pbf|mvt)$/,'.mlt'));
            const mltMetaPath = mltPath.replace('.advanced','') + '.meta.pbf';
            const meta = fs.readFileSync(mltMetaPath);
            const mlt = MltDecoder.decodeMlTile(fs.readFileSync(mltPath), TileSetMetadata.fromBinary(meta));
            const mvt = new VectorTile(new Protobuf(fs.readFileSync(mvtPath)));
            return { mlt, mvt };
        })
}
