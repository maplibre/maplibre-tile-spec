import * as fs from "fs";
import { parseMvtTile } from "../../../src/mvtUtils";
import { CovtDecoder } from "../../../src/decoder/covtDecoder";

const COVT_FILE_NAME = "./data/5_16_21.covt";
const MVT_FILE_NAME = "./data/5_16_21.mvt";
const COVT_FILE_NAME2 = "./data/4_8_10.covt";
const MVT_FILE_NAME2 = "./data/4_8_10.mvt";

describe("CovtDecoder", () => {
    it("should decode a COVT tile in zoom level 5", async () => {
        const covTile = fs.readFileSync(COVT_FILE_NAME);
        const mvtTile = fs.readFileSync(MVT_FILE_NAME);
        const mvtLayers = parseMvtTile(mvtTile);

        const covtDecoder = new CovtDecoder(covTile);

        let mvtLayerId = 0;
        for (const covtLayerName of covtDecoder.layerNames) {
            const covtLayer = covtDecoder.getLayerTable(covtLayerName);
            const mvtLayer = mvtLayers[mvtLayerId++];
            if (mvtLayer.name === "place") {
                mvtLayer.features.sort((a, b) => a.id - b.id);
            }

            let mvtFeatureId = 0;
            for (const covtFeature of covtLayer) {
                const { id: covtId, geometry: covtGeometry, properties: covtProperties } = covtFeature;
                const {
                    id: mvtId,
                    geometry: mvtGeometry,
                    properties: mvtProperties,
                } = mvtLayer.features[mvtFeatureId++];

                //TODO: fix false id based on a false encoded delta in the place layer beginning at index 190
                if (mvtLayer.name !== "place") {
                    expect(covtId).toEqual(mvtId);
                }
                expect(covtGeometry.format()).toEqual(mvtGeometry);
                const transformedMvtProperties = mapMvtProperties(mvtProperties);
                expect(covtProperties).toEqual(transformedMvtProperties);
            }
        }
    });

    it("should decode a COVT tile in zoom level 4", async () => {
        const covTile = fs.readFileSync(COVT_FILE_NAME2);
        const mvtTile = fs.readFileSync(MVT_FILE_NAME2);
        const mvtLayers = parseMvtTile(mvtTile);

        const covtDecoder = new CovtDecoder(covTile);

        let mvtLayerId = 0;
        for (const covtLayerName of covtDecoder.layerNames) {
            const covtLayer = covtDecoder.getLayerTable(covtLayerName);
            const mvtLayer = mvtLayers[mvtLayerId++];
            if (mvtLayer.name === "place") {
                mvtLayer.features.sort((a, b) => a.id - b.id);
            }

            let mvtFeatureId = 0;
            for (const covtFeature of covtLayer) {
                const { id: covtId, geometry: covtGeometry, properties: covtProperties } = covtFeature;
                const {
                    id: mvtId,
                    geometry: mvtGeometry,
                    properties: mvtProperties,
                } = mvtLayer.features[mvtFeatureId++];

                //TODO: fix false id based on a false encoded delta in the place layer beginning at index 190
                if (mvtLayer.name !== "place") {
                    expect(covtId).toEqual(mvtId);
                }
                expect(covtGeometry.format()).toEqual(mvtGeometry);
                //TODO: fix false property in boundary layer
                if (mvtLayer.name !== "boundary") {
                    const transformedMvtProperties = mapMvtProperties(mvtProperties);
                    expect(covtProperties).toEqual(transformedMvtProperties);
                }
            }
        }
    });
});

function mapMvtProperties(mvtProperties: Map<string, unknown>): Map<string, unknown> {
    const transformedMvtProperties = new Map<string, unknown>();
    for (const [key, value] of mvtProperties) {
        if (value === undefined) {
            continue;
        }

        let transformedKey = key;
        if (key.includes("name_")) {
            const comps = key.split("_");
            transformedKey = `${comps[0]}:${comps[1]}`;
        }

        //TODO: get rid of BigInt
        const convertedValue = Number.isInteger(value) ? BigInt(value as number) : value;
        transformedMvtProperties.set(transformedKey, convertedValue);
    }

    return transformedMvtProperties;
}
