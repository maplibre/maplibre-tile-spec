import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";
import { fixtureIndex } from "./plugins/fixture-index.ts";

export default defineConfig({
  base: "./",
  plugins: [vue(), fixtureIndex()],
  build: {
    outDir: "../app",
    emptyOutDir: true,
  },
  server: {
    // The fixture plugin serves the tiles it indexed; everything else beside them stays unreachable, as in a build.
    fs: { deny: ["**/fixtures/**"] },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.spec.ts", "plugins/**/*.spec.ts"],
  },
});
