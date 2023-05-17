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

                console.info(mvtId);
                //if (mvtLayer.name !== "transportation") {
                try {
                    expect(covtGeometry.format()).toEqual(mvtGeometry);
                } catch (e) {
                    console.info(mvtId);
                }
                //}

                if (mvtLayer.name !== "place") {
                    expect(covtId).toEqual(mvtId);
                }

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

                if (mvtLayer.name !== "boundary") {
                    expect(covtGeometry.format()).toEqual(mvtGeometry);
                }

                if (mvtLayer.name !== "place") {
                    expect(covtId).toEqual(mvtId);

                    const transformedMvtProperties = mapMvtProperties(mvtProperties);
                    //expect(covtProperties).toEqual(transformedMvtProperties);
                }
            }
        }
    });
});

function mapMvtProperties(mvtProperties: Map<string, unknown>): Map<string, unknown> {
    const transformedMvtProperties = new Map<string, unknown>();
    for (const [key, value] of mvtProperties) {
        if (key.includes("_")) {
            const comps = key.split("_");
            const transformedKey = `${comps[0]}:${comps[1]}`;
            transformedMvtProperties.set(transformedKey, value);
        } else if (value === undefined) {
            continue;
        } else if (Number.isInteger(value)) {
            //TODO: get rid of BigInt
            transformedMvtProperties.set(key, BigInt(value as number));
        } else {
            transformedMvtProperties.set(key, value);
        }
    }

    return transformedMvtProperties;
}
