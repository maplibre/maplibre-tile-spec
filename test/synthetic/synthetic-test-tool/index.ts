import { glob, readFile } from "node:fs/promises";
import { basename, join } from "node:path";
import JSON5 from "json5";

async function collectGlob(pattern: string): Promise<string[]> {
    const result: string[] = [];
    for await (const f of glob(pattern)) {
        result.push(f);
    }
    return result.sort();
}

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

function numbersEqualWithTolerance(expected: number, actual: number): boolean {
    if (!Number.isFinite(expected)) return Object.is(expected, actual);
    if (Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
        return Math.abs(actual) <= ABSOLUTE_FLOAT_TOLERANCE;
    }
    const relativeError = Math.abs(actual - expected) / Math.abs(expected);
    return relativeError <= RELATIVE_FLOAT_TOLERANCE;
}

export function deepEqualWithTolerance(expected: unknown, actual: unknown, path = ""): void {
    const loc = path || "root";

    if (typeof expected === "number" && typeof actual === "number") {
        if (!numbersEqualWithTolerance(expected, actual)) {
            throw new Error(`Mismatch at ${loc}: expected ${expected}, got ${actual}`);
        }
        return;
    }

    if (expected === null || actual === null || expected === undefined || actual === undefined) {
        if (expected !== actual) {
            throw new Error(`Mismatch at ${loc}: expected ${expected}, got ${actual}`);
        }
        return;
    }

    if (typeof expected !== typeof actual) {
        throw new Error(`Type mismatch at ${loc}: expected ${typeof expected}, got ${typeof actual}`);
    }

    if (Array.isArray(expected)) {
        if (!Array.isArray(actual)) {
            throw new Error(`Expected array at ${loc}, got ${typeof actual}`);
        }
        if (expected.length !== actual.length) {
            throw new Error(`Array length mismatch at ${loc}: expected ${expected.length}, got ${actual.length}`);
        }
        for (let i = 0; i < expected.length; i++) {
            deepEqualWithTolerance(expected[i], (actual as unknown[])[i], `${path}[${i}]`);
        }
        return;
    }

    if (typeof expected === "object") {
        const expectedKeys = Object.keys(expected as object).sort();
        const actualKeys = Object.keys(actual as object).sort();
        if (JSON.stringify(expectedKeys) !== JSON.stringify(actualKeys)) {
            throw new Error(`Key mismatch at ${loc}: expected [${expectedKeys}], got [${actualKeys}]`);
        }
        for (const key of expectedKeys) {
            deepEqualWithTolerance(
                (expected as Record<string, unknown>)[key],
                (actual as Record<string, unknown>)[key],
                path ? `${path}.${key}` : key
            );
        }
        return;
    }

    if (expected !== actual) {
        throw new Error(`Mismatch at ${loc}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}

export class SyntheticTestRunner {
    shouldSkip(_testName: string): false | string {
        return false;
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
                deepEqualWithTolerance(expected, actual);
                console.log(`OK - ${name}`);
                passed++;
            } catch (err) {
                console.log(`FAIL - ${name}`);
                console.error(
                    "expected:\n",
                    JSON5.stringify(expected, null, 2),
                    "\nactual:\n",
                    JSON5.stringify(actual, null, 2)
                );
                failed++;
            }
        }

        console.log(`\n${passed} passed, ${failed} failed, ${skipped} skipped, ${mltFiles.length} total`);
        if (failed > 0) throw new Error(`${failed} test(s) failed`);
    }

    async getTestCases(syntheticDir: string): Promise<{ active: string[]; skipped: [string, string][] }> {
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
