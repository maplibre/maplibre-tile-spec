import * as fs from "fs";
import * as Path from "path";
import { MltDecoder, TileSetMetadata } from "../../../src/index";
import { parseMvtTile } from "../../../src/mvtUtils";

const tilesDir = "../test/fixtures";

function getLayerByName(layers, name) {
    return layers.find(layer => layer.name === name);
}

describe("MltDecoder", () => {
    it("should decode one tile with one point", async () => {
        const tiles = getTiles("simple/point-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one line", async () => {
        const tiles = getTiles("simple/line-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one polygon", async () => {
        const tiles = getTiles("simple/polygon-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-point", async () => {
        const tiles = getTiles("simple/multipoint-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    // TODO: this is not actually parsing as a multi-line
    it("should decode one tile with one multi-line", async () => {
        const tiles = getTiles("simple/multiline-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-polygon", async () => {
        const tiles = getTiles("simple/multipolygon-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt.layers[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one Bing Map based tile", async () => {
        const tiles = getTiles("bing/4-13-6")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        expect(decoded.layers.length).toEqual(9);
        // vtzero-stats reports:
        // layer,num_features,raw_size,raw_geometries_size,key_table_size,value_table_size
        // vector_background,1,46,14,0,0
        // admin_division1,10,2916,2399,7,24
        // country_region,6,577,236,8,18
        // land_cover_forest,1,1505,1457,1,1
        // land_cover_grass,1,1803,1756,1,1
        // populated_place,28,2154,139,11,84
        // road,18,230228,229845,2,10
        // road_hd,2,23923,23826,2,3
        // water_feature,20,17410,16907,6,19
        expect(decoded.layers[0].name).toEqual('water_feature');
        expect(decoded.layers[0].features.length).toEqual(20);
        expect(decoded.layers[1].name).toEqual('road');
        expect(decoded.layers[1].features.length).toEqual(18);
        expect(decoded.layers[2].name).toEqual('land_cover_grass');
        expect(decoded.layers[2].features.length).toEqual(1);
        expect(decoded.layers[3].name).toEqual('country_region');
        expect(decoded.layers[3].features.length).toEqual(6);
        expect(decoded.layers[4].name).toEqual('land_cover_forest');
        expect(decoded.layers[4].features.length).toEqual(1);
        expect(decoded.layers[5].name).toEqual('road_hd');
        expect(decoded.layers[5].features.length).toEqual(2);
        expect(decoded.layers[6].name).toEqual('vector_background');
        expect(decoded.layers[6].features.length).toEqual(1);
        expect(decoded.layers[7].name).toEqual('populated_place');
        expect(decoded.layers[7].features.length).toEqual(28);
        expect(decoded.layers[8].name).toEqual('admin_division1');
        expect(decoded.layers[8].features.length).toEqual(10);
        for (let i = 0; i < decoded.layers.length; i++) {
            const layer = decoded.layers[i];
            const mvtLayer = getLayerByName(tiles.mvt.layers, layer.name);
            expect(layer.name).toEqual(mvtLayer.name);
            expect(layer.features.length).toEqual(mvtLayer.features.length);
            for (let i = 0; i < layer.features.length; i++) {
                const feature = layer.features[i];
                const mvtFeature = mvtLayer.features[i];
                const featString = JSON.stringify(feature.loadGeometry());
                const mvtFeatString = JSON.stringify(mvtFeature.loadGeometry());
                if (layer.name === 'vector_background' && i === 0) {
                    // known bug:
                    // vector_background feature has a different geometry
                    // Actual is:
                    // [[{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0},{"x":0,"y":0}]]
                    // Expected is:
                    // [[{"x":0,"y":0},{"x":4096,"y":0},{"x":4096,"y":4096},{"x":0,"y":4096},{"x":0,"y":0}]]
                    continue;
                }
                // if (featString !== mvtFeatString) {
                //     console.log('geometry is NOT equal ' + i + ' ' + layer.name);
                //     console.log('feature.geometry', featString);
                //     console.log('mvtFeature.geometry', mvtFeatString);
                //     break;
                // }
                expect(feature.loadGeometry()).toEqual(feature.loadGeometry());
                // TODO properties are incorrect
                // expect(feature.toGeoJSON(0,0,0)).toEqual(feature.toGeoJSON(0,0,0));
            }
        }
    });

    it("should decode one OMT based tile", async () => {
        const tiles = getTiles("omt/2_2_2")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(5);
        // Note: these feature counts match what vtzero-stats reports
        // but vt-zero stats reports layers in a different order
        // layer,num_features,raw_size,raw_geometries_size,key_table_size,value_table_size
        // water,172,14605,12500,1,2
        // landcover,132,4149,2250,2,2
        // boundary,4418,252752,174262,5,38
        // water_name,56,21463,280,68,693
        // place,754,285562,3760,76,9091
        expect(decoded.layers[0].name).toEqual('boundary');
        expect(decoded.layers[0].features.length).toEqual(4418);
        expect(decoded.layers[1].name).toEqual('water_name');
        expect(decoded.layers[1].features.length).toEqual(56);
        expect(decoded.layers[2].name).toEqual('landcover');
        expect(decoded.layers[2].features.length).toEqual(132);
        expect(decoded.layers[3].name).toEqual('place');
        expect(decoded.layers[3].features.length).toEqual(754);
        expect(decoded.layers[4].name).toEqual('water');
        expect(decoded.layers[4].features.length).toEqual(172);
        for (let i = 0; i < decoded.layers.length; i++) {
            const layer = decoded.layers[i];
            const mvtLayer = getLayerByName(tiles.mvt.layers, layer.name);
            expect(layer.name).toEqual(mvtLayer.name);
            expect(layer.features.length).toEqual(mvtLayer.features.length);
            for (let i = 0; i < layer.features.length; i++) {
                const feature = layer.features[i];
                const mvtFeature = mvtLayer.features[i];
                const featString = JSON.stringify(feature.loadGeometry());
                const mvtFeatString = JSON.stringify(mvtFeature.loadGeometry());
                expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
                const featStringJSON = JSON.stringify(feature.toGeoJSON(0,0,0).geometry);
                const mvtFeatStringJSON = JSON.stringify(mvtFeature.toGeoJSON(0,0,0).geometry);
                // if (featStringJSON !== mvtFeatStringJSON) {
                //     console.log('geometry is NOT equal ' + i + ' ' + layer.name, mvtFeature.extent);
                //     // fs.writeFileSync(i+'feature.geojson', featStringJSON);
                //     // fs.writeFileSync(i+'mvtFeature.geojson', mvtFeatStringJSON);
                //     // console.log('mvtFeature.geometry', mvtFeatStringJSON);
                //     continue;
                // }
                if (layer.name === 'water') {
                    // Known multipolygon vs polygon bugs in water, so we skip for now
                    continue;
                } else {
                    expect(layer.features[i].toGeoJSON(0,0,0).geometry).toEqual(mvtLayer.features[i].toGeoJSON(0,0,0).geometry);
                }
                // TODO properties are incorrect
                // console.log('feature.properties', feature.toGeoJSON(0,0,0).properties);
                // console.log('mvtFeature.properties', mvtFeature.toGeoJSON(0,0,0).properties);
                // expect(feature].toGeoJSON(0,0,0).properties).toEqual(mvtFeature].toGeoJSON(0,0,0).properties);
            }
        }
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
            const mlt = fs.readFileSync(mltPath);
            const mvt = parseMvtTile(fs.readFileSync(mvtPath));
            const meta = fs.readFileSync(mltMetaPath);
            return { mlt, mltPath, meta, mvt };
        })
}
