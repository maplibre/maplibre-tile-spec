import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type FixtureEntry,
  fixtureKey,
  fixturePrefix,
  groupFixtures,
  loadFixture,
  loadFixtureIndex,
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
      { label: "0x01 \u00b7 extent", entries: [entries[2]] },
      { label: "0x01 \u00b7 ids", entries: [entries[3], entries[1]] },
      { label: "0x02 \u00b7 point", entries: [entries[0]] },
    ]);
  });
});
