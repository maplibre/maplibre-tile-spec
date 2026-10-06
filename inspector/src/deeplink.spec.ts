import { beforeEach, describe, expect, it } from "vitest";
import {
  type DeepLink,
  deepLinkSearch,
  readDeepLink,
  writeDeepLink,
} from "./deeplink.ts";

/** A link carrying nothing, which each case names only the parameters it is about over. */
const bare: DeepLink = {
  fixture: null,
  url: null,
  layer: null,
  region: null,
  at: null,
  filters: [],
  query: "",
  geo: true,
  stats: false,
};

describe("the filter parameter", () => {
  it("reads every picked chip, repeated rather than joined", () => {
    expect(readDeepLink("?f=tag:0x02&f=logical:alp").filters).toEqual([
      "tag:0x02",
      "logical:alp",
    ]);
  });

  /** Values carry brackets, so they have to survive the round trip intact. */
  it("round-trips a value spelled with punctuation", () => {
    const link = { ...bare, filters: ["streamType:data[vertex]"] };
    expect(readDeepLink(deepLinkSearch(link)).filters).toEqual(link.filters);
  });

  it("ignores a chip that names no axis", () => {
    expect(readDeepLink("?f=nonsense").filters).toEqual([]);
  });

  it("writes nothing when nothing is picked", () => {
    expect(deepLinkSearch(bare)).toBe("");
  });

  it("carries the filter box beside the chips", () => {
    expect(readDeepLink("?q=fsst").query).toBe("fsst");
    expect(deepLinkSearch({ ...bare, query: "fsst" })).toBe("?q=fsst");
  });

  it("round-trips a chip and a typed word together", () => {
    const link = { ...bare, filters: ["logical:alp"], query: "f64" };
    const back = readDeepLink(deepLinkSearch(link));
    expect([back.filters, back.query]).toEqual([link.filters, link.query]);
  });
});

describe("the graphics parameter", () => {
  it("draws the panel when the link says nothing", () => {
    expect(readDeepLink("").geo).toBe(true);
    expect(deepLinkSearch(bare)).toBe("");
  });

  it("writes only the hidden state, and reads it back", () => {
    const hidden = { ...bare, geo: false };
    expect(deepLinkSearch(hidden)).toBe("?geo=0");
    expect(readDeepLink(deepLinkSearch(hidden)).geo).toBe(false);
  });
});

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

  /** A bare `?url=` would otherwise resolve to this page, and be read as a tile. */
  it("drops a blank one rather than pointing the app at itself", () => {
    expect(readDeepLink("?url=").url).toBeNull();
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

describe("a link that names its region", () => {
  it("reads `at` for a hand-written link", () => {
    expect(readDeepLink("?fixture=0x02%2Fpoly_tri.mlt&at=tri_lengths").at).toBe(
      "tri_lengths",
    );
  });

  /** The app resolves it to a number, so the address bar carries that instead. */
  it("never writes it back", () => {
    expect(deepLinkSearch({ ...bare, at: "tri_lengths", region: 8 })).toBe(
      "?region=8",
    );
  });
});

describe("the statistics panel", () => {
  it("is hidden unless the link says otherwise", () => {
    expect(readDeepLink("").stats).toBe(false);
  });

  it("writes only the on state", () => {
    const shown = { ...bare, stats: true };
    expect(deepLinkSearch(bare)).toBe("");
    expect(deepLinkSearch(shown)).toBe("?stats=1");
    expect(readDeepLink("?stats=1").stats).toBe(true);
  });
});
