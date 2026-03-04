import { globSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

export function compareWithTolerance(
  received: unknown,
  expected: unknown,
): boolean | undefined {
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

export function getTestCases(
  syntheticDir: string,
  skipList: string[],
): {
  active: { name: string; content: object; fileName: string }[];
  skipped: string[];
} {
  const mltFiles = [];
  for (const f of globSync(join(syntheticDir, "*.mlt"))) {
    mltFiles.push(f);
  }
  mltFiles.sort();

  const active: { name: string; content: object; fileName: string }[] = [];
  const skipped: string[] = [];

  for (const mltFile of mltFiles) {
    const testName = basename(mltFile).replace(/\.mlt$/, "");
    if (skipList.includes(testName)) {
      skipped.push(testName);
    } else {
      const jsonFile = join(syntheticDir, `${testName}.json`);
      const expectedRaw = readFileSync(jsonFile, "utf-8");
      const expected = JSON.parse(expectedRaw);
      active.push({ name: testName, fileName: mltFile, content: expected });
    }
  }

  return { active, skipped };
}
