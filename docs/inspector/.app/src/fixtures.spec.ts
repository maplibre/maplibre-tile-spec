import { afterEach, describe, expect, it, vi } from "vitest";
import {
  axisKey,
  axisRows,
  coverageGaps,
  type FixtureEntry,
  fixtureKey,
  fixturePrefix,
  fixtureTag,
  fuzzyMatch,
  groupFixtures,
  loadFixture,
  loadFixtureIndex,
  matchesAxes,
  searchVocabulary,
  sectionRows,
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

const faceted: FixtureEntry[] = [
  {
    name: "a.mlt",
    directory: "0x01",
    bytes: 1,
    facets: { geometry: ["point"], logical: ["rle"] },
  },
  {
    name: "b.mlt",
    directory: "0x01-rust",
    bytes: 1,
    facets: { geometry: ["point", "polygon"] },
  },
  {
    name: "c.mlt",
    directory: "0x02",
    bytes: 1,
    facets: { geometry: ["polygon"], logical: ["rle"], strLayout: ["fsst"] },
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

  /** Resolved against the page, nothing comes back as the page, which is not a tile. */
  it("refuses nothing at all rather than resolving it to the page itself", () => {
    expect(tileAddress("")).toBeNull();
    expect(tileAddress("   ")).toBeNull();
  });
});

describe("axisRows", () => {
  const rows = axisRows(faceted, faceted);
  const row = (key: string) => rows.find((r) => r.key === key);
  const shown = (key: string) => row(key)?.values.map((v) => v.value);
  const count = (key: string, value: string) =>
    row(key)?.values.find((v) => v.value === value);

  it("counts each value over the matching entries", () => {
    expect(count("geometry", "point")).toEqual({
      value: "point",
      count: 2,
      total: 2,
    });
  });

  /** The point of a spec-first vocabulary: the gap is the interesting part. */
  it("offers a value no entry carries, rather than hiding it", () => {
    expect(shown("geometry")).toContain("multi-polygon");
    expect(count("geometry", "multi-polygon")).toEqual({
      value: "multi-polygon",
      count: 0,
      total: 0,
    });
  });

  it("keeps a value every entry carries, which the old bar dropped", () => {
    const all: FixtureEntry[] = [
      {
        name: "a.mlt",
        directory: "0x01",
        bytes: 1,
        facets: { geometry: ["point"] },
      },
      {
        name: "b.mlt",
        directory: "0x01",
        bytes: 1,
        facets: { geometry: ["point"] },
      },
    ];
    const geometry = axisRows(all, all).find((r) => r.key === "geometry");
    expect(geometry?.values.find((v) => v.value === "point")?.count).toBe(2);
  });

  it("separates what the filter would narrow to from what the index holds", () => {
    const narrowed = axisRows(faceted, [faceted[0]]);
    const point = narrowed
      .find((r) => r.key === "geometry")
      ?.values.find((v) => v.value === "point");
    expect(point).toEqual({ value: "point", count: 1, total: 2 });
  });

  it("orders the data types by family, not alphabetically", () => {
    expect(shown("dataType")?.slice(0, 5)).toEqual([
      "bool",
      "i8",
      "i32",
      "i64",
      "u8",
    ]);
  });

  it("puts a value the vocabulary does not name past the end, not silently first", () => {
    const odd: FixtureEntry[] = [
      {
        name: "a.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { logical: ["quantized"] },
      },
    ];
    const logical = axisRows(odd, odd).find((r) => r.key === "logical");
    const names = logical?.values.map((v) => v.value) ?? [];
    expect(names.at(-1)).toBe("quantized");
    expect(names.indexOf("alp")).toBeLessThan(names.indexOf("quantized"));
  });

  it("lists only the extents the index holds, smallest first", () => {
    const sized: FixtureEntry[] = [
      {
        name: "a.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { extent: ["4096"] },
      },
      {
        name: "b.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { extent: ["512"] },
      },
      {
        name: "c.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { extent: ["64"] },
      },
    ];
    const extent = axisRows(sized, sized).find((r) => r.key === "extent");
    // Not the 16 codes v2 allows: the 13 nobody uses are not gaps worth showing.
    // Numeric, so "512" does not file between "4096" and "64".
    expect(extent?.values.map((v) => v.value)).toEqual(["64", "512", "4096"]);
  });

  it("reads nothing off a tile mlt ls could not read", () => {
    const broken: FixtureEntry[] = [
      { name: "x.mlt", directory: "0x02", bytes: 1 },
    ];
    const geometry = axisRows(broken, broken).find((r) => r.key === "geometry");
    expect(geometry?.values.every((v) => v.total === 0)).toBe(true);
  });
});

describe("sectionRows", () => {
  it("groups the axes under the heading the sheet stacks them by", () => {
    const rows = axisRows(faceted, faceted);
    expect(sectionRows(rows, "Streams").map((r) => r.key)).toEqual([
      "streamType",
      "physical",
      "logical",
    ]);
  });
});

describe("matchesAxes", () => {
  const pick = (...keys: string[]) => new Set(keys);

  it("keeps everything when nothing is picked", () => {
    expect(faceted.filter((e) => matchesAxes(e, pick()))).toHaveLength(3);
  });

  it("requires every picked value within one axis, not just one of them", () => {
    const hit = faceted.filter((e) =>
      matchesAxes(e, pick("geometry:point", "geometry:polygon")),
    );
    expect(hit.map((e) => e.name)).toEqual(["b.mlt"]);
  });

  it("finds nothing for two tags, since a tile carries exactly one", () => {
    expect(
      faceted.filter((e) => matchesAxes(e, pick("tag:0x01", "tag:0x02"))),
    ).toEqual([]);
  });

  it("requires every picked axis to hold", () => {
    const hit = faceted.filter((e) =>
      matchesAxes(e, pick("geometry:polygon", "tag:0x02")),
    );
    expect(hit.map((e) => e.name)).toEqual(["c.mlt"]);
  });

  /** `data` is a prefix of `data[vertex]`, so a sloppy key split would confuse them. */
  it("tells one axis from another whose value contains a colon-free prefix", () => {
    const rows: FixtureEntry[] = [
      {
        name: "a.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { streamType: ["data"] },
      },
      {
        name: "b.mlt",
        directory: "0x02",
        bytes: 1,
        facets: { streamType: ["data[vertex]"] },
      },
    ];
    const hit = rows.filter((e) =>
      matchesAxes(e, pick("streamType:data[vertex]")),
    );
    expect(hit.map((e) => e.name)).toEqual(["b.mlt"]);
  });
});

describe("searchVocabulary", () => {
  const rows = axisRows(faceted, faceted);

  it("offers a chip for a word that is in no file name", () => {
    const hits = searchVocabulary(rows, "alp");
    expect(hits.map((h) => `${h.axis.key}:${h.value.value}`)).toEqual([
      "logical:alp",
    ]);
  });

  it("matches across axes", () => {
    const hits = searchVocabulary(rows, "front");
    expect(hits.map((h) => h.axis.key)).toEqual(["dictLayout"]);
  });

  it("offers nothing for an empty box", () => {
    expect(searchVocabulary(rows, "   ")).toEqual([]);
  });

  it("leaves out a chip that is already picked", () => {
    const hits = searchVocabulary(rows, "alp", new Set(["logical:alp"]));
    expect(hits).toEqual([]);
  });
});

describe("coverageGaps", () => {
  it("names every value no fixture in the index carries", () => {
    const gaps = coverageGaps(axisRows(faceted, faceted));
    const keys = gaps.map((g) => axisKey(g.axis, g.value.value));
    expect(keys).toContain("geometry:multi-polygon");
    expect(keys).toContain("logical:alp");
    expect(keys).not.toContain("geometry:point");
  });
});
