import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type FixtureEntry,
  facetsOf,
  fixtureKey,
  fixturePrefix,
  fixtureTag,
  fuzzyMatch,
  groupFixtures,
  loadFixture,
  loadFixtureIndex,
  matchesFacets,
  sortFixtures,
  starterFixtures,
  tileAddress,
} from "./fixtures.ts";

const entry: FixtureEntry = {
  name: "point-int.mlt",
  directory: "0x01",
  bytes: 42,
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("fixtureKey", () => {
  it("joins directory and name", () => {
    expect(fixtureKey(entry)).toBe("0x01/point-int.mlt");
  });
});

describe("loadFixtureIndex", () => {
  it("reads the index written by the build", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json([entry])));
    await expect(loadFixtureIndex()).resolves.toEqual([entry]);
  });

  it("names the status of a failed request", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(null, { status: 404, statusText: "Not Found" }),
        ),
    );
    await expect(loadFixtureIndex()).rejects.toThrow(
      "fixtures.json: 404 Not Found",
    );
  });
});

describe("loadFixture", () => {
  it("reads the tile bytes", async () => {
    const fetched = vi
      .fn()
      .mockResolvedValue(new Response(new Uint8Array([1, 2, 3])));
    vi.stubGlobal("fetch", fetched);
    await expect(loadFixture("0x01/point-int.mlt")).resolves.toEqual(
      new Uint8Array([1, 2, 3]),
    );
    expect(fetched).toHaveBeenCalledWith("fixtures/0x01/point-int.mlt");
  });

  it("names the status of a failed request", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(null, { status: 500, statusText: "Server Error" }),
        ),
    );
    await expect(loadFixture("0x01/point-int.mlt")).rejects.toThrow(
      "0x01/point-int.mlt: 500 Server Error",
    );
  });
});

describe("fixturePrefix", () => {
  it("takes everything up to the first underscore", () => {
    expect(fixturePrefix("ids64_minmax_delta.mlt")).toBe("ids64");
  });

  it("takes the whole stem when there is no underscore", () => {
    expect(fixturePrefix("point.mlt")).toBe("point");
  });
});

describe("groupFixtures", () => {
  const entries: FixtureEntry[] = [
    { name: "point.mlt", directory: "0x02", bytes: 1 },
    { name: "ids_rle.mlt", directory: "0x01", bytes: 2 },
    { name: "extent_512.mlt", directory: "0x01", bytes: 3 },
    { name: "ids.mlt", directory: "0x01", bytes: 4 },
  ];

  it("orders by directory, then prefix, then name", () => {
    expect(groupFixtures(entries)).toEqual([
      { label: "0x01/extent", entries: [entries[2]] },
      { label: "0x01/ids", entries: [entries[3], entries[1]] },
      { label: "0x02/point", entries: [entries[0]] },
    ]);
  });
});

describe("fuzzyMatch", () => {
  it("matches a subsequence spread over the name", () => {
    expect(fuzzyMatch("props_str_fsst", "pstrf")).toBe(true);
  });

  it("matches a plain substring", () => {
    expect(fuzzyMatch("props_str_fsst", "fsst")).toBe(true);
  });

  it("rejects the same letters out of order", () => {
    expect(fuzzyMatch("props_str_fsst", "fsstp")).toBe(false);
  });

  it("rejects a letter the name does not carry", () => {
    expect(fuzzyMatch("props_str_fsst", "psz")).toBe(false);
  });

  it("matches everything on an empty needle", () => {
    expect(fuzzyMatch("props_str_fsst", "")).toBe(true);
  });

  it("needs as many of a letter as the needle asks for", () => {
    expect(fuzzyMatch("point", "ttt")).toBe(false);
  });
});

describe("starterFixtures", () => {
  it("keeps the starters the index carries", () => {
    expect(
      starterFixtures([{ name: "point.mlt", directory: "0x02", bytes: 21 }]),
    ).toEqual([
      {
        key: "0x02/point.mlt",
        title: "a single point",
        note: "the smallest tile there is",
      },
    ]);
  });

  it("offers nothing from an index that carries no starter", () => {
    expect(starterFixtures([entry])).toEqual([]);
  });
});

const faceted = [
  {
    name: "a.mlt",
    directory: "0x01",
    bytes: 1,
    geometries: ["Point"],
    encodings: ["RLE"],
  },
  {
    name: "b.mlt",
    directory: "0x01-rust",
    bytes: 1,
    geometries: ["Point", "Polygon"],
    encodings: [],
  },
  {
    name: "c.mlt",
    directory: "0x02",
    bytes: 1,
    geometries: ["Polygon"],
    encodings: ["RLE", "FSST"],
  },
];

describe("fixtureTag", () => {
  it("reads the wire tag out of the directory", () => {
    expect(faceted.map(fixtureTag)).toEqual(["0x01", "0x01", "0x02"]);
  });
});

describe("sortFixtures", () => {
  const rows = [
    { name: "b.mlt", directory: "0x01", bytes: 30 },
    { name: "a.mlt", directory: "0x01", bytes: 10 },
    { name: "c.mlt", directory: "0x01", bytes: 20 },
  ];

  it("sorts by key, and reverses it", () => {
    expect(sortFixtures(rows, "name", false).map((e) => e.name)).toEqual([
      "a.mlt",
      "b.mlt",
      "c.mlt",
    ]);
    expect(sortFixtures(rows, "name", true).map((e) => e.name)).toEqual([
      "c.mlt",
      "b.mlt",
      "a.mlt",
    ]);
  });

  it("sorts by size, and reverses it", () => {
    expect(sortFixtures(rows, "bytes", false).map((e) => e.bytes)).toEqual([
      10, 20, 30,
    ]);
    expect(sortFixtures(rows, "bytes", true).map((e) => e.bytes)).toEqual([
      30, 20, 10,
    ]);
  });

  it("breaks a size tie on the key, so the order is stable", () => {
    const tied = [
      { name: "b.mlt", directory: "0x02", bytes: 5 },
      { name: "a.mlt", directory: "0x01", bytes: 5 },
    ];
    expect(sortFixtures(tied, "bytes", false).map((e) => e.name)).toEqual([
      "a.mlt",
      "b.mlt",
    ]);
  });

  it("leaves the input alone", () => {
    const before = rows.map((e) => e.name);
    sortFixtures(rows, "bytes", true);
    expect(rows.map((e) => e.name)).toEqual(before);
  });
});

describe("tileAddress", () => {
  it("takes a web address as it is given", () => {
    expect(tileAddress("https://example.org/a.mlt")).toBe(
      "https://example.org/a.mlt",
    );
  });

  it("reads a relative one against the page, so a tile beside it can be named", () => {
    expect(tileAddress("fixtures/0x02/point.mlt")).toBe(
      `${location.origin}/fixtures/0x02/point.mlt`,
    );
  });

  it("trims what was pasted, which often arrives with a space on it", () => {
    expect(tileAddress("  https://example.org/a.mlt ")).toBe(
      "https://example.org/a.mlt",
    );
  });

  it("refuses a scheme a link could point at this reader's own machine", () => {
    expect(tileAddress("file:///etc/passwd")).toBeNull();
  });

  it("refuses one that is not an address at all", () => {
    expect(tileAddress("http://")).toBeNull();
  });
});

describe("facetsOf", () => {
  it("counts each value over the entries", () => {
    const geometry = facetsOf(faceted).find((f) => f.label === "geometry");
    expect(geometry?.values).toEqual([
      { value: "Point", count: 2 },
      { value: "Polygon", count: 2 },
    ]);
  });

  it("keeps a lone value when only some entries carry it, which is how a flag reads", () => {
    const flagged = [
      { name: "a.mlt", directory: "0x01", bytes: 1, content: ["m-values"] },
      { name: "b.mlt", directory: "0x01", bytes: 1, content: [] },
    ];
    const has = facetsOf(flagged).find((f) => f.label === "has");
    expect(has?.values).toEqual([{ value: "m-values", count: 1 }]);
  });

  it("puts data types and tile flags on one row", () => {
    const mixed = [
      {
        name: "a.mlt",
        directory: "0x01",
        bytes: 1,
        content: ["bool", "m-values"],
      },
      { name: "b.mlt", directory: "0x01", bytes: 1, content: ["str"] },
    ];
    const rows = facetsOf(mixed).filter((f) => f.label === "has");
    expect(rows).toHaveLength(1);
    expect(rows[0].values.map((v) => v.value).sort()).toEqual([
      "bool",
      "m-values",
      "str",
    ]);
  });

  it("orders the types by family, and the flag last however common it is", () => {
    const rows = [
      {
        name: "a.mlt",
        directory: "0x01",
        bytes: 1,
        content: ["m-values", "str", "u8", "i32", "bool", "f64", "i8"],
      },
      { name: "b.mlt", directory: "0x01", bytes: 1, content: ["m-values"] },
      { name: "c.mlt", directory: "0x01", bytes: 1, content: [] },
    ];
    const has = facetsOf(rows).find((f) => f.label === "has");
    expect(has?.values.map((v) => v.value)).toEqual([
      "bool",
      "i8",
      "i32",
      "u8",
      "f64",
      "str",
      "m-values",
    ]);
  });

  it("puts a type the table does not name past the end, not silently first", () => {
    const rows = [
      {
        name: "a.mlt",
        directory: "0x01",
        bytes: 1,
        content: ["quantized", "str"],
      },
      { name: "b.mlt", directory: "0x01", bytes: 1, content: [] },
    ];
    const has = facetsOf(rows).find((f) => f.label === "has");
    expect(has?.values.map((v) => v.value)).toEqual(["str", "quantized"]);
  });

  it("drops a value every entry carries, which could not narrow anything", () => {
    const all = [
      { name: "a.mlt", directory: "0x01", bytes: 1, geometries: ["Point"] },
      { name: "b.mlt", directory: "0x01", bytes: 1, geometries: ["Point"] },
    ];
    expect(facetsOf(all).map((f) => f.label)).not.toContain("geometry");
  });
});

describe("matchesFacets", () => {
  const facets = facetsOf(faceted);
  const pick = (...keys: string[]) => new Set(keys);

  it("keeps everything when nothing is picked", () => {
    expect(
      faceted.filter((e) => matchesFacets(e, facets, pick())),
    ).toHaveLength(3);
  });

  it("requires every picked value within one facet, not just one of them", () => {
    const hit = faceted.filter((e) =>
      matchesFacets(e, facets, pick("geometry:Point", "geometry:Polygon")),
    );
    expect(hit.map((e) => e.name)).toEqual(["b.mlt"]);
  });

  it("finds nothing for two tags, since a tile carries exactly one", () => {
    const hit = faceted.filter((e) =>
      matchesFacets(e, facets, pick("tag:0x01", "tag:0x02")),
    );
    expect(hit).toEqual([]);
  });

  it("requires every picked facet to hold", () => {
    const hit = faceted.filter((e) =>
      matchesFacets(e, facets, pick("geometry:Polygon", "tag:0x02")),
    );
    expect(hit.map((e) => e.name)).toEqual(["c.mlt"]);
  });

  it("requires a tile to carry every picked `has` value", () => {
    const rows = [
      {
        name: "both.mlt",
        directory: "0x02",
        bytes: 1,
        content: ["str", "bool"],
      },
      { name: "one.mlt", directory: "0x02", bytes: 1, content: ["str"] },
      { name: "other.mlt", directory: "0x02", bytes: 1, content: ["bool"] },
    ];
    const has = facetsOf(rows);
    const hit = rows.filter((e) =>
      matchesFacets(e, has, pick("has:str", "has:bool")),
    );
    expect(hit.map((e) => e.name)).toEqual(["both.mlt"]);
  });
});
