import type { FeatureCollection } from "geojson";
import { describe, expect, it } from "vitest";
import { region } from "./testing.ts";
import { geoStat, tileStat } from "./tileStats.ts";

const v2Tree = {
  bufLen: 130,
  regions: [
    region({ offset: 0, len: 100, label: 'layer[0] "roads"', container: true }),
    region({
      offset: 0,
      len: 1,
      label: "tag",
      depth: 1,
      value: "0x02 -> Tag02",
    }),
    region({ offset: 1, len: 1, label: "feature_count", depth: 1, value: "3" }),
    region({
      offset: 2,
      len: 30,
      label: "geometry",
      depth: 1,
      container: true,
    }),
    region({
      offset: 2,
      len: 10,
      label: "vertices",
      depth: 2,
      container: true,
    }),
    region({
      offset: 32,
      len: 8,
      label: "column[0] LongId",
      depth: 1,
      container: true,
    }),
    region({
      offset: 40,
      len: 40,
      label: 'column[1] OptStr "name"',
      depth: 1,
      container: true,
    }),
    region({
      offset: 80,
      len: 15,
      label: 'm_value[0] U32 "m0"',
      depth: 1,
      container: true,
    }),
    region({
      offset: 100,
      len: 30,
      label: 'layer[1] "water"',
      container: true,
    }),
    region({
      offset: 100,
      len: 20,
      label: "column[0] Geometry",
      depth: 1,
      container: true,
    }),
  ],
};

describe("tileStat", () => {
  const stat = tileStat(v2Tree);

  it("splits a layer by what each column holds", () => {
    expect(stat.layers[0].bytes).toEqual({
      geometry: 30,
      properties: 55,
      ids: 8,
      metadata: 7,
    });
  });

  it("names the layers and their columns", () => {
    expect(stat.layers.map((l) => l.name)).toEqual(["roads", "water"]);
    expect(stat.layers[0].columns.map((c) => [c.name, c.category])).toEqual([
      ["", "geometry"],
      ["", "ids"],
      ["name", "properties"],
      ["m0", "properties"],
    ]);
  });

  it("reads a v1 geometry column as geometry", () => {
    expect(stat.layers[1].bytes.geometry).toBe(20);
  });

  it("adds the layers up across the tile", () => {
    expect(stat.bytes).toEqual({
      geometry: 50,
      properties: 55,
      ids: 8,
      metadata: 17,
    });
  });

  it("leaves bytes no layer claims as unannotated", () => {
    expect(tileStat({ ...v2Tree, bufLen: 140 }).unannotated).toBe(10);
  });
});

describe("geoStat", () => {
  const feature = (layer: string, type: string, coordinates: unknown) => ({
    type: "Feature" as const,
    geometry: { type, coordinates } as never,
    properties: { _layer: layer },
  });
  const tile: FeatureCollection = {
    type: "FeatureCollection",
    features: [
      feature("a", "Point", [0, 0]),
      feature("a", "LineString", [
        [0, 0],
        [1, 1],
        [2, 2],
      ]),
      feature("b", "Point", [1, 1]),
    ],
  };

  it("counts features and vertices per layer", () => {
    const { layers } = geoStat(tile);
    expect(layers.get("a")?.features).toBe(2);
    expect(layers.get("a")?.vertices).toBe(4);
  });

  it("counts geometry types across the tile", () => {
    expect([...geoStat(tile).whole.types]).toEqual([
      ["Point", 2],
      ["LineString", 1],
    ]);
  });
});
