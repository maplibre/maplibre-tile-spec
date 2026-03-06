import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { VectorTileLike } from "@maplibre/vt-pbf";
import { bench, describe } from "vitest";
import tsDecodeTile from "../../ts/src/mltDecoder";
import type FeatureTable from "../../ts/src/vector/featureTable";
import { decodeTile as wasmDecodeTile } from "./vectorTile";

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURE_ROOT = resolve(__dirname, "../../../test/expected/tag0x01/omt");

function loadFixture(file: string): Uint8Array {
  return new Uint8Array(readFileSync(resolve(FIXTURE_ROOT, file)));
}

const FIXTURES = [
  { label: "small  (~39 KB)", bytes: loadFixture("11_1062_1368.mlt") },
  { label: "medium (~80 KB)", bytes: loadFixture("10_530_682.mlt") },
  { label: "large  (~763 KB)", bytes: loadFixture("14_8298_10748.mlt") },
] as const;

/**
 * Materialises every feature's geometry and properties from a WASM-decoded
 * tile.  Geometry is decoded via loadGeometry().  Properties are decoded
 * lazily inside MltFeature (memoised with ??=), so reading the getter here
 * forces a WASM boundary crossing and full property deserialisation.
 *
 * Returns the total feature count so the call cannot be eliminated as dead
 * code by the JS engine.
 */
function traverseWasm(tile: VectorTileLike): number {
  let featureCount = 0;
  for (const layer of Object.values(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      feature.loadGeometry();
      void feature.properties;
      featureCount++;
    }
  }
  return featureCount;
}

/**
 * Materialises every feature from a TypeScript-decoded tile.
 * getFeatures() eagerly constructs geometry and properties for all features.
 *
 * Returns the total feature count so the call cannot be eliminated as dead
 * code by the JS engine.
 */
function traverseTs(tables: FeatureTable[]): number {
  let featureCount = 0;
  for (const table of tables) {
    featureCount += table.getFeatures().length;
  }
  return featureCount;
}

const BENCH_OPTIONS = { warmupIterations: 10, time: 2000 } as const;

for (const { label, bytes } of FIXTURES) {
  describe(`decode + full traversal — ${label}`, () => {
    bench(
      "WASM decoder",
      () => {
        const tile = wasmDecodeTile(bytes);
        if (traverseWasm(tile) < 0) throw new Error("unreachable");
      },
      BENCH_OPTIONS,
    );

    bench(
      "TS decoder",
      () => {
        const tables = tsDecodeTile(bytes);
        if (traverseTs(tables) < 0) throw new Error("unreachable");
      },
      BENCH_OPTIONS,
    );
  });
}
