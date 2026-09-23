import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FIXTURE_DIRECTORIES, indexFixtures } from "./fixture-index.ts";

const index = indexFixtures(join(import.meta.dirname, "..", "fixtures"));

describe("indexFixtures", () => {
  it("indexes every synthetic tile", () => {
    expect(index).toHaveLength(1223);
  });

  it("indexes each directory", () => {
    const counts = Object.fromEntries(
      FIXTURE_DIRECTORIES.map((directory) => [
        directory,
        index.filter((entry) => entry.directory === directory).length,
      ]),
    );
    expect(counts).toEqual({
      "0x01": 366,
      "0x01-rust": 194,
      "0x02": 528,
      "0x01-simple": 6,
      "0x01-omt": 95,
      "0x01-bing": 17,
      "0x01-amazon": 11,
      "0x01-amazon_here": 5,
      "0x01-osm": 1,
    });
  });

  it("indexes tiles only", () => {
    expect(index.filter((entry) => !entry.name.endsWith(".mlt"))).toEqual([]);
  });

  it("records a non-zero size for every tile", () => {
    expect(index.filter((entry) => entry.bytes <= 0)).toEqual([]);
  });

  it("reads geometry and encoding facets from mlt ls", () => {
    expect(
      index.find(
        (entry) =>
          entry.directory === "0x02" && entry.name === "props_str_fsst.mlt",
      ),
    ).toEqual({
      name: "props_str_fsst.mlt",
      directory: "0x02",
      bytes: 230,
      geometries: ["Point"],
      encodings: ["FSST", "Dictionary"],
    });
  });

  it("orders by directory then by name", () => {
    const keys = index.map((entry) => `${entry.directory}/${entry.name}`);
    const ordered = FIXTURE_DIRECTORIES.flatMap((directory) =>
      keys.filter((key) => key.startsWith(`${directory}/`)).sort(),
    );
    expect(keys).toEqual(ordered);
  });
});
