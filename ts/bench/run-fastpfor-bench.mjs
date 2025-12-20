import { spawnSync } from "node:child_process";

const userArgs = process.argv.slice(2);
const env = { ...process.env };

const includeBig = userArgs.includes("--big") || env.BENCH_BIG === "1";
if (includeBig) env.BENCH_BIG = "1";

const filteredArgs = userArgs.filter((a) => a !== "--big");

const isWindows = process.platform === "win32";
const npmCmd = isWindows ? "npm.cmd" : "npm";

const result = spawnSync(
    npmCmd,
    ["exec", "--", "vitest", "run", "bench/fastpfor.bench.spec.ts", ...filteredArgs],
    { stdio: "inherit", env },
);

process.exit(result.status ?? 1);

