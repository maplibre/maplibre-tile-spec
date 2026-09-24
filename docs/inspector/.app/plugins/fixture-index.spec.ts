import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FIXTURE_DIRECTORIES, indexFixtures } from "./fixture-index.ts";

const index = indexFixtures(join(import.meta.dirname, "..", "fixtures"));

describe("indexFixtures", () => {
  it("indexes every synthetic tile", () => {
    expect(index.length).toMatchInlineSnapshot(`1262`);
  });

  it("indexes each directory", () => {
    const counts = Object.fromEntries(
      FIXTURE_DIRECTORIES.map((directory) => [
        directory,
        index.filter((entry) => entry.directory === directory).length,
      ]),
    );
    expect(counts).toMatchInlineSnapshot(`
      {
        "0x01": 366,
        "0x01-amazon": 11,
        "0x01-amazon_here": 5,
        "0x01-bing": 17,
        "0x01-omt": 95,
        "0x01-osm": 1,
        "0x01-rust": 194,
        "0x01-simple": 6,
        "0x02": 567,
      }
    `);
  });

  it("indexes tiles only", () => {
    expect(index.filter((entry) => !entry.name.endsWith(".mlt"))).toEqual([]);
  });

  it("records a non-zero size for every tile", () => {
    expect(index.filter((entry) => entry.bytes <= 0)).toEqual([]);
  });

  it("reads geometry and encoding facets from mlt ls", () => {
    const entry = index.find(
      (entry) =>
        entry.directory === "0x02" && entry.name === "props_str_fsst.mlt",
    );
    // Not `Dictionary`: an FSST-plain column has a symbol table but no value
    // dictionary. The old substring match read one out of the `length[dictionary]`
    // stream's name, which is what reading the axes separately avoids.
    expect(entry?.encodings).toEqual(["FSST"]);
    expect(entry).toMatchInlineSnapshot(`
      {
        "bytes": 230,
        "content": [
          "str",
        ],
        "directory": "0x02",
        "encodings": [
          "FSST",
        ],
        "facets": {
          "dataType": [
            "str",
          ],
          "dictLayout": [],
          "extent": [
            "64",
          ],
          "geomLayout": [
            "points",
          ],
          "geometry": [
            "point",
          ],
          "logical": [
            "componentwise-delta",
          ],
          "physical": [
            "varint",
          ],
          "strLayout": [
            "fsst",
          ],
          "streamType": [
            "data[fsst]",
            "data[single]",
            "data[vertex]",
            "length[dictionary]",
            "length[symbol]",
          ],
        },
        "geometries": [
          "point",
        ],
        "name": "props_str_fsst.mlt",
      }
    `);
  });

  it("names a dictionary column's layout without matching stream names", () => {
    const dict = index.find(
      (entry) =>
        entry.directory === "0x02" && entry.name === "props_str_front_dict.mlt",
    );
    expect(dict?.facets?.strLayout).toEqual(["dict"]);
    expect(dict?.facets?.dictLayout).toEqual(["front-coded"]);
    expect(dict?.encodings).toContain("Dictionary");
  });

  it("reads property data types from mlt ls", () => {
    expect(
      index.filter((entry) => entry.content?.length).length,
    ).toBeGreaterThan(0);
    expect(
      index.find(
        (entry) => entry.directory === "0x01" && entry.name === "prop_bool.mlt",
      )?.content,
    ).toEqual(["bool"]);
  });

  it("reads a shared-dict column as str, since SharedDict is an encoding", () => {
    expect(index.some((entry) => entry.content?.includes("shared-dict"))).toBe(
      false,
    );
    const shared = index.find(
      (entry) =>
        entry.directory === "0x01" &&
        entry.name === "props_shared_dict_no_child_name.mlt",
    );
    expect(shared?.content).toContain("str");
    expect(shared?.encodings).toContain("SharedDict");
  });

  it("flags the tiles that carry m-values", () => {
    const flagged = index.filter((entry) =>
      entry.content?.includes("m-values"),
    );
    expect(flagged.length).toBeGreaterThan(0);
    expect(flagged.every((entry) => entry.directory.startsWith("0x02"))).toBe(
      true,
    );
  });

  it("counts an m-value column's data type like a property column's", () => {
    // mvalues.mlt has no property columns at all, only i32 and u32 m-values
    expect(
      index.find(
        (entry) => entry.directory === "0x02" && entry.name === "mvalues.mlt",
      )?.content,
    ).toEqual(["i32", "m-values", "u32"]);
  });

  it("orders by directory then by name", () => {
    const keys = index.map((entry) => `${entry.directory}/${entry.name}`);
    const ordered = FIXTURE_DIRECTORIES.flatMap((directory) =>
      keys.filter((key) => key.startsWith(`${directory}/`)).sort(),
    );
    expect(keys).toEqual(ordered);
  });
});
