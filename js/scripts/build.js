import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const outDir = path.join(rootDir, "min");
mkdirSync(outDir, { recursive: true });

function findEsbuild() {
  const ext = process.platform === "win32" ? ".exe" : "";
  const archPkg = `${process.platform}-${process.arch}`;
  const candidates = [
    path.resolve(
      rootDir,
      "../rust/mlt-wasm/node_modules/@esbuild",
      archPkg,
      `bin/esbuild${ext}`,
    ),
    path.resolve(
      rootDir,
      "../rust/mlt-wasm/node_modules/@esbuild",
      archPkg,
      `esbuild${ext}`,
    ),
    "esbuild",
  ];
  for (const c of candidates) {
    if (c === "esbuild" || existsSync(c)) return c;
  }
  throw new Error("esbuild binary not found");
}

const esbuild = findEsbuild();
const targets = [
  ["src/index.js", "min/index.js"],
  ["src/columns.js", "min/columns.js"],
];

for (const [entry, outRel] of targets) {
  const inFile = path.join(rootDir, entry);
  const outFile = path.join(rootDir, outRel);
  execFileSync(
    esbuild,
    [
      inFile,
      "--bundle",
      "--format=esm",
      "--target=es2022",
      "--minify",
      "--mangle-props=/^_/",
      `--outfile=${outFile}`,
    ],
    { stdio: "inherit" },
  );
  const raw = readFileSync(outFile);
  const gz = gzipSync(raw, { level: 9 });
  const br = brotliCompressSync(raw, {
    params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
  });
  console.log(
    `${outRel.padEnd(16)} | raw: ${(raw.length / 1024).toFixed(2).padStart(6)} KB (${raw.length} B) | gzip: ${(gz.length / 1024).toFixed(2).padStart(6)} KB (${gz.length} B) | brotli: ${(br.length / 1024).toFixed(2).padStart(6)} KB (${br.length} B)`,
  );
}
