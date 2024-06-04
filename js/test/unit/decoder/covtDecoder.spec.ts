import * as fs from "fs";
import { MVTLayer, parseMvtTile } from "../../../src/mvtUtils";
import { CovtDecoder } from "../../../src/decoder/covtDecoder";
import * as Path from "path";
import { toBeDeepCloseTo, toMatchCloseTo } from "jest-matcher-deep-close-to";
expect.extend({ toBeDeepCloseTo, toMatchCloseTo });

const tilesDir = "../test/fixtures";

describe("CovtDecoder", () => {
    it.skip("should decode Amazon based tiles", async () => {
        const tiles = getTiles(Path.join(tilesDir, "amazon"));

        for (const tile of tiles) {
            console.info(tile.mlt);
            const mltTile = fs.readFileSync(tile.mlt);
            const mvtTile = fs.readFileSync(tile.mvt);
            const mvtLayers = parseMvtTile(mvtTile);

            const covtDecoder = new CovtDecoder(mltTile);

            compareTiles(covtDecoder, mvtLayers, true);
        }
    });

    it.skip("should decode Bing Map based tiles", async () => {
        const tiles = getTiles(Path.join(tilesDir, "bing"));

        for (const tile of tiles) {
            console.info(tile.mlt);
            // NB This tile is very slow (test times out), we skip it here for now
            if (tile.mlt === "data/bing/5-16-11.mlt") continue;
            const mltTile = fs.readFileSync(tile.mlt);
            const mvtTile = fs.readFileSync(tile.mvt);
            const mvtLayers = parseMvtTile(mvtTile);

            const covtDecoder = new CovtDecoder(mltTile);

            /* The features of Bing tiles have no ids */
            compareTiles(covtDecoder, mvtLayers, false);
        }
    });

    it("should decode OpenMapTiles schema based tiles", async () => {
        const tiles = getTiles(Path.join(tilesDir, "omt"));

        for (const tile of tiles) {
            const mltTile = fs.readFileSync(tile.mlt);
            const mvtTile = fs.readFileSync(tile.mvt);
            const mvtLayers = parseMvtTile(mvtTile);

            const covtDecoder = new CovtDecoder(mltTile);

            compareTiles(covtDecoder, mvtLayers);
        }
    });
});

function mapMvtProperties(mvtProperties: Map<string, unknown>): Map<string, unknown> {
    const transformedMvtProperties = new Map<string, unknown>();
    for (const [key, value] of mvtProperties) {
        if (value === undefined) {
            continue;
        }

        /* Bing Maps tiles contain id in the feature properties and the feature id is set to 0.
         * The Java MVT decoder used for the generation of the mltTiles ignores this id while the js decoder inserts the id in the feature properties.
         * */
        if (key.includes("id")) {
            //console.info("remove id property: ", value);
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

function getTiles(dir: string): { mvt: string; mlt: string }[] {
    const tiles = fs.readdirSync(dir)
        .filter(file => file.endsWith('.mvt') || file.endsWith('.pbf'))
        .sort();
    const mappedTiles = [];
    for (let i = 0; i < tiles.length; i += 2) {
        const tile = tiles[i];
        const mvtPath = Path.join(dir, tile);
        const mltPath = Path.join(dir.replace('fixtures','expected'), tile.replace(/\.(pbf|mvt)$/,'.mlt'));
        const tilePair = { mlt: mltPath, mvt: mvtPath };
        mappedTiles.push(tilePair);
    }
    return mappedTiles;
}

/*
 * Get a plain object representation of the geometry points
 *
 * Specifically remove member functions included by vector-tile
 */
function getPlainObjectGeometry(geometry) {
    if (geometry instanceof Array) {
        return geometry.map(part => getPlainObjectGeometry(part));
    }
    return {
        x: geometry.x,
        y: geometry.y,
    };
}

function compareTiles(covtDecoder: CovtDecoder, mvtLayers: MVTLayer[], compareIds = true) {
    //let mvtLayerId = 0;
    for (const covtLayerName of covtDecoder.layerNames) {
        const covtLayer = covtDecoder.getLayerTable(covtLayerName);
        //const mvtLayer = mvtLayers[mvtLayerId++];
        //Depending on the used MVT decoding library the layer can be in different order
        const mvtLayer = mvtLayers.find((l) => l.name === covtLayer.layerMetadata.name);

        //console.info(covtLayerName);

        /* the following layers are sorted based on the id in mltTiles */
        if (["building", "poi", "place"].indexOf(covtLayerName) !== -1) {
            mvtLayer.features.sort((a, b) => a.id - b.id);
        }

        Array.from(covtLayer).forEach((covtFeature, i) => {
            const { id: covtId, geometry: covtGeometry, properties: covtProperties } = covtFeature;
            const { id: mvtId, geometry: mvtGeometry, properties: mvtProperties } = mvtLayer.features[i];

            //TODO: fix id issue in place layer
            if (covtLayerName !== "place" && compareIds) {
                expect(covtId).toEqual(mvtId);
            }

            /*if (covtLayerName === "water_feature") {
                try {
                    expect(covtGeometry.format()).toEqual(mvtGeometry);
                } catch (e) {
                    console.error("error ", covtLayerName);
                }
            }*/

            expect(covtGeometry.format()).toEqual(getPlainObjectGeometry(mvtGeometry));

            const transformedMvtProperties = mapMvtProperties(mvtProperties);
            (expect(covtProperties) as any).toMatchCloseTo(transformedMvtProperties, 8);
        });
    }
}
