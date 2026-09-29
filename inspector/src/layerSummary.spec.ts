import { describe, expect, it } from "vitest";
import type { Region } from "./annotate.ts";
import { layerSummary } from "./layerSummary.ts";
import { region } from "./testing.ts";

/** A v1 layer: the extent is spelled out, and only the geometry types count the features. */
function v1(): Region[] {
  return [
    region({ offset: 0, len: 40, label: 'layer[0] "roads"', container: true }),
    region({ offset: 0, len: 1, label: "size", depth: 1, value: "39" }),
    region({
      offset: 1,
      len: 1,
      label: "tag",
      depth: 1,
      value: "0x01 -> Tag01",
    }),
    region({ offset: 2, len: 6, label: "name", depth: 1, value: '"roads"' }),
    region({ offset: 8, len: 2, label: "extent", depth: 1, value: "4096" }),
    region({ offset: 10, len: 1, label: "column_count", depth: 1, value: "2" }),
    region({
      offset: 11,
      len: 20,
      label: "column[0] Geometry",
      depth: 1,
      container: true,
    }),
    region({ offset: 11, len: 10, label: "meta", depth: 2, container: true }),
    region({ offset: 11, len: 1, label: "num_values", depth: 3, value: "2" }),
    region({
      offset: 12,
      len: 3,
      label: "num_rle_values",
      depth: 3,
      value: "790",
    }),
    region({
      offset: 31,
      len: 9,
      label: 'column[1] OptStr "surface"',
      depth: 1,
      container: true,
    }),
    region({
      offset: 31,
      len: 1,
      label: "type",
      depth: 2,
      value: "0x1D OptStr",
    }),
  ];
}

/** A v2 layer: the header byte holds the extent and the feature count stands alone. */
function v2(): Region[] {
  return [
    region({ offset: 0, len: 37, label: 'layer[0] "layer1"', container: true }),
    region({
      offset: 0,
      len: 1,
      label: "tag",
      depth: 1,
      value: "0x02 -> Tag02",
    }),
    region({ offset: 1, len: 7, label: "name", depth: 1, value: '"layer1"' }),
    region({
      offset: 8,
      len: 1,
      label: "header",
      depth: 1,
      value: "extent = 64, every feature is a Point",
    }),
    region({ offset: 9, len: 1, label: "feature_count", depth: 1, value: "2" }),
    region({
      offset: 10,
      len: 17,
      label: 'column[0] OptU64 "val"',
      depth: 1,
      container: true,
    }),
  ];
}

describe("a v1 layer", () => {
  it("reads its size, tag and extent", () => {
    expect(layerSummary(v1(), 0)).toMatchObject({
      len: 40,
      tag: "Tag01",
      extent: "4096",
    });
  });

  it("counts the features the geometry types decode to, not the runs stored", () => {
    expect(layerSummary(v1(), 0)?.features).toBe(790);
  });

  it("falls back to the stored count when nothing is run-length coded", () => {
    const regions = v1();
    regions.splice(9, 1);
    expect(layerSummary(regions, 0)?.features).toBe(2);
  });

  it("has no count to give when the walk never reached the geometry", () => {
    expect(layerSummary(v1().slice(0, 6), 0)?.features).toBeNull();
  });

  it("lists the columns, leaving a nameless type without one", () => {
    expect(layerSummary(v1(), 0)?.columns).toEqual([
      { name: "", type: "Geometry" },
      { name: "surface", type: "OptStr" },
    ]);
  });
});

describe("a v2 layer", () => {
  it("takes the extent out of the header byte", () => {
    expect(layerSummary(v2(), 0)).toMatchObject({
      tag: "Tag02",
      extent: "64",
      features: 2,
    });
  });
});

describe("anything that is not a layer", () => {
  it("has no summary", () => {
    expect(layerSummary(v1(), 6)).toBeNull();
    expect(layerSummary(v1(), 4)).toBeNull();
    expect(layerSummary(v1(), 99)).toBeNull();
  });
});
