import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { afterAll } from "vitest";
import { coverageDump } from "../pkg/mlt_wasm.js";

const dir = process.env.MLT_WASM_PROFRAW_DIR as string;

afterAll(() => {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, `${crypto.randomUUID()}.profraw`), coverageDump());
});
