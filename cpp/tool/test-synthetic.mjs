#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { glob, readFile } from "node:fs/promises";
import { deepStrictEqual } from "node:assert";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const binary = resolve(__dirname, "../build/tool/mlt-cpp-json");
const mltFiles = (await Array.fromAsync(glob(resolve(__dirname, "../../test/synthetic/0x01/*.mlt")))).sort();

// Normalize numbers to f32 precision so that f32→f64 widening differences are ignored.
// Values that survive Math.fround unchanged (small integers, etc.) are not affected.
function normalizeFloats(val) {
  if (typeof val === "number") return Math.fround(val);
  if (Array.isArray(val)) return val.map(normalizeFloats);
  if (val && typeof val === "object") {
    return Object.fromEntries(Object.entries(val).map(([k, v]) => [k, normalizeFloats(v)]));
  }
  return val;
}

// Expected JSON files that contain NaN or Infinity (not valid JSON)
const SKIP_NAN_INF = new Set([
  "prop_f32_nan.mlt",
  "prop_f32_neg_inf.mlt",
  "prop_f32_pos_inf.mlt",
  "prop_f64_nan.mlt",
  "prop_f64_neg_inf.mlt",
  "prop_f64_max.mlt",
]);

// FastPFOR-encoded tiles (requires MLT_WITH_FASTPFOR=ON at build time)
const SKIP_FASTPFOR = new Set([
  "polygon_fpf.mlt",
  "polygon_hole_fpf.mlt",
  "polygon_morton_tes.mlt",
  "polygon_multi_fpf.mlt",
]);

console.log(`Found ${mltFiles.length} MLT files\n`);

let passed = 0;
let failed = 0;
let skipped = 0;

for (const mltFile of mltFiles) {
  const name = mltFile.split("/").pop();
  const jsonFile = mltFile.replace(/\.mlt$/, ".json");

  if (SKIP_NAN_INF.has(name) || SKIP_FASTPFOR.has(name)) {
    console.log(`SKIP ${name}`);
    skipped++;
    continue;
  }

  try {
    const actual = execFileSync(binary, [mltFile], { encoding: "utf-8" });
    const expected = await readFile(jsonFile, "utf-8");

    const actualObj = normalizeFloats(JSON.parse(actual));
    const expectedObj = normalizeFloats(JSON.parse(expected));

    deepStrictEqual(actualObj, expectedObj);
    console.log(`OK   ${name}`);
    passed++;
  } catch (err) {
    if (err?.code === "ERR_ASSERTION") {
      console.log(`DIFF ${name}`);
      console.log(`  ${err.message.split("\n").slice(0, 3).join("\n  ")}`);
    } else {
      console.log(`FAIL ${name}`);
      console.log(`  ${err.stderr?.trim() || err.message}`);
    }
    failed++;
  }
}

console.log(`\n${passed} passed, ${failed} failed, ${skipped} skipped, ${mltFiles.length} total`);
process.exit(failed > 0 ? 1 : 0);
