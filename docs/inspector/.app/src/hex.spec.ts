import { describe, expect, it } from "vitest";
import {
  ancestors,
  bandTint,
  byteOwners,
  defaultView,
  fadedFrom,
  fitColumns,
  hexOffset,
  leafStep,
  regionBands,
  regionDotPath,
  regionPath,
  tipPlacement,
  wholeIndices,
} from "./hex.ts";
import { region, tinyTree } from "./testing.ts";

const tree = tinyTree();

describe("fitColumns", () => {
  it("fits 16 columns into the map pane of the docs page's 61rem cap", () => {
    expect(fitColumns(624)).toBe(16);
  });

  it("fits 28 columns into the map pane of an uncapped page", () => {
    expect(fitColumns(948)).toBe(28);
  });

  it("fits 40 columns into the map pane of a standalone 1700px window", () => {
    expect(fitColumns(1348)).toBe(40);
  });

  it("leaves under one column of slack, so the bytes reach the gutter", () => {
    const pane = 900;
    const used = 84 + fitColumns(pane) * 30.3;
    expect(pane - used).toBeLessThan(4 * 30.3);
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

describe("regionBands", () => {
  it("counts a band per container and hands it to their leaves", () => {
    expect([...regionBands(tree)]).toEqual([0, 0, 1, 1, 1]);
  });

  it("keeps counting past the sixth container, where the tint wraps", () => {
    const nested = {
      bufLen: 7,
      regions: Array.from({ length: 7 }, (_, depth) =>
        region({
          offset: 0,
          len: 7 - depth,
          label: `c${depth}`,
          depth,
          container: true,
        }),
      ),
    };
    expect([...regionBands(nested)]).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });

  it("parts a leaf from the seventh container it touches", () => {
    const wrapped = {
      bufLen: 7,
      regions: [
        region({ offset: 0, len: 7, label: "layer[0]", container: true }),
        ...Array.from({ length: 6 }, (_, at) =>
          region({
            offset: at,
            len: 1,
            label: `c${at}`,
            depth: 1,
            container: true,
          }),
        ),
        region({ offset: 6, len: 1, label: "tail", depth: 1 }),
      ],
    };
    const bands = regionBands(wrapped);
    expect(bandTint(bands[6])).toBe(bandTint(bands[7]));
    expect(bands[6]).not.toBe(bands[7]);
  });

  it("leaves a top-level leaf untinted", () => {
    const flat = {
      bufLen: 2,
      regions: [region({ offset: 0, len: 2, label: "name" })],
    };
    expect([...regionBands(flat)]).toEqual([-1]);
  });
});

describe("bandTint", () => {
  it("cycles the palette over the bands", () => {
    expect([0, 5, 6, 7].map(bandTint)).toEqual([0, 5, 0, 1]);
  });

  it("leaves a bandless region untinted", () => {
    expect(bandTint(-1)).toBe(-1);
  });
});

describe("regionPath", () => {
  it("names the containers a leaf sits in, outermost first", () => {
    expect(regionPath(tree.regions, 4)).toEqual(["layer[0]", "geometry"]);
  });

  it("is empty for a top-level container", () => {
    expect(regionPath(tree.regions, 0)).toEqual([]);
  });

  it("skips the column data and stream header groupings", () => {
    const regions = [
      region({ offset: 0, len: 4, label: "layer[0]", container: true }),
      region({
        offset: 0,
        len: 4,
        label: "column data",
        depth: 1,
        container: true,
      }),
      region({
        offset: 0,
        len: 4,
        label: "column[1] Geometry",
        depth: 2,
        container: true,
      }),
      region({
        offset: 0,
        len: 4,
        label: "stream[2]",
        depth: 3,
        container: true,
      }),
      region({
        offset: 0,
        len: 2,
        label: "header",
        depth: 4,
        container: true,
      }),
      region({ offset: 0, len: 1, label: "stream_type", depth: 5 }),
    ];
    expect(regionPath(regions, 5)).toEqual([
      "layer[0]",
      "column[1] Geometry",
      "stream[2]",
    ]);
  });
});

describe("regionDotPath", () => {
  it("names a leaf by the containers holding it", () => {
    expect(regionDotPath(tree.regions, 3)).toBe("layer[0].geometry.encoding");
  });

  it("names a top-level container by itself", () => {
    expect(regionDotPath(tree.regions, 0)).toBe("layer[0]");
  });
});

describe("tipPlacement", () => {
  const tip = { width: 200, height: 100 };
  const viewport = { width: 1000, height: 800 };

  it("sits below and right of the pointer", () => {
    expect(tipPlacement({ x: 300, y: 400 }, tip, viewport)).toEqual({
      x: 314,
      y: 414,
    });
  });

  it("stops short of the right edge", () => {
    expect(tipPlacement({ x: 980, y: 400 }, tip, viewport).x).toBe(786);
  });

  it("flips above a pointer near the bottom edge", () => {
    expect(tipPlacement({ x: 300, y: 760 }, tip, viewport).y).toBe(646);
  });

  it("keeps a tip wider than the viewport at the left edge", () => {
    expect(
      tipPlacement({ x: 300, y: 400 }, { width: 2000, height: 100 }, viewport)
        .x,
    ).toBe(14);
  });

  it("keeps a tip taller than the viewport at the top edge", () => {
    expect(
      tipPlacement({ x: 300, y: 400 }, { width: 200, height: 900 }, viewport).y,
    ).toBe(14);
  });
});

describe("ancestors", () => {
  it("indexes the containers a leaf sits in", () => {
    expect(ancestors(tree.regions, 4)).toEqual([0, 2]);
  });
});

describe("fadedFrom", () => {
  it("never fades metadata", () => {
    expect(fadedFrom(tree.regions[1])).toBe(Number.POSITIVE_INFINITY);
  });

  it("fades a blob past its first byte", () => {
    expect(fadedFrom(tree.regions[4])).toBe(4);
  });
});

describe("defaultView", () => {
  it("starts with the sections untinted", () => {
    expect(defaultView().colorful).toBe(false);
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

describe("hexOffset", () => {
  it("pads an offset to four digits", () => {
    expect(hexOffset(0x2a, 0x100)).toBe("002a");
  });

  it("keeps four digits for a tile of exactly 64 KiB", () => {
    expect(hexOffset(0x2a, 0x10000)).toBe("002a");
  });

  it("widens every offset of a tile past 64 KiB", () => {
    expect(hexOffset(0x2a, 0x10001)).toBe("0002a");
  });
});
