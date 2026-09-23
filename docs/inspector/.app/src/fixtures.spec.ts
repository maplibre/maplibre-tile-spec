import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type FixtureEntry,
  facetsOf,
  fixtureEncoder,
  fixtureKey,
  fixturePrefix,
  fixtureTag,
  fuzzyMatch,
  groupFixtures,
  loadFixture,
  loadFixtureIndex,
  matchesFacets,
  starterFixtures,
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

describe("fixtureTag and fixtureEncoder", () => {
  it("reads the wire tag and the encoder out of the directory", () => {
    expect(faceted.map(fixtureTag)).toEqual(["0x01", "0x01", "0x02"]);
    expect(faceted.map(fixtureEncoder)).toEqual(["java", "rust", "java"]);
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

  it("treats values within one facet as alternatives", () => {
    const hit = faceted.filter((e) =>
      matchesFacets(e, facets, pick("geometry:Point", "geometry:Polygon")),
    );
    expect(hit.map((e) => e.name)).toEqual(["a.mlt", "b.mlt", "c.mlt"]);
  });

  it("requires every picked facet to hold", () => {
    const hit = faceted.filter((e) =>
      matchesFacets(e, facets, pick("geometry:Polygon", "tag:0x02")),
    );
    expect(hit.map((e) => e.name)).toEqual(["c.mlt"]);
  });
});
