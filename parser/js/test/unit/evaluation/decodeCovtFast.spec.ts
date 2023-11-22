import { decodeCovt, Geometry, Point } from "../../../src/evaluation";
import fs = require("fs");
import * as zlib from "zlib";
import MbTilesRepository from "../../../src/evaluation/mbTilesRepository";
import { parseMvtTile, parseMvtTileFast } from "../../../src/mvtUtils";
import { decodeCovtFast } from "../../../src/evaluation/covtConverterFast";

const MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";
const COVT_FILE_NAME = "../../tests/fixtures/covt_omt/5_16_21.covt";

describe("decodeCovtFast", () => {
    it("should decode a COVT tile", async () => {
        const mbTilesRepository = await MbTilesRepository.create(MBTILES_FILE_NAME);
        const gzipCompressedMvtTile = await mbTilesRepository.getTile({ z: 5, x: 16, y: 21 });

        const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);
        const covTile = fs.readFileSync(COVT_FILE_NAME);

        for (let i = 0; i < 100; i++) {
            console.time("mvt");
            const mvtLayers = parseMvtTileFast(mvtTile);
            console.timeEnd("mvt");

            console.time("covt");
            const covtLayers = decodeCovtFast(covTile);
            console.timeEnd("covt");
        }
    });
});
