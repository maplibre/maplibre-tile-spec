import { parseMvtTileFast } from "../../src/mvtUtils";
import Benchmark = require("benchmark");
import * as fs from "fs";
import { CovtDecoder } from "../../src/decoder/covtDecoder";

const MVT_OMT_FILE_NAME = "./data/omt/5_16_20.mvt";
const COVT_OMT_FILE_NAME = "./data/omt/5_16_20.covt";
/*const MVT_BING_FILE_NAME = "./data/bing/6-32-22.mvt";
const COVT_BING_FILE_NAME = "./data/bing/6-32-22.covt";*/
/*const MVT_BING_FILE_NAME = "./data/bing/5-16-11.mvt";
const COVT_BING_FILE_NAME = "./data/bing/5-16-11.covt";*/
const MVT_BING_FILE_NAME = "./data/bing/4-8-5.mvt";
const COVT_BING_FILE_NAME = "./data/bing/4-8-5.covt";

const covtBenchmarks = [];
const mvtBenchmarks = [];
const suiteRuns = 10;

(async () => {
    /*const mvtTile = fs.readFileSync(MVT_OMT_FILE_NAME);
    const covTile = fs.readFileSync(COVT_OMT_FILE_NAME);*/
    const mvtTile = fs.readFileSync(MVT_BING_FILE_NAME);
    const covTile = fs.readFileSync(COVT_BING_FILE_NAME);

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
    //for (let i = 0; i < suiteRuns; i++) {
    new Benchmark.Suite()
        .add("MVT", () => {
            const mvtLayers = parseMvtTileFast(mvtTile);
        })
        .add("COVT", () => {
            const covtDecoder = new CovtDecoder(covTile, null);
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
    //}

    const covtAvgBenchmark = covtBenchmarks.reduce((p, c) => p + c, 0) / suiteRuns;
    const mvtAvgBenchmark = mvtBenchmarks.reduce((p, c) => p + c, 0) / suiteRuns;
    console.info("Avg Covt Benchmark: ", covtAvgBenchmark);
    console.info("Avg Mvt Benchmark: ", mvtAvgBenchmark);
    console.info("Covt to Mvt total decoding performance ratio: ", covtAvgBenchmark / mvtAvgBenchmark);
})();
