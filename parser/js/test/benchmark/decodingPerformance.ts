import MbTilesRepository from "../../src/evaluation/mbTilesRepository";
import zlib from "zlib";
import { parseMvtTile, parseMvtTileFast } from "../../src/mvtUtils";
import Benchmark = require("benchmark");
import * as fs from "fs";
import { decodeCovtFast } from "../../src/evaluation/covtConverterFast";
import { CovtDecoder } from "../../src/decoder/covtDecoder";

const MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";
//const COVT_FILE_NAME = "../../tests/fixtures/covt_omt/5_16_21.covt";
const COVT_FILE_NAME = "../../tests/fixtures/covt_omt/4_8_10.covt";
const covtBenchmarks = [];
const mvtBenchmarks = [];
const suiteRuns = 10;

(async () => {
    const mbTilesRepository = await MbTilesRepository.create(MBTILES_FILE_NAME);
    //const gzipCompressedMvtTile = await mbTilesRepository.getTile({ z: 5, x: 16, y: 21 });
    const gzipCompressedMvtTile = await mbTilesRepository.getTile({ z: 4, x: 8, y: 10 });

    const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);
    const covTile = fs.readFileSync(COVT_FILE_NAME);

    /*for (let i = 0; i < 100; i++) {
        console.time("mvt");
        const mvtLayers = parseMvtTile(mvtTile);
        console.timeEnd("mvt");
    }

    for (let i = 0; i < 100; i++) {
        console.time("covt");
        const covtLayers = decodeCovt(covTile);
        console.timeEnd("covt");
    }*/
    for (let i = 0; i < suiteRuns; i++) {
        new Benchmark.Suite()
            .add("MVT", () => {
                const mvtLayers = parseMvtTileFast(mvtTile);
            })
            .add("COVT", () => {
                //const covtLayers = decodeCovtFast(covTile);
                const covtDecoder = new CovtDecoder(covTile);
            })
            .on("cycle", (event) => console.info(String(event.target)))
            .on("complete", function () {
                const arr: any[] = Array.from(this);
                const covtBenchmark = arr.find((b) => b.name.includes("COVT"));
                const mvtBenchmark = arr.find((b) => b.name.includes("MVT"));
                console.log("MVT mean decoding time: ", mvtBenchmark.stats.mean);
                console.log("COVT mean decoding time: ", covtBenchmark.stats.mean);
                console.log("COVT to MVT decoding performance ratio: ", covtBenchmark.hz / mvtBenchmark.hz);
                covtBenchmarks.push(covtBenchmark.hz);
                mvtBenchmarks.push(mvtBenchmark.hz);
            })
            .run();
    }

    const covtAvgBenchmark = covtBenchmarks.reduce((p, c) => p + c, 0) / suiteRuns;
    const mvtAvgBenchmark = mvtBenchmarks.reduce((p, c) => p + c, 0) / suiteRuns;
    console.info("Avg Covt Benchmark: ", covtAvgBenchmark);
    console.info("Avg Mvt Benchmark: ", mvtAvgBenchmark);
    console.info("Covt to Mvt total decoding performance ratio: ", covtAvgBenchmark / mvtAvgBenchmark);
})();
