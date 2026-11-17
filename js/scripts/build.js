import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const outDir = path.join(rootDir, "dist");
mkdirSync(outDir, { recursive: true });

function findEsbuild() {
  const ext = process.platform === "win32" ? ".exe" : "";
  const archPkg = `${process.platform}-${process.arch}`;
  const roots = [
    path.join(rootDir, "node_modules/@esbuild", archPkg),
    path.resolve(rootDir, "../rust/mlt-wasm/node_modules/@esbuild", archPkg),
  ];
  const candidates = [];
  for (const r of roots) {
    candidates.push(path.join(r, `bin/esbuild${ext}`));
    candidates.push(path.join(r, `esbuild${ext}`));
  }
  candidates.push("esbuild");
  for (const c of candidates) {
    if (c === "esbuild" || existsSync(c)) return c;
  }
  throw new Error("esbuild binary not found");
}

const esbuild = findEsbuild();
const targets = [
  ["src/index.js", "dist/index.js", "src/index.d.ts", "dist/index.d.ts"],
  ["src/columns.js", "dist/columns.js", "src/columns.d.ts", "dist/columns.d.ts"],
];

for (const [entry, outRel, dtsSrcRel, dtsOutRel] of targets) {
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
  copyFileSync(path.join(rootDir, dtsSrcRel), path.join(rootDir, dtsOutRel));

  const raw = readFileSync(outFile);
  const gz = gzipSync(raw, { level: 9 });
  const br = brotliCompressSync(raw, {
    params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
  });
  console.log(
    `${outRel.padEnd(16)} | raw: ${(raw.length / 1024).toFixed(2).padStart(6)} KB (${raw.length} B) | gzip: ${(gz.length / 1024).toFixed(2).padStart(6)} KB (${gz.length} B) | brotli: ${(br.length / 1024).toFixed(2).padStart(6)} KB (${br.length} B)`,
  );
}
