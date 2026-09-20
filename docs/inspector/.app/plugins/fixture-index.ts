/** Indexes the symlinked synthetic fixtures and serves only the tiles it indexed. */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import type { FixtureEntry } from "../src/fixtures.ts";

/** The four directories of synthetic tiles, which are disjoint sets rather than re-encodings of one another. */
export const FIXTURE_DIRECTORIES = ["0x01", "0x01-rust", "0x02", "0x02-java"];

/** Scans the fixture directories for tiles, ordered by directory then by name. */
export function indexFixtures(root: string): FixtureEntry[] {
  return FIXTURE_DIRECTORIES.flatMap((directory) =>
    readdirSync(join(root, directory))
      .filter((name) => name.endsWith(".mlt"))
      .sort()
      .map((name) => ({
        name,
        directory,
        bytes: statSync(join(root, directory, name)).size,
      })),
  );
}

export function fixtureIndex(): Plugin {
  let root = "";
  let index: FixtureEntry[] = [];
  let isBuild = false;

  const tilePath = (key: string) => join(root, key);
  const served = () =>
    new Set(index.map((entry) => `${entry.directory}/${entry.name}`));

  return {
    name: "mlt-fixture-index",

    configResolved(config) {
      root = join(config.root, "fixtures");
      index = indexFixtures(root);
      isBuild = config.command === "build";
    },

    buildStart() {
      if (!isBuild) return;
      this.emitFile({
        type: "asset",
        fileName: "fixtures.json",
        source: `${JSON.stringify(index, null, 2)}\n`,
      });
      for (const key of served()) {
        this.emitFile({
          type: "asset",
          fileName: `fixtures/${key}`,
          source: readFileSync(tilePath(key)),
        });
      }
    },

    configureServer(server) {
      const keys = served();
      server.middlewares.use((request, response, next) => {
        const path = (request.url ?? "").split("?")[0].replace(/^\//, "");
        if (path === "fixtures.json") {
          response.setHeader("Content-Type", "application/json");
          response.end(`${JSON.stringify(index, null, 2)}\n`);
          return;
        }
        const key = path.startsWith("fixtures/")
          ? path.slice("fixtures/".length)
          : "";
        if (!keys.has(key)) {
          next();
          return;
        }
        response.setHeader("Content-Type", "application/octet-stream");
        response.end(readFileSync(tilePath(key)));
      });
    },
  };
}
