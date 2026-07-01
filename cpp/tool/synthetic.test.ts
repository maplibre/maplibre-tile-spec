import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  compareWithTolerance,
  expectUnsupported,
  getTestCases,
  writeActualOutput,
} from "synthetic-test-utils";
import { describe, expect, it } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");

const SKIPPED_TESTS = [
  "0x02/prop_nested_big",
  "0x02/prop_nested_ints",
  "0x02/prop_nested_json",
  "0x02/prop_nested_list_root",
  "0x02/prop_nested_list",
  "0x02/prop_nested_mixed_root",
  "0x02/prop_nested_null",
  "0x02/prop_nested_shared",
  "0x02/prop_nested_specials",
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

  for (const { name, content, fileName } of testCases.skipped) {
    it(`${name} (unsupported)`, () =>
      expectUnsupported(() => decodeMLT(fileName), content));
  }
});

async function decodeMLT(mltFilePath: string) {
  const output = execFileSync(binary, [mltFilePath], { encoding: "utf-8" });
  return JSON.parse(output);
}
