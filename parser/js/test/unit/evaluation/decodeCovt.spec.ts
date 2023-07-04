import { decodeCovt, Geometry, Point } from "../../../src/evaluation";
import fs = require("fs");
import * as zlib from "zlib";
import MbTilesRepository from "../../../src/evaluation/mbTilesRepository";
import { parseMvtTile } from "../../../src/mvtUtils";

const MBTILES_FILE_NAME = "C:\\mapdata\\europe.mbtiles";
const COVT_FILE_NAME = "./data/5_16_21.covt";

describe("decodeCovt", () => {
    it("should decode a COVT tile", async () => {
        const mbTilesRepository = await MbTilesRepository.create(MBTILES_FILE_NAME);
        const gzipCompressedMvtTile = await mbTilesRepository.getTile({ z: 5, x: 16, y: 21 });

        const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);
        const covTile = fs.readFileSync(COVT_FILE_NAME);

        console.time("mvt");
        const mvtLayers = parseMvtTile(mvtTile);
        console.timeEnd("mvt");

        console.time("covt");
        const covtLayers = decodeCovt(covTile);
        console.timeEnd("covt");

        for (let i = 0; i < covtLayers.length; i++) {
            const covtLayer = covtLayers[i];
            const mvtLayer = mvtLayers[i];
            if (mvtLayer.name === "place") {
                mvtLayer.features.sort((a, b) => a.id - b.id);
                //covtLayer.ids.sort((a, b) => a - b);
            }

            //TODO: directly compare features
            const covtIds = covtLayer.map((f) => Number(f.id));
            const mvtIds = mvtLayer.features.map((f) => f.id);
            const covtGeometries = covtLayer.map((f) => f.geometry);
            const mvtGeometries = mvtLayer.features.map((f) => f.geometry);
            const covtProperties = covtLayer.map((f) => f.properties);
            const mvtProperties = mvtLayer.features.map((f) => f.properties);
            const transformedMvtProperties = [];
            for (const mvtProperty of mvtProperties) {
                const props = {};
                for (const [key, value] of Object.entries(mvtProperty)) {
                    if (key.includes("_")) {
                        const comps = key.split("_");
                        const transformedKey = `${comps[0]}:${comps[1]}`;
                        props[transformedKey] = value;
                    } else if (value === undefined) {
                        continue;
                    } else if (Number.isInteger(value)) {
                        props[key] = BigInt(value as any);
                    } else {
                        props[key] = value;
                    }
                }
                transformedMvtProperties.push(props);
            }

            const transformedCovtProperties = [];
            for (const covtProperty of covtProperties) {
                const props = {};
                for (const [key, value] of Object.entries(covtProperty)) {
                    if (value === undefined) {
                        continue;
                    } else {
                        props[key] = value;
                    }
                }
                transformedCovtProperties.push(props);
            }

            //expect(covtIds).toEqual(mvtIds);
            expect(covtGeometries).toEqual(mvtGeometries);
            //expect(transformedCovtProperties).toEqual(transformedMvtProperties);
        }
    });
});
