import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { VectorTileLike } from "@maplibre/vt-pbf";
import { beforeAll, bench, describe } from "vitest";
import tsDecodeTile from "../../ts/src/mltDecoder";
import type FeatureTable from "../../ts/src/vector/featureTable";
import { decodeTile as wasmDecodeTile } from "./vectorTile";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OMT = resolve(__dirname, "../../../test/expected/tag0x01/omt");

function loadPool(zoom: number): Uint8Array[] {
  return readdirSync(OMT)
    .filter((f) => f.startsWith(`${zoom}_`) && f.endsWith(".mlt"))
    .sort()
    .map((f) => new Uint8Array(readFileSync(resolve(OMT, f))));
}

// zoom 11: 12 tiles, 39–89 KB each  (~760 KB total — exceeds typical L2)
// zoom 10: 12 tiles, 70–128 KB each (~1.1 MB total)
// zoom 14: 10 tiles, 344–763 KB each (~5.5 MB total — exceeds most L3s)
const POOLS = [
  { label: "small  (zoom 11, 39–89 KB)", pool: loadPool(11) },
  { label: "medium (zoom 10, 70–128 KB)", pool: loadPool(10) },
  { label: "large  (zoom 14, 344–763 KB)", pool: loadPool(14) },
];

function traverseWasm(tile: VectorTileLike): number {
  let n = 0;
  for (const layer of Object.values(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
      const f = layer.feature(i);
      void f.properties;
      f.loadGeometry();
      n++;
    }
  }
  return n;
}

function traverseTs(tables: FeatureTable[]): number {
  let n = 0;
  for (const table of tables) {
    const geometries = table.geometryVector.getGeometries();
    for (let i = 0; i < table.numFeatures; i++) {
      void geometries[i];
      for (const col of table.propertyVectors) {
        if (col) col.getValue(i);
      }
      n++;
    }
  }
  return n;
}

// Requires Node to be started with --expose-gc (done by the bench npm script); no-op otherwise.
function drainGC(): void {
  const _gc = (globalThis as { gc?: () => void }).gc;
  if (typeof _gc === "function") {
    _gc();
    _gc();
  }
}

const OPTIONS = {
  warmupTime: 500,
  time: 2000,
  minSamples: 40,
} as const;

for (const { label, pool } of POOLS) {
  describe(`decode + traverse - ${label}`, () => {
    let ti = 0;
    let wi = 0;

    // Drain heap garbage left by the previous describe block so GC does not
    // fire at an unpredictable point inside this block's measurement window.
    beforeAll(drainGC);

    bench(
      "TS decoder",
      () => {
        const tables = tsDecodeTile(pool[ti++ % pool.length]);
        if (traverseTs(tables) < 0) throw new Error("unreachable");
      },
      OPTIONS,
    );

    bench(
      "WASM decoder",
      () => {
        const tile = wasmDecodeTile(pool[wi++ % pool.length]);
        if (traverseWasm(tile) < 0) throw new Error("unreachable");
      },
      OPTIONS,
    );
  });
}
