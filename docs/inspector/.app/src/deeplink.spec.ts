import { describe, expect, it } from "vitest";
import { deepLinkSearch, readDeepLink } from "./deeplink.ts";

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
