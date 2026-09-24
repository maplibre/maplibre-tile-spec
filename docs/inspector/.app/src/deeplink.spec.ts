import { beforeEach, describe, expect, it } from "vitest";
import {
  type DeepLink,
  deepLinkSearch,
  readDeepLink,
  writeDeepLink,
} from "./deeplink.ts";

/** A link carrying nothing, which each case names only the parameters it is about over. */
const bare: DeepLink = { fixture: null, url: null, layer: null, region: null };

describe("readDeepLink", () => {
  it("reads the three parameters of the contract", () => {
    expect(readDeepLink("?fixture=0x02/point.mlt&layer=3&region=57")).toEqual({
      ...bare,
      fixture: "0x02/point.mlt",
      layer: 3,
      region: 57,
    });
  });

  it("ignores everything else in the query", () => {
    expect(readDeepLink("?width=48")).toEqual(bare);
  });

  it("drops an index that is not a whole number", () => {
    expect(readDeepLink("?region=-1&layer=two")).toEqual(bare);
  });
});

describe("the url parameter", () => {
  it("reads the address a link points at", () => {
    expect(readDeepLink("?url=https%3A%2F%2Fexample.org%2Fa.mlt").url).toBe(
      "https://example.org/a.mlt",
    );
  });

  it("drops one no tile could be fetched from", () => {
    expect(readDeepLink("?url=file%3A%2F%2F%2Fetc%2Fpasswd").url).toBeNull();
  });

  it("writes it back, so a tile from anywhere can be linked to", () => {
    expect(deepLinkSearch({ ...bare, url: "https://example.org/a.mlt" })).toBe(
      "?url=https%3A%2F%2Fexample.org%2Fa.mlt",
    );
  });
});

describe("deepLinkSearch", () => {
  it("writes the parameters it has", () => {
    expect(
      deepLinkSearch({ ...bare, fixture: "0x01/point.mlt", region: 4 }),
    ).toBe("?fixture=0x01%2Fpoint.mlt&region=4");
  });

  it("writes nothing for an empty link", () => {
    expect(deepLinkSearch({ ...bare })).toBe("");
  });
});

describe("writeDeepLink", () => {
  beforeEach(() => {
    history.replaceState(null, "", "/");
  });

  it("stays on one entry for a move within a tile", () => {
    const entries = history.length;
    writeDeepLink({ ...bare, fixture: "0x01/point.mlt", region: 4 });
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt&region=4");
    expect(history.length).toBe(entries);
  });

  it("leaves one behind for a link that opens another tile", () => {
    const entries = history.length;
    writeDeepLink({ ...bare, fixture: "0x01/point.mlt" }, true);
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt");
    expect(history.length).toBe(entries + 1);
  });

  it("keeps a fragment, which names nothing this app owns", () => {
    history.replaceState(null, "", "/#annotating");
    writeDeepLink({ ...bare });
    expect(location.hash).toBe("#annotating");
  });
});
