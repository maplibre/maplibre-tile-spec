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
    const skipEntry = skipList.find(
      (entry) => testName === entry || testName.startsWith(`${entry}/`),
    );
    if (skipEntry !== undefined) {
      matched.add(skipEntry);
      skipped.push(testCase);
    } else {
      active.push(testCase);
    }
  }

  const unmatched = skipList.filter((name) => !matched.has(name));
  if (unmatched.length > 0) {
    throw new Error(
      `Exclusion list references unknown synthetic test(s): ${unmatched.join(", ")}. ` +
        `Use the full path-style name, e.g. "0x01/poly_fpf", or a directory, e.g. "0x02".`,
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
    "decoded and matched the expected output - remove it from the exclusion list",
  ).not.toEqual(content);
}

type Position = number[];

function ringArea(ring: Position[]): number {
  let sum = 0;
  for (let i = 0; i + 1 < ring.length; i++) {
    sum += ring[i][0] * ring[i + 1][1] - ring[i + 1][0] * ring[i][1];
  }
  return sum;
}

function windPolygon(rings: Position[][]): Position[][] {
  return rings.map((ring, i) => {
    const area = ringArea(ring);
    return area !== 0 && area > 0 !== (i === 0) ? [...ring].reverse() : ring;
  });
}

function windGeometry<G>(geometry: G): G {
  const g = geometry as {
    type?: string;
    coordinates?: unknown;
    geometries?: unknown[];
  } | null;
  if (g?.type === "Polygon") {
    return {
      ...g,
      coordinates: windPolygon(g.coordinates as Position[][]),
    } as G;
  }
  if (g?.type === "MultiPolygon") {
    return {
      ...g,
      coordinates: (g.coordinates as Position[][][]).map(windPolygon),
    } as G;
  }
  if (g?.type === "GeometryCollection") {
    return { ...g, geometries: g.geometries?.map(windGeometry) } as G;
  }
  return geometry;
}

/**
 * Winds the polygon rings of a GeoJSON feature collection as MVT does: the exterior of each polygon
 * has a positive area and its holes a negative one. MLT v1 lets an encoder wind each ring either
 * way and takes the exterior and the holes from the topology, which GeoJSON keeps in its nesting,
 * so comparing collections after this ignores the winding.
 */
export function normalizeWinding<T extends object>(collection: T): T {
  const { features } = collection as { features?: { geometry?: unknown }[] };
  if (!Array.isArray(features)) {
    return collection;
  }
  return {
    ...collection,
    features: features.map((feature) => ({
      ...feature,
      geometry: windGeometry(feature.geometry),
    })),
  };
}
