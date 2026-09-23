import { beforeEach, describe, expect, it } from "vitest";
import { deepLinkSearch, readDeepLink, writeDeepLink } from "./deeplink.ts";

describe("readDeepLink", () => {
  it("reads the three parameters of the contract", () => {
    expect(readDeepLink("?fixture=0x02/point.mlt&layer=3&region=57")).toEqual({
      fixture: "0x02/point.mlt",
      layer: 3,
      region: 57,
    });
  });

  it("ignores everything else in the query", () => {
    expect(readDeepLink("?width=48")).toEqual({
      fixture: null,
      layer: null,
      region: null,
    });
  });

  it("drops an index that is not a whole number", () => {
    expect(readDeepLink("?region=-1&layer=two")).toEqual({
      fixture: null,
      layer: null,
      region: null,
    });
  });
});

describe("deepLinkSearch", () => {
  it("writes the parameters it has", () => {
    expect(
      deepLinkSearch({ fixture: "0x01/point.mlt", layer: null, region: 4 }),
    ).toBe("?fixture=0x01%2Fpoint.mlt&region=4");
  });

  it("writes nothing for an empty link", () => {
    expect(deepLinkSearch({ fixture: null, layer: null, region: null })).toBe(
      "",
    );
  });
});

describe("writeDeepLink", () => {
  beforeEach(() => {
    history.replaceState(null, "", "/");
  });

  it("stays on one entry for a move within a tile", () => {
    const entries = history.length;
    writeDeepLink({ fixture: "0x01/point.mlt", layer: null, region: 4 });
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt&region=4");
    expect(history.length).toBe(entries);
  });

  it("leaves one behind for a link that opens another tile", () => {
    const entries = history.length;
    writeDeepLink(
      { fixture: "0x01/point.mlt", layer: null, region: null },
      true,
    );
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt");
    expect(history.length).toBe(entries + 1);
  });

  it("keeps a fragment, which names nothing this app owns", () => {
    history.replaceState(null, "", "/#annotating");
    writeDeepLink({ fixture: null, layer: null, region: null });
    expect(location.hash).toBe("#annotating");
  });
});
