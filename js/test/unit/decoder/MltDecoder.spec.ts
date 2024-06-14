import * as fs from "fs";
import * as Path from "path";
import { MltDecoder } from "../../../src/decoder/MltDecoder";
import { TileSetMetadata } from "../../../src/metadata/mlt_tileset_metadata_pb";

const tilesDir = "../test/fixtures";

describe("MltDecoder", () => {
    it("should decode one Bing Map based tile", async () => {
        const { mltMetadata, tiles } = getTiles(Path.join(tilesDir, "bing"));
        const tile = tiles.find(t => t.mlt.includes('4-13-6.mlt'));

        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(mltMetadata);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        try {
            const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        } catch (e) {
            expect(e.message).toEqual("Currently only FastPfor encoding supported for the VertexBuffer.");
        }
    });

    it("should decode one OMT based tile", async () => {
        const { mltMetadata, tiles } = getTiles(Path.join(tilesDir, "omt"));
        const tile = tiles.find(t => t.mlt.includes('2_2_2.mlt'));

        const mltTile = fs.readFileSync(tile.mlt);
        const mltMetadataPbf = fs.readFileSync(mltMetadata);
        const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
        try {
            const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
        } catch (e) {
            expect(e.message).toEqual("Strings are not supported yet for ScalarColumns.");
        }
    });

});

function getTiles(dir: string): { mltMetadata: string; tiles: { mvt: string; mlt: string }[] } {
    const mltDir = dir.replace('fixtures', 'expected');

    return {
        mltMetadata: Path.join(mltDir, 'mltmetadata.pbf'),
        tiles: fs.readdirSync(dir)
            .filter(file => file.endsWith('.mvt') || file.endsWith('.pbf'))
            .map((mvtFilename) : { mvt: string; mlt: string } => {
                const mvt = Path.join(dir, mvtFilename);

                const mltFilename = mvtFilename.replace(/\.(pbf|mvt)$/,'.mlt');
                const mlt = Path.join(mltDir, mltFilename);

                return { mlt, mvt };
            })
    };
}
