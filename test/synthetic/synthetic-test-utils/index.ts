import { globSync, readFileSync, writeFileSync } from "node:fs";
import * as path from "node:path";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));
const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

export function compareWithTolerance(
  received: unknown,
  expected: unknown,
): boolean | undefined {
  if (typeof expected === "string") {
    if (expected.endsWith("NAN")) {
      expected = Number.NaN;
    } else if (expected.endsWith("INFINITY")) {
      expected = expected.endsWith("NEG_INFINITY")
        ? Number.NEGATIVE_INFINITY
        : Number.POSITIVE_INFINITY;
    }
  }

  if (typeof received !== "number" || typeof expected !== "number") {
    return undefined;
  }

  if (!Number.isFinite(expected)) return Object.is(received, expected);

  if (Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
    return Math.abs(received) <= ABSOLUTE_FLOAT_TOLERANCE;
  }

  const relativeError = Math.abs(received - expected) / Math.abs(expected);
  return relativeError <= RELATIVE_FLOAT_TOLERANCE;
}

export function writeActualOutput(
  mltFile: string,
  actual: Record<string, unknown>,
): string {
  const actualFile = mltFile.replace(/\.mlt$/, ".actual.json");
  writeFileSync(actualFile, `${JSON.stringify(actual, null, 2)}\n`, "utf-8");
  return actualFile;
}

export type TestCase = { name: string; content: object; fileName: string };

export function getTestCases(skipList: string[]): {
  active: TestCase[];
  skipped: TestCase[];
} {
  const syntheticDir = resolve(__dirname, "..");
  const mltFiles = globSync(`**/*.mlt`, {
    cwd: syntheticDir,
  }).map((mltFile: string) => path.join(syntheticDir, mltFile));

  const active: TestCase[] = [];
  const skipped: TestCase[] = [];
  const matched = new Set<string>();

  for (const mltFile of mltFiles) {
    const testName = path.relative(syntheticDir, mltFile).replace(/\.mlt$/, "");
    const jsonFile = mltFile.replace(/\.mlt$/, ".json");
    const expected = JSON.parse(readFileSync(jsonFile, "utf-8"));
    const testCase = { name: testName, fileName: mltFile, content: expected };
    if (skipList.includes(testName)) {
      matched.add(testName);
      skipped.push(testCase);
    } else {
      active.push(testCase);
    }
  }

  const unmatched = skipList.filter((name) => !matched.has(name));
  if (unmatched.length > 0) {
    throw new Error(
      `Exclusion list references unknown synthetic test(s): ${unmatched.join(", ")}. ` +
        `Use the full path-style name, e.g. "0x01/poly_fpf".`,
    );
  }

  return { active, skipped };
}

export async function expectUnsupported(
  decode: () => Promise<unknown>,
  content: object,
): Promise<void> {
  let actual: unknown;
  try {
    actual = await decode();
  } catch {
    return;
  }
  expect(
    actual,
    "decoded and matched the expected output — remove it from the exclusion list",
  ).not.toEqual(content);
}
