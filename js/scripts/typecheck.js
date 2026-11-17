import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");

const tscCandidates = [
  path.join(rootDir, "node_modules/typescript/bin/tsc"),
  path.resolve(rootDir, "../rust/mlt-wasm/node_modules/typescript/bin/tsc"),
];

const tscScript = tscCandidates.find((p) => existsSync(p));
if (tscScript) {
  execFileSync(
    process.execPath,
    [tscScript, "-p", path.join(rootDir, "tsconfig.json")],
    { stdio: "inherit" },
  );
} else {
  execFileSync("tsc", ["-p", path.join(rootDir, "tsconfig.json")], {
    stdio: "inherit",
    shell: process.platform === "win32",
  });
}
console.log("TypeScript declarations verified (strict NodeNext).");
