#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SyntheticTestRunner } from "synthetic-test-tool";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");

// FastPFOR-encoded tiles (requires MLT_WITH_FASTPFOR=ON at build time)
const SKIP_FASTPFOR = new Set([
  "polygon_fpf",
  "polygon_hole_fpf",
  "polygon_morton_tes",
  "polygon_multi_fpf",
  "polygon_fpf_tes",
]);

class SyntheticTestRunnerCpp extends SyntheticTestRunner {
  shouldSkip(testName) {
    if (SKIP_FASTPFOR.has(testName))
      return "FastPFor requires MLT_WITH_FASTPFOR=ON";
    return false;
  }

  async decodeMLT(mltFilePath) {
    const output = execFileSync(binary, [mltFilePath], { encoding: "utf-8" });
    return JSON.parse(output);
  }
}

const runner = new SyntheticTestRunnerCpp();
const { active, skipped: skippedCases } = await runner.getTestCases();

let passed = 0;
let failed = 0;
let skipped = 0;

for (const [testName, reason] of skippedCases) {
  console.log(`SKIP ${testName} (${reason})`);
  skipped++;
}

for (const testName of active) {
  const result = await runner.runCase(testName);
  switch (result.status) {
    case "ok": {
      console.log(`OK - ${result.testName}`);
      passed++;
      break;
    }
    case "fail": {
      console.log(`FAIL - ${result.testName}`);
      failed++;
      break;
    }
    case "skip": {
      console.log(`SKIP ${result.testName} (${result.reason})`);
      skipped++;
      break;
    }
  }
}

const total = passed + failed + skipped;
console.log(
  `\n${passed} passed, ${failed} failed, ${skipped} skipped, ${total} total`,
);
if (failed > 0) process.exit(1);
