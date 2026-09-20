import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type FixtureEntry,
  fixtureKey,
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
