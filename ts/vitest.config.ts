import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
    test: {
        coverage: {
            reportOnFailure: true,
            // Cobertura file names are relative to `projectRoot`, and must be repository-relative for GitHub code coverage
            reporter: [["text"], ["cobertura", { projectRoot: path.resolve(import.meta.dirname, "..") }]],
        },
    },
});
