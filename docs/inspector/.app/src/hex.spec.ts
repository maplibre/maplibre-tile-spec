import { describe, expect, it } from "vitest";
import {
  ancestors,
  byteOwners,
  defaultView,
  fadedFrom,
  fitColumns,
  hex8,
  leafStep,
  regionPath,
  wholeIndices,
} from "./hex.ts";
import { region, tinyTree } from "./testing.ts";

const tree = tinyTree();

describe("fitColumns", () => {
  it("fits 16 columns into the docs page's 61rem cap", () => {
    expect(fitColumns(976)).toBe(16);
  });

  it("fits 32 columns into an uncapped page", () => {
    expect(fitColumns(1300)).toBe(32);
  });

  it("fits 48 columns into a standalone 1700px window", () => {
    expect(fitColumns(1700)).toBe(48);
  });

  it("clamps a narrow pane to eight columns", () => {
    expect(fitColumns(320)).toBe(8);
  });

  it("clamps a wide pane to 64 columns", () => {
    expect(fitColumns(4000)).toBe(64);
  });
});

describe("byteOwners", () => {
  it("names the leaf that owns each byte, skipping containers", () => {
    expect([...byteOwners(tree)]).toEqual([1, 1, 3, 4, 4, 4, 4, 4]);
  });

  it("leaves bytes no region covers at -1", () => {
    const partial = {
      bufLen: 4,
      regions: [region({ offset: 0, len: 2, label: "name" })],
    };
    expect([...byteOwners(partial)]).toEqual([0, 0, -1, -1]);
  });
});

describe("regionPath", () => {
  it("names the containers a leaf sits in, outermost first", () => {
    expect(regionPath(tree.regions, 4)).toEqual(["layer[0]", "geometry"]);
  });

  it("is empty for a top-level container", () => {
    expect(regionPath(tree.regions, 0)).toEqual([]);
  });
});

describe("ancestors", () => {
  it("indexes the containers a leaf sits in", () => {
    expect(ancestors(tree.regions, 4)).toEqual([0, 2]);
  });
});

describe("fadedFrom", () => {
  it("never fades metadata", () => {
    expect(fadedFrom(tree.regions[1], defaultView())).toBe(
      Number.POSITIVE_INFINITY,
    );
  });

  it("fades a blob past its first byte", () => {
    expect(fadedFrom(tree.regions[4], defaultView())).toBe(4);
  });

  it("fades a whole blob that the data knob hides", () => {
    expect(
      fadedFrom(tree.regions[4], { ...defaultView(), dataMode: "hidden" }),
    ).toBe(3);
  });

  it("fades the raw bytes the data knob replaces with decoded values", () => {
    expect(
      fadedFrom(tree.regions[4], { ...defaultView(), dataMode: "decoded" }),
    ).toBe(3);
  });
});

describe("wholeIndices", () => {
  it("maps a filtered tree back onto the tree decodeBlob counts", () => {
    const part = [tree.regions[2], tree.regions[3], tree.regions[4]];
    expect([...wholeIndices(tree.regions, part)]).toEqual([2, 3, 4]);
  });

  it("marks a region the whole tree does not hold", () => {
    const part = [region({ offset: 99, len: 1, label: "elsewhere" })];
    expect([...wholeIndices(tree.regions, part)]).toEqual([-1]);
  });
});

describe("leafStep", () => {
  it("skips containers walking forwards", () => {
    expect(leafStep(tree.regions, 1, 1)).toBe(3);
  });

  it("skips containers walking backwards", () => {
    expect(leafStep(tree.regions, 3, -1)).toBe(1);
  });

  it("stops at the last leaf", () => {
    expect(leafStep(tree.regions, 4, 1)).toBe(null);
  });
});

describe("hex8", () => {
  it("pads an offset to eight digits", () => {
    expect(hex8(0x2a)).toBe("0000002a");
  });
});
