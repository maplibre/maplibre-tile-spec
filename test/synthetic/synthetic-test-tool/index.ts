import { glob, readFile, writeFile } from "node:fs/promises";
import { basename, join } from "node:path";
import JSON5 from "json5";
import { expect } from "vitest";

async function collectGlob(pattern: string): Promise<string[]> {
  const result: string[] = [];
  for await (const f of glob(pattern)) {
    result.push(f);
  }
  return result.sort();
}

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

expect.addEqualityTesters([
  (received, expected) => {
    if (typeof received !== "number" || typeof expected !== "number") {
      return undefined;
    }

    if (!Number.isFinite(expected)) return Object.is(received, expected);

    if (Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
      return Math.abs(received) <= ABSOLUTE_FLOAT_TOLERANCE;
    }

    const relativeError = Math.abs(received - expected) / Math.abs(expected);
    return relativeError <= RELATIVE_FLOAT_TOLERANCE;
  },
]);

export class SyntheticTestRunner {
  shouldSkip(_testName: string): false | string {
    return false;
  }

  private async writeActualOutput(
    mltFile: string,
    actual: Record<string, unknown>,
  ) {
    const actualFile = mltFile.replace(/\.mlt$/, ".actual.json");
    await writeFile(
      actualFile,
      `${JSON5.stringify(actual, null, 2)}\n`,
      "utf-8",
    );
    return actualFile;
  }

  decodeMLT(_mltFilePath: string): Promise<Record<string, unknown>> {
    throw new Error("not implemented");
  }

  async run(syntheticDir: string): Promise<void> {
    const mltFiles = await collectGlob(join(syntheticDir, "*.mlt"));
    let passed = 0;
    let failed = 0;
    let skipped = 0;

    for (const mltFile of mltFiles) {
      const name = basename(mltFile);
      const testName = name.replace(/\.mlt$/, "");
      const jsonFile = mltFile.replace(/\.mlt$/, ".json");

      const skipReason = this.shouldSkip(testName);
      if (skipReason !== false) {
        console.log(`SKIP ${name} (${skipReason})`);
        skipped++;
        continue;
      }

      let actual: Record<string, unknown>;
      try {
        actual = await this.decodeMLT(mltFile);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.log(`FAIL - ${name} (crash: ${msg})`);
        failed++;
        continue;
      }

      const expectedRaw = await readFile(jsonFile, "utf-8");
      const expected = JSON5.parse(expectedRaw);

      try {
        expect(actual).toEqual(expected);
        console.log(`OK - ${name}`);
        passed++;
      } catch (_err) {
        console.log(`FAIL - ${name}`);
        const actualFile = await this.writeActualOutput(mltFile, actual);
        console.log(`wrote actual output to ${actualFile}`);
        failed++;
      }
    }

    console.log(
      `\n${passed} passed, ${failed} failed, ${skipped} skipped, ${mltFiles.length} total`,
    );
    if (failed > 0) throw new Error(`${failed} test(s) failed`);
  }

  async getTestCases(
    syntheticDir: string,
  ): Promise<{ active: string[]; skipped: [string, string][] }> {
    const mltFiles = await collectGlob(join(syntheticDir, "*.mlt"));
    const testNames = mltFiles.map((f) => basename(f).replace(/\.mlt$/, ""));

    const active: string[] = [];
    const skipped: [string, string][] = [];

    for (const testName of testNames) {
      const skipReason = this.shouldSkip(testName);
      if (skipReason !== false) {
        skipped.push([testName, skipReason]);
      } else {
        active.push(testName);
      }
    }

    return { active, skipped };
  }
}
