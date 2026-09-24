/** Indexes the synthetic and real-world fixtures and serves only the tiles it indexed. */

import { execFileSync } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  realpathSync,
  statSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import type { Plugin } from "vite";
import type { FixtureEntry } from "../src/fixtures.ts";

/** The axes `mlt ls --format json` reports, each a sorted array of spec names. */
type LsFacets = Record<string, string[]>;

/**
 * The coarse encoding names the facet bar shows, each read off the axes that
 * actually carry it rather than matched against one concatenated label.
 *
 * `Presence` holds only for v1: v2 stores a presence field as a bare bitfield with
 * no stream header, so it never appears as a stream type. Giving v2 a presence facet
 * needs `mlt ls` to report the coding, which its parser currently discards.
 */
const ENCODINGS: [name: string, of: (f: LsFacets) => boolean][] = [
  ["FastPFOR", (f) => has(f.physical, (v) => v.startsWith("fastpfor"))],
  ["BitPacked", (f) => has(f.physical, (v) => v === "bitpacked")],
  ["RLE", (f) => has(f.logical, (v) => v.endsWith("rle"))],
  ["Morton", (f) => has(f.logical, (v) => v.startsWith("morton"))],
  ["FSST", (f) => has(f.strLayout, (v) => v.startsWith("fsst"))],
  ["SharedDict", (f) => has(f.streamType, (v) => v === "data[shared]")],
  [
    "Dictionary",
    (f) =>
      has(f.strLayout, (v) => v.endsWith("dict")) ||
      has(f.logical, (v) => v === "dict"),
  ],
  ["Presence", (f) => has(f.streamType, (v) => v === "present")],
  ["Tessellated", (f) => has(f.geomLayout, (v) => v.startsWith("tess"))],
];

const has = (values: string[] | undefined, hit: (v: string) => boolean) =>
  (values ?? []).some(hit);

interface LsRow {
  path: string;
  info?: {
    geometries?: string[];
    content?: string[];
    facets?: LsFacets;
  };
}

/**
 * Geometry and encoding facets per tile, read from `mlt ls`.
 * `just inspector::build` builds the binary, and without it the picker simply has fewer buttons.
 */
function readFacets(
  dirs: { directory: string; path: string }[],
): Map<string, Partial<FixtureEntry>> {
  const facets = new Map<string, Partial<FixtureEntry>>();
  if (dirs.length === 0) return facets;
  const binary = join(dirs[0].path, "../../../rust/target/release/mlt");
  if (!existsSync(binary)) return facets;
  const label = new Map(dirs.map((d) => [d.path, d.directory]));
  // A tile ls cannot read exits non-zero while still printing a row per file, so the
  // output is read either way rather than dropping every other tile's facets with it.
  let rows: LsRow[];
  try {
    const out = execFileSync(
      binary,
      [
        "ls",
        "--format",
        "json",
        "--details",
        "algorithms",
        ...dirs.map((d) => d.path),
      ],
      { encoding: "utf8", maxBuffer: 256 << 20 },
    );
    rows = JSON.parse(out) as LsRow[];
  } catch (cause) {
    const out = (cause as { stdout?: string }).stdout;
    if (out === undefined) return facets;
    try {
      rows = JSON.parse(out) as LsRow[];
    } catch {
      return facets;
    }
  }
  for (const row of rows) {
    const directory = label.get(dirname(row.path));
    if (!row.info || directory === undefined) continue;
    const axes = row.info.facets ?? {};
    facets.set(`${directory}/${basename(row.path)}`, {
      geometries: axes.geometry ?? row.info.geometries ?? [],
      encodings: ENCODINGS.flatMap(([name, of]) => (of(axes) ? [name] : [])),
      content: axes.dataType ?? row.info.content ?? [],
      facets: axes,
    });
  }
  return facets;
}

/**
 * Every corpus the picker offers: the prefix its keys carry, and where the tiles are,
 * relative to `test/synthetic` which the `fixtures` symlink points at.
 * The synthetic sets are disjoint rather than re-encodings of one another; the rest are
 * real-world tiles the Java encoder wrote into `test/expected`.
 */
export const FIXTURE_SOURCES: { directory: string; from: string }[] = [
  { directory: "0x01", from: "0x01" },
  { directory: "0x01-rust", from: "0x01-rust" },
  { directory: "0x02", from: "0x02" },
  { directory: "0x01-simple", from: "../expected/0x01/simple" },
  { directory: "0x01-omt", from: "../expected/0x01/omt" },
  { directory: "0x01-bing", from: "../expected/0x01/bing" },
  { directory: "0x01-amazon", from: "../expected/0x01/amazon" },
  { directory: "0x01-amazon_here", from: "../expected/0x01/amazon_here" },
  { directory: "0x01-osm", from: "../fixtures/osm" },
];

/** Directory names, which is all the picker's grouping needs. */
export const FIXTURE_DIRECTORIES = FIXTURE_SOURCES.map((s) => s.directory);

/** The index plus where each key's bytes actually live, which is outside `fixtures` for most. */
export function scanFixtures(root: string): {
  index: FixtureEntry[];
  paths: Map<string, string>;
} {
  // `fixtures` is a symlink, and join() is lexical, so `../expected` needs the real path.
  const base = realpathSync(root);
  const dirs = FIXTURE_SOURCES.map((source) => ({
    directory: source.directory,
    path: join(base, source.from),
  })).filter((source) => existsSync(source.path));
  const facets = readFacets(dirs);
  const paths = new Map<string, string>();
  const index = dirs.flatMap(({ directory, path }) =>
    readdirSync(path)
      .filter((name) => name.endsWith(".mlt"))
      .sort()
      .map((name) => {
        const key = `${directory}/${name}`;
        paths.set(key, join(path, name));
        return {
          name,
          directory,
          bytes: statSync(join(path, name)).size,
          ...facets.get(key),
        };
      }),
  );
  return { index, paths };
}

/** Scans the fixture directories for tiles, ordered by directory then by name. */
export function indexFixtures(root: string): FixtureEntry[] {
  return scanFixtures(root).index;
}

export function fixtureIndex(): Plugin {
  let index: FixtureEntry[] = [];
  let paths = new Map<string, string>();
  let isBuild = false;

  const served = () => new Set(paths.keys());

  return {
    name: "mlt-fixture-index",

    configResolved(config) {
      ({ index, paths } = scanFixtures(join(config.root, "fixtures")));
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
          source: readFileSync(paths.get(key) as string),
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
        response.end(readFileSync(paths.get(key) as string));
      });
    },
  };
}
