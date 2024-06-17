import * as fs from "fs";
import * as Path from "path";
import { MltDecoder } from "../../../src/decoder/MltDecoder";
import { TileSetMetadata } from "../../../src/metadata/mlt_tileset_metadata_pb";
import { parseMvtTile } from "../../../src/mvtUtils";

const tilesDir = "../test/fixtures";

describe("MltDecoder", () => {
    it("should decode one tile with one point", async () => {
        const tiles = getTiles("simple/point-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one line", async () => {
        const tiles = getTiles("simple/line-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one polygon", async () => {
        const tiles = getTiles("simple/polygon-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
        // console.log(feature.loadGeometry());
        // console.log(mvtFeature.loadGeometry());
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        // console.log(feature.toGeoJSON(0,0,0).geometry.coordinates);
        // console.log(mvtFeature.toGeoJSON(0,0,0).geometry.coordinates);
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-point", async () => {
        const tiles = getTiles("simple/multipoint-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    // TODO: this is not actually parsing as a multi-line
    it("should decode one tile with one multi-line", async () => {
        const tiles = getTiles("simple/multiline-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
        expect(feature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
        expect(feature.toGeoJSON(0,0,0)).toEqual(mvtFeature.toGeoJSON(0,0,0));
    });

    it("should decode one tile with one multi-polygon", async () => {
        const tiles = getTiles("simple/multipolygon-boolean")[0];
        const featureTables = MltDecoder.generateFeatureTables(TileSetMetadata.fromBinary(tiles.meta));
        const decoded = MltDecoder.decodeMlTile(tiles.mlt, featureTables);
        const feature = decoded.layers[0].features[0];
        const mvtFeature = tiles.mvt[0].features[0];
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
