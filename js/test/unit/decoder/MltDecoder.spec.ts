import * as fs from "fs";
import * as Path from "path";
import { MltDecoder } from "../../../src/decoder/MltDecoder";
import { TileSetMetadata } from "../../../src/metadata/mlt_tileset_metadata_pb";

const tilesDir = "../test/fixtures";

describe("MltDecoder", () => {
    it("should decode one tile with one point", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/point-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        expect(decoded.layers[0].features[0].properties).toEqual({"key": true});
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual([25, 17]);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": [25, 17], "type": "Point"});
    });

    it("should decode one tile with one line", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/line-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual([ [ 2, 2 ], [ 2, 10 ], [ 10, 10 ] ]);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": [ [ 2, 2 ], [ 2, 10 ], [ 10, 10 ] ], "type": "LineString"});
    });

    it("should decode one tile with one polygon", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/polygon-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        const coords = [ [ [ 3, 6 ], [ 8, 12 ], [ 20, 34 ], [ 3, 6 ] ] ];
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual(coords);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": coords, "type": "Polygon"});
    });

    it("should decode one tile with one multi-point", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/multipoint-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        const coords = [ [ 5, 7 ], [ 3, 2 ] ];
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual(coords);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": coords, "type": "MultiPoint"});
    });

    it("should decode one tile with one multi-line", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/multiline-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        const coords = [ [ [ 2, 2 ], [ 2, 10 ], [ 10, 10 ] ], [ [ 1, 1 ], [ 3, 5 ] ] ];
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual(coords);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": coords, "type": "MultiLineString"});
    });

    it("should decode one tile with one multi-polygon", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "simple"));
        const tile = tiles.find(t => t.mlt.includes('/multipolygon-boolean.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
        expect(decoded.layers.length).toEqual(1);
        expect(decoded.layers[0].features.length).toEqual(1);
        const coords = [[[[0,0],[10,0],[10,10],[0,10],[0,0]]],[[[11,11],[20,11],[20,20],[11,20],[11,11]],[[13,13],[13,17],[17,17],[17,13],[13,13]]]];
        expect(decoded.layers[0].features[0].geometry.coordinates).toEqual(coords);
        expect(decoded.layers[0].features[0].geometry.toGeoJSON()).toEqual({"coordinates": coords, "type": "MultiPolygon"});
    });


    it("should decode one Bing Map based tile", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "bing"));
        const tile = tiles.find(t => t.mlt.includes('/4-13-6.mlt'));
        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        expect(decoded).toBeDefined();
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
        const { tiles } =  getTiles(Path.join(tilesDir, "omt"));
        const tile = tiles.find(t => t.mlt.includes('/2_2_2.mlt'));

        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
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

function getTiles(dir: string): { tiles: { mvt: string; meta: string; mlt: string }[] } {
    const mltDir = dir.replace('fixtures', 'expected');

    return {
        tiles: fs.readdirSync(dir)
            .filter(file => file.endsWith('.mvt') || file.endsWith('.pbf'))
            .map((mvtFilename) : { mvt: string; meta: string; mlt: string } => {
                const mvt = Path.join(dir, mvtFilename);

                const mltFilename = mvtFilename.replace(/\.(pbf|mvt)$/,'.mlt');
                const mlt = Path.join(mltDir, mltFilename);

                const meta = mlt.replace('.advanced','') + '.meta.pbf';
                return { mlt, meta, mvt };
            })
    };
}
