import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { decodeTileColumns } from "../dist/columns.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function collectMlt(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...collectMlt(p));
    else if (e.isFile() && e.name.endsWith(".mlt")) {
      out.push(new Uint8Array(readFileSync(p)));
    }
  }
  return out;
}

function forceGc() {
  if (typeof globalThis.gc === "function") {
    globalThis.gc();
    globalThis.gc();
  }
}

function measureWorkload(name, tiles, iterations) {
  // Warmup JIT
  for (let w = 0; w < Math.min(20, iterations); w++) {
    for (const t of tiles) decodeTileColumns(t);
  }

  forceGc();
  const memBefore = process.memoryUsage();

  // Measure single-iteration allocation delta without GC in between
  let lastResult = null;
  for (const t of tiles) {
    lastResult = decodeTileColumns(t);
  }
  const memAfterOne = process.memoryUsage();
  const allocDeltaKB = (memAfterOne.heapUsed - memBefore.heapUsed) / 1024;

  // Measure sustained throughput
  const t0 = performance.now();
  let layerCount = 0;
  for (let i = 0; i < iterations; i++) {
    for (const t of tiles) {
      layerCount += decodeTileColumns(t).layers.length;
    }
  }
  const elapsedMs = performance.now() - t0;

  // Measure retained memory after releasing decoded results + GC
  lastResult = null;
  forceGc();
  const memRetained = process.memoryUsage();
  const retainedDeltaKB = Math.max(0, (memRetained.heapUsed - memBefore.heapUsed) / 1024);

  const totalDecodes = iterations * tiles.length;
  const totalBytes =
    iterations * tiles.reduce((sum, t) => sum + t.byteLength, 0);
  const msPerTile = elapsedMs / totalDecodes;
  const mbPerSec = totalBytes / (1024 * 1024) / (elapsedMs / 1000);

  console.log(
    `${name.padEnd(28)} | ${msPerTile.toFixed(3).padStart(8)} ms/tile | ${mbPerSec.toFixed(1).padStart(7)} MB/s | alloc ${allocDeltaKB.toFixed(1).padStart(8)} KB | retained ${retainedDeltaKB.toFixed(1).padStart(6)} KB (layers=${layerCount})`,
  );
}

const workloads = [];

const meshFixture16 = path.resolve(
  __dirname,
  "../../test/fixtures/meshes/tiles/16/34588/21180.mlt",
);
const meshFixture15 = path.resolve(
  __dirname,
  "../../test/fixtures/meshes/tiles/15/17294/10590.mlt",
);
workloads.push({
  name: "CityGML mesh fixture z16",
  tiles: [new Uint8Array(readFileSync(meshFixture16))],
  iterations: 2000,
});
workloads.push({
  name: "CityGML mesh fixture z15",
  tiles: [new Uint8Array(readFileSync(meshFixture15))],
  iterations: 2000,
});

const hamburgTemp = "C:\\Users\\thur_mat\\AppData\\Local\\Temp\\mlt-mesh-run\\hamburg_v7_tiles";
if (existsSync(hamburgTemp)) {
  const hhTiles = [
    "16/34584/21177.mlt",
    "15/17292/10588.mlt",
    "14/8646/5294.mlt",
    "13/4323/2647.mlt",
  ]
    .map((rel) => path.join(hamburgTemp, rel))
    .filter((p) => existsSync(p));
  for (const p of hhTiles) {
    const z = path.basename(path.dirname(path.dirname(p)));
    workloads.push({
      name: `Hamburg 3D tile z${z} (${(readFileSync(p).length / 1024).toFixed(0)}KB)`,
      tiles: [new Uint8Array(readFileSync(p))],
      iterations: 200,
    });
  }
}

const osmXlarge = path.resolve(__dirname, "../../test/fixtures/osm/xlarge.mlt");
if (existsSync(osmXlarge)) {
  workloads.push({
    name: `OSM 2D xlarge (${(readFileSync(osmXlarge).length / 1024).toFixed(0)}KB)`,
    tiles: [new Uint8Array(readFileSync(osmXlarge))],
    iterations: 60,
  });
}

const synthetics = collectMlt(path.resolve(__dirname, "../../test/synthetic"));
workloads.push({
  name: `All ${synthetics.length} synthetics`,
  tiles: synthetics,
  iterations: 10,
});

console.log(
  "Workload                     |     Time / tile |  Speed  | 1-pass Heap Δ  | Retained Δ",
);
console.log("-".repeat(92));
for (const w of workloads) {
  measureWorkload(w.name, w.tiles, w.iterations);
}
