import MbTilesRepository from "../../src/evaluation/mbTilesRepository";
import zlib from "zlib";
import { parseMvtTile } from "../../src/mvtUtils";
import { decodeCovt } from "../../src/evaluation";
import Benchmark = require("benchmark");
import * as fs from "fs";

const MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";
const COVT_FILE_NAME = "./data/5_16_21.covt";

(async () => {
    const mbTilesRepository = await MbTilesRepository.create(MBTILES_FILE_NAME);
    const gzipCompressedMvtTile = await mbTilesRepository.getTile({ z: 5, x: 16, y: 21 });

    const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);
    const covTile = fs.readFileSync(COVT_FILE_NAME);

    const mvtLayers = parseMvtTile(mvtTile);
    const covtLayers = decodeCovt(covTile);
    const covtTransportationLayer = covtLayers[3];
    const covtTransportationVertexBuffer = [];
    for (const geometry of covtTransportationLayer.geometries) {
        for (const ring of geometry) {
            for (const vertex of ring) {
                const v: any = vertex;
                //TODO: test with dictionary as the coordinates are ICE encoded
                covtTransportationVertexBuffer.push(v.x);
                covtTransportationVertexBuffer.push(v.y);
            }
        }
    }

    const mvtTransportationLayer = mvtLayers[3].features;
    new Benchmark.Suite()
        .add("MVT", () => {
            const scale = 2;
            for (const feature of mvtTransportationLayer) {
                const geometry = feature.geometry;
                for (let r = 0; r < geometry.length; r++) {
                    const ring = geometry[r];
                    for (let p = 0; p < ring.length; p++) {
                        const point = ring[p];
                        point.x = Math.round(point.x * scale);
                        point.y = Math.round(point.y * scale);
                    }
                }
            }
        })
        .add("COVT", () => {
            const scale = 2;
            /*for (const covtLayer of covtLayers) {
                for (const geometry of covtLayer.geometries) {

                    for (let r = 0; r < geometry.length; r++) {
                        const ring = geometry[r];
                        for (let p = 0; p < ring.length; p++) {
                            const point = ring[p];
                            point.x = Math.round(point.x * scale);
                            point.y = Math.round(point.y * scale);
                        }
                    }
                }
            }*/
        })
        .on("cycle", (event) => console.info(String(event.target)))
        .on("complete", function () {
            const arr: any[] = Array.from(this);
            const covtBenchmark = arr.find((b) => b.name.includes("COVT"));
            const mvtBenchmark = arr.find((b) => b.name.includes("MVT"));
            console.log("MVT mean decoding time: ", mvtBenchmark.stats.mean);
            console.log("COVT mean decoding time: ", covtBenchmark.stats.mean);
            console.log("COVT to MVT decoding performance ratio: ", covtBenchmark.hz / mvtBenchmark.hz);
        })
        .run();
})();
