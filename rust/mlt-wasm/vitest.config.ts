import path from "node:path";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import { coverageConfigDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  test: {
    // Dumping the wasm coverage counters needs a build that exports them, see `just rust::wasm-coverage`
    setupFiles: process.env.MLT_WASM_PROFRAW_DIR
      ? ["./js/coverage.setup.ts"]
      : [],
    coverage: {
      reportOnFailure: true,
      // pkg/ is wasm-bindgen's generated glue, js/coverage.setup.ts only runs under `wasm-coverage`
      exclude: [
        ...coverageConfigDefaults.exclude,
        "pkg/**",
        "js/coverage.setup.ts",
      ],
      // Cobertura file names are relative to `projectRoot`, and must be repository-relative for GitHub code coverage
      reporter: [
        ["text"],
        [
          "cobertura",
          { projectRoot: path.resolve(import.meta.dirname, "../..") },
        ],
      ],
    },
  },
});
