import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "./",
  plugins: [vue()],
  build: {
    outDir: "../app",
    emptyOutDir: true,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.spec.ts"],
  },
});
