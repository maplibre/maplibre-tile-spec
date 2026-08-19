import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { featureTablesToFeatureCollection } from "./vector/featureTablesToGeoJson";
import {
    compareWithTolerance,
    expectUnsupported,
    getTestCases,
    writeActualOutput,
} from "../../test/synthetic/synthetic-test-utils";
import decodeTile from "./mltDecoder";

/**
 * Synthetics the decoder cannot handle yet. These still run: `expectUnsupported` asserts they fail,
 * so an entry that starts decoding correctly fails the test until it is removed from this list.
 * Prefer fixing the decoder over adding to it.
 */
const UNIMPLEMENTED_SYNTHETICS: string[] = [];

describe("MLT Decoder - Synthetic tests", () => {
    expect.addEqualityTesters([compareWithTolerance]);
    const testCases = getTestCases(UNIMPLEMENTED_SYNTHETICS);
    for (const { name, content, fileName } of testCases.active) {
        it(name, async () => {
            const actual = await decodeMLT(fileName);
            writeActualOutput(fileName, actual);
            expect(actual).toEqual(content);
        });
    }

    for (const { name, content, fileName } of testCases.skipped) {
        it(`${name} (unsupported)`, () => expectUnsupported(() => decodeMLT(fileName), content));
    }
});

async function decodeMLT(mltFilePath: string) {
    const mltBuffer = await readFile(mltFilePath);
    const featureTables = decodeTile(mltBuffer, undefined, false);
    return featureTablesToFeatureCollection(featureTables) as unknown as Record<string, unknown>;
}
