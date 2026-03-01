#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SyntheticTestRunner } from "synthetic-test-tool";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");
const syntheticDir = resolve(__dirname, "../../test/synthetic/0x01");

// Expected JSON files that contain NaN or Infinity (not valid JSON)
const SKIP_NAN_INF = new Set([
  "prop_f32_nan",
  "prop_f32_neg_inf",
  "prop_f32_pos_inf",
  "prop_f64_nan",
  "prop_f64_neg_inf",
  "prop_f64_max",
]);

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
    if (SKIP_NAN_INF.has(testName)) return "NaN/Infinity not valid JSON";
    if (SKIP_FASTPFOR.has(testName)) return "FastPFor requires MLT_WITH_FASTPFOR=ON";
    return false;
  }

  async decodeMLT(mltFilePath) {
    const output = execFileSync(binary, [mltFilePath], { encoding: "utf-8" });
    return JSON.parse(output);
  }
}

await new SyntheticTestRunnerCpp().run(syntheticDir).catch(() => process.exit(1));
