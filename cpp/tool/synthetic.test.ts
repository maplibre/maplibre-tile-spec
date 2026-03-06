import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  compareWithTolerance,
  getTestCases,
  writeActualOutput,
} from "synthetic-test-utils";
import { describe, expect, it } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");

// FastPFOR-encoded tiles (requires MLT_WITH_FASTPFOR=ON at build time)
const SKIPPED_TESTS = [
  "polygon_fpf",
  "polygon_hole_fpf",
  "polygon_morton_tes",
  "polygon_multi_fpf",
  "polygon_fpf_tes",
  "poly_colinear_fpf",
  "poly_colinear_fpf_tes",
  "poly_fpf",
  "poly_fpf_tes",
  "poly_hole_fpf",
  "poly_hole_fpf_tes",
  "poly_hole_touching_fpf",
  "poly_hole_touching_fpf_tes",
  "poly_multi_fpf",
  "poly_multi_fpf_tes",
  "poly_self_intersect_fpf",
  "poly_self_intersect_fpf_tes",
];

describe("MLT Decoder - Synthetic tests", () => {
  expect.addEqualityTesters([compareWithTolerance]);
  const testCases = getTestCases(SKIPPED_TESTS);
  for (const { name, content, fileName } of testCases.active) {
    it(name, async () => {
      const actual = await decodeMLT(fileName);
      try {
        expect(actual).toEqual(content);
      } catch (error) {
        writeActualOutput(fileName, actual);
        throw error;
      }
    });
  }

  for (const skippedTest of testCases.skipped) {
    it.skip(skippedTest, () => {
      // Test is skipped since it is not supported yet
    });
  }
});

async function decodeMLT(mltFilePath: string) {
  const output = execFileSync(binary, [mltFilePath], { encoding: "utf-8" });
  return JSON.parse(output);
}
