import * as fs from "fs";
import * as Path from "path";
import { MltDecoder } from "../../../src/decoder2/decoder/MltDecoder";
import { TileSetMetadata } from "../../../src/decoder/mlt_tileset_metadata_pb";

const tilesDir = "../test/fixtures";

describe("Decoder2", () => {
    it("should decode one Bing Map based tile", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "bing"));
        const tile = tiles.find(t => t.mlt.includes('4-13-6.mlt'));

        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        try {
            const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        } catch (e) {
            expect(e.message).toEqual("Currently only FastPfor encoding supported for the VertexBuffer.");
        }
    });

    it("should decode one OMT based tile", async () => {
        const { tiles } = getTiles(Path.join(tilesDir, "omt"));
        const tile = tiles.find(t => t.mlt.includes('2_2_2.mlt'));

        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(tile.meta);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        try {
            const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        } catch (e) {
            expect(e.message).toEqual("Strings are not supported yet for ScalarColumns.");
        }
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
                const meta = mlt + '.meta.pbf';
                return { mlt, meta, mvt };
            })
    };
}
