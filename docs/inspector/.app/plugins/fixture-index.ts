/** Indexes the symlinked synthetic fixtures and serves only the tiles it indexed. */

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import type { FixtureEntry } from "../src/fixtures.ts";

/** Coarse encoding names, keyed by the substring `mlt ls` spells them with. */
const ENCODINGS: [needle: string, label: string][] = [
  ["FastPFOR", "FastPFOR"],
  ["BitPacked", "BitPacked"],
  ["Rle", "RLE"],
  ["Morton", "Morton"],
  ["Fsst", "FSST"],
  ["Shared", "SharedDict"],
  ["Dict", "Dictionary"],
  ["Present", "Presence"],
  ["Triangles", "Tessellated"],
];

interface LsRow {
  path: string;
  info?: { geometries?: string[]; algorithms?: string[] };
}

/**
 * Geometry and encoding facets per tile, read from `mlt ls`.
 * The binary is optional: without it the picker simply has fewer buttons.
 */
function readFacets(root: string): Map<string, Partial<FixtureEntry>> {
  const facets = new Map<string, Partial<FixtureEntry>>();
  const binary = join(root, "../../../../rust/target/release/mlt");
  if (!existsSync(binary)) return facets;
  let rows: LsRow[];
  try {
    const out = execFileSync(
      binary,
      [
        "ls",
        "--format",
        "json",
        ...FIXTURE_DIRECTORIES.map((d) => join(root, d)),
      ],
      { encoding: "utf8", maxBuffer: 64 << 20 },
    );
    rows = JSON.parse(out) as LsRow[];
  } catch {
    return facets;
  }
  for (const row of rows) {
    if (!row.info) continue;
    const algorithms = (row.info.algorithms ?? []).join(" ");
    facets.set(row.path.split("/").slice(-2).join("/"), {
      geometries: row.info.geometries ?? [],
      encodings: ENCODINGS.flatMap(([needle, label]) =>
        algorithms.includes(needle) ? [label] : [],
      ),
    });
  }
  return facets;
}

/** The three directories of synthetic tiles, which are disjoint sets rather than re-encodings of one another. */
export const FIXTURE_DIRECTORIES = ["0x01", "0x01-rust", "0x02"];

/** Scans the fixture directories for tiles, ordered by directory then by name. */
export function indexFixtures(root: string): FixtureEntry[] {
  const facets = readFacets(root);
  return FIXTURE_DIRECTORIES.flatMap((directory) =>
    readdirSync(join(root, directory))
      .filter((name) => name.endsWith(".mlt"))
      .sort()
      .map((name) => ({
        name,
        directory,
        bytes: statSync(join(root, directory, name)).size,
        ...facets.get(`${directory}/${name}`),
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
