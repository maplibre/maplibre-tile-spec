import { readFileSync } from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { bench, describe } from "vitest";
import decodeTile from "./mltDecoder";

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const tilePaths = [
    path.resolve(currentDir, "../../test/expected/tag0x01/omt/14_8298_10748.mlt"),
    path.resolve(currentDir, "../../test/expected/tag0x01/omt/11_1063_1367.mlt"),
];
const tileBuffers = tilePaths.map((tilePath) => new Uint8Array(readFileSync(tilePath)));

describe("MLT decoder performance", () => {
    bench("Decode properties only (deferred geometry)", () => {
        let sum = 0;

        for (const buffer of tileBuffers) {
            const tables = decodeTile(buffer);
            for (const table of tables) {
                sum += table.numFeatures;

                const propertyVectors = table.propertyVectors ?? [];
                if (propertyVectors.length > 0) {
                    const value = propertyVectors[0].getValue(0);
                    if (value !== null && value !== undefined) {
                        sum++;
                    }
                }
            }
        }

        if (sum === -1) {
            throw new Error("Bench guard");
        }
    });

    bench("Decode full (geometry + properties)", () => {
        let sum = 0;

        for (const buffer of tileBuffers) {
            const tables = decodeTile(buffer);
            for (const table of tables) {
                sum += table.getFeatures().length;
            }
        }

        if (sum === -1) {
            throw new Error("Bench guard");
        }
    });
});
