import { describe, it, expect } from "vitest";
import { decodeTile } from "../index";
import path from "node:path";
import fs from "node:fs";

const TILE = path.resolve(__dirname, "../../../test/expected/tag0x01/simple/multiline-boolean.mlt");

describe("FeatureTable", () => {
    it("should iterate through features correctly", () => {
        const bytes = new Uint8Array(fs.readFileSync(TILE));
        const featureTables = decodeTile(bytes);

        const table = featureTables[0];

        expect(table.name).toBe("layer");
        expect(table.extent).toBe(4096);

        let featureCount = 0;
        for (const feature of table) {
            expect(feature.geometry).toBeTruthy();
            expect(feature.geometry.coordinates).toBeInstanceOf(Array);
            expect(feature.geometry.coordinates.length).toBeGreaterThan(0);
            expect(typeof feature.geometry.type).toBe("number");

            featureCount++;
        }
        expect(featureCount).toBe(table.numFeatures);
    });
});