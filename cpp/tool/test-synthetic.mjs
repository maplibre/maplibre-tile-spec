#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SyntheticTestRunner } from "synthetic-test-tool";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");
const syntheticDir = resolve(__dirname, "../../test/synthetic/0x01");

class SyntheticTestRunnerCpp extends SyntheticTestRunner {
  shouldSkip(_testName) {
    return false;
  }

  async decodeMLT(mltFilePath) {
    const output = execFileSync(binary, [mltFilePath], { encoding: "utf-8" });
    return JSON.parse(output);
  }
}

await new SyntheticTestRunnerCpp()
  .run(syntheticDir)
  .catch(() => process.exit(1));
