import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { decodeTileColumns as jsDecode } from "../min/columns.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wasmPkgPath = path.resolve(__dirname, "../../rust/mlt-wasm/pkg/mlt_wasm.js");

let wasmDecode = null;
if (existsSync(wasmPkgPath)) {
  const wasmMod = await import(`file:///${wasmPkgPath.replaceAll("\\", "/")}`);
  wasmDecode = wasmMod.decodeTileColumns;
}

const cases = [
  [
    "CityGML fixture z16",
    path.resolve(__dirname, "../../test/fixtures/meshes/tiles/16/34588/21180.mlt"),
    3000,
  ],
  [
    "CityGML fixture z15",
    path.resolve(__dirname, "../../test/fixtures/meshes/tiles/15/17294/10590.mlt"),
    3000,
  ],
  [
    "Hamburg z16 (88 KB)",
    "C:\\Users\\thur_mat\\AppData\\Local\\Temp\\mlt-mesh-run\\hamburg_v7_tiles\\16\\34584\\21177.mlt",
    500,
  ],
  [
    "Hamburg z15 (61 KB)",
    "C:\\Users\\thur_mat\\AppData\\Local\\Temp\\mlt-mesh-run\\hamburg_v7_tiles\\15\\17292\\10588.mlt",
    500,
  ],
  [
    "Hamburg z14 (255 KB)",
    "C:\\Users\\thur_mat\\AppData\\Local\\Temp\\mlt-mesh-run\\hamburg_v7_tiles\\14\\8646\\5294.mlt",
    300,
  ],
  [
    "Hamburg z13 (196 KB)",
    "C:\\Users\\thur_mat\\AppData\\Local\\Temp\\mlt-mesh-run\\hamburg_v7_tiles\\13\\4323\\2647.mlt",
    300,
  ],
].filter(([, p]) => existsSync(p));

console.log(
  "Workload               | Pure JS (ms) | JS (MB/s) | Rust WASM (ms) | WASM (MB/s) | Speedup vs WASM",
);
console.log("-".repeat(94));

for (const [label, filePath, iters] of cases) {
  const bytes = new Uint8Array(readFileSync(filePath));
  const mb = bytes.byteLength / (1024 * 1024);

  // Interleaved warmup + multi-round median to eliminate turbo/thermal drift
  for (let w = 0; w < 50; w++) {
    jsDecode(bytes);
    if (wasmDecode) wasmDecode(bytes);
  }

  const rounds = 5;
  const jsTimes = [];
  const wasmTimes = [];
  const perRound = Math.max(20, Math.floor(iters / rounds));

  for (let r = 0; r < rounds; r++) {
    const j0 = performance.now();
    for (let i = 0; i < perRound; i++) jsDecode(bytes);
    jsTimes.push((performance.now() - j0) / perRound);

    if (wasmDecode) {
      const w0 = performance.now();
      for (let i = 0; i < perRound; i++) wasmDecode(bytes);
      wasmTimes.push((performance.now() - w0) / perRound);
    }
  }

  jsTimes.sort((a, b) => a - b);
  wasmTimes.sort((a, b) => a - b);
  const jsMs = jsTimes[rounds >> 1];
  const wasmMs = wasmTimes[rounds >> 1];
  const jsMbs = mb / (jsMs / 1000);
  const wasmMbs = mb / (wasmMs / 1000);
  const speedup = wasmMs / jsMs;

  console.log(
    `${label.padEnd(22)} | ${jsMs.toFixed(3).padStart(12)} | ${jsMbs.toFixed(1).padStart(9)} | ${wasmMs.toFixed(3).padStart(14)} | ${wasmMbs.toFixed(1).padStart(11)} | ${speedup.toFixed(2)}x (${speedup >= 1 ? "JS FASTER" : "WASM faster"})`,
  );
}
