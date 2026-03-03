import { glob, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import JSON5 from "json5";
import { expect } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));

export const syntheticTestDir = resolve(__dirname, "../0x01");

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

export type SyntheticCaseResult =
  | {
      status: "ok";
      testName: string;
      name: string;
    }
  | {
      status: "skip";
      testName: string;
      name: string;
      reason: string;
    }
  | {
      status: "fail";
      testName: string;
      name: string;
      error: unknown;
    }
  | {
      status: "crash";
      testName: string;
      name: string;
      reason: string;
      error: unknown;
    };

export class SyntheticTestRunner implements AsyncIterable<SyntheticCaseResult> {
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
    console.log(`wrote actual output to ${actualFile}`);
    return actualFile;
  }

  decodeMLT(_mltFilePath: string): Promise<Record<string, unknown>> {
    throw new Error("not implemented");
  }

  private async runCase(
    syntheticDir: string,
    testName: string,
  ): Promise<SyntheticCaseResult> {
    const mltFile = join(syntheticDir, `${testName}.mlt`);
    const name = `${testName}.mlt`;
    const jsonFile = join(syntheticDir, `${testName}.json`);

    const skipReason = this.shouldSkip(testName);
    if (skipReason !== false) {
      return {
        status: "skip",
        testName,
        name,
        reason: skipReason,
      };
    }

    let actual: Record<string, unknown>;
    try {
      actual = await this.decodeMLT(mltFile);
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      return {
        status: "crash",
        testName,
        name,
        reason,
        error,
      };
    }

    const expectedRaw = await readFile(jsonFile, "utf-8");
    const expected = JSON5.parse(expectedRaw);

    try {
      expect(actual).toEqual(expected);
      return {
        status: "ok",
        testName,
        name,
      };
    } catch (error) {
      const _actualFile = await this.writeActualOutput(mltFile, actual);
      return {
        status: "fail",
        testName,
        name,
        error,
      };
    }
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<SyntheticCaseResult> {
    const files = await collectGlob(join(syntheticTestDir, "*.mlt"));
    const names = files.map((fileName) =>
      basename(fileName).replace(/\.mlt$/, ""),
    );

    for (const testName of names) {
      yield await this.runCase(syntheticTestDir, testName);
    }
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
