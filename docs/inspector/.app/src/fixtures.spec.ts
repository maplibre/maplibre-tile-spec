import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type FixtureEntry,
  fixtureKey,
  fixturePrefix,
  fixtureTags,
  fixtureTokens,
  fuzzyMatch,
  groupFixtures,
  loadFixture,
  loadFixtureIndex,
  matchesTags,
  starterFixtures,
  tagCounts,
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

describe("fixtureTokens", () => {
  it("splits a name on its underscores", () => {
    expect(fixtureTokens("mix_2_line_mpoly_tes.mlt")).toEqual([
      "mix",
      "2",
      "line",
      "mpoly",
      "tes",
    ]);
  });

  it("drops the extension", () => {
    expect(fixtureTokens("point.mlt")).toEqual(["point"]);
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

const tagged: FixtureEntry[] = [
  { name: "mix_2_line_mpoly_tes.mlt", directory: "0x02", bytes: 1 },
  { name: "props_str_fsst.mlt", directory: "0x01", bytes: 2 },
  { name: "point.mlt", directory: "0x02", bytes: 3 },
];

describe("fixtureTags", () => {
  it("tags the geometries and the streams a mixed name lists", () => {
    expect([...fixtureTags(tagged[0])]).toEqual([
      "v2",
      "line",
      "polygon",
      "multi-part",
      "tessellated",
    ]);
  });

  it("tags a name's columns and its encoding", () => {
    expect([...fixtureTags(tagged[1])]).toEqual(["v1", "properties", "fsst"]);
  });

  it("takes the version from the directory a re-encoding sits in", () => {
    expect(
      fixtureTags({ name: "point.mlt", directory: "0x01-rust", bytes: 1 }),
    ).toContain("v1");
  });

  it("leaves a directory outside both versions untagged", () => {
    expect([
      ...fixtureTags({ name: "point.mlt", directory: "0x03", bytes: 1 }),
    ]).toEqual(["point"]);
  });
});

describe("matchesTags", () => {
  const passing = (picked: string[]) =>
    tagged
      .filter((entry) => matchesTags(fixtureTags(entry), new Set(picked)))
      .map((entry) => entry.name);

  it("keeps every fixture while nothing is picked", () => {
    expect(passing([])).toEqual([
      "mix_2_line_mpoly_tes.mlt",
      "props_str_fsst.mlt",
      "point.mlt",
    ]);
  });

  it("ors the tags of one facet", () => {
    expect(passing(["point", "line"])).toEqual([
      "mix_2_line_mpoly_tes.mlt",
      "point.mlt",
    ]);
  });

  it("ands across facets", () => {
    expect(passing(["v2", "properties"])).toEqual([]);
  });

  it("ands a version onto a geometry", () => {
    expect(passing(["v2", "line"])).toEqual(["mix_2_line_mpoly_tes.mlt"]);
  });
});

describe("tagCounts", () => {
  it("counts what a chip alone would leave", () => {
    const counts = tagCounts(tagged, new Set());
    expect(counts.get("v2")).toBe(2);
    expect(counts.get("tessellated")).toBe(1);
    expect(counts.get("morton")).toBe(0);
  });

  it("counts a chip against the other facets' picks", () => {
    const counts = tagCounts(tagged, new Set(["v2"]));
    expect(counts.get("point")).toBe(1);
    expect(counts.get("properties")).toBe(0);
  });

  it("counts a chip as if its own facet were unpicked", () => {
    expect(tagCounts(tagged, new Set(["v2"])).get("v1")).toBe(1);
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
