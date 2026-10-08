import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  decodeTileColumns,
  columnValue,
  type MltColumnLayer,
} from "./columns";

function fixture(name: string): Uint8Array {
  const url = new URL(`../../../test/synthetic/${name}.mlt`, import.meta.url);
  return new Uint8Array(readFileSync(url));
}

function onlyLayer(name: string): MltColumnLayer {
  const { layers } = decodeTileColumns(fixture(name));
  expect(layers).toHaveLength(1);
  return layers[0];
}

describe("decodeTileColumns", () => {
  it("reads a v1 point as one (x, y) pair", () => {
    const layer = onlyLayer("0x01/point");
    expect(layer.version).toBe(1);
    expect(layer.featureCount).toBe(1);
    expect(layer.geometry.dimension).toBe(2);
    expect(layer.geometry.zStep).toBeUndefined();
    expect(Array.from(layer.geometry.vertices)).toEqual([13, 42]);
    expect(layer.ids).toBeUndefined();
  });

  it("interleaves z after x and y", () => {
    const layer = onlyLayer("0x02/z_point");
    expect(layer.version).toBe(2);
    const { geometry } = layer;
    if (geometry.dimension !== 3) throw new Error("expected 3D vertices");
    // `dimension` narrows `zStep` to a number, which tsc checks here.
    const zStep: number = geometry.zStep;
    expect(zStep).toBe(-1);
    expect(Array.from(geometry.vertices)).toEqual([13, 42, 100_120]);
  });

  it("marks absent ids in a bitmap and leaves their slots at 0", () => {
    const { ids } = onlyLayer("0x01/ids_opt");
    expect(ids).toBeDefined();
    if (ids === undefined) return;
    expect(Array.from(ids.values)).toEqual([100, 101, 0, 105, 106]);
    expect(Array.from(ids.present ?? [])).toEqual([0b11011]);
    expect([0, 1, 2, 3, 4].map((i) => columnValue(ids, i))).toEqual([
      100,
      101,
      undefined,
      105,
      106,
    ]);
  });

  it("marks absent property values in a bitmap and leaves their slots at 0", () => {
    const [val] = onlyLayer("0x02/presence_bitmap").properties;
    expect(val.name).toBe("val");
    expect(Array.from(val.values)).toEqual([0, 0, 2, 0, 4, 0, 6, 0]);
    // A stored 0 and an absent value differ only by the bitmap, which columnValue reads.
    expect([0, 1, 2, 3].map((i) => columnValue(val, i))).toEqual([0, undefined, 2, undefined]);
    expect(Array.from(val.present ?? [])).toEqual([0b0101_0101]);
  });

  it("rejects an index that is not a feature, rather than reading it as no value", () => {
    const [val] = onlyLayer("0x02/prop_i32").properties;
    expect(columnValue(val, 0)).toBe(42);
    for (const index of [-1, 1, 0.5, Number.NaN]) {
      expect(() => columnValue(val, index)).toThrow(RangeError);
    }
  });

  it("leaves out the bitmap when every feature has a value", () => {
    const [val] = onlyLayer("0x02/prop_i32").properties;
    expect(val).toEqual({
      name: "val",
      type: "i32",
      values: Int32Array.of(42),
    });
  });

  it("passes on the tessellation of a layer with outlines", () => {
    const { geometry } = onlyLayer("0x02/mix_2_poly_mpoly_tes");
    expect(geometry.partOffsets).toBeDefined();
    expect(geometry.triangleOffsets).toBeDefined();
    expect(geometry.indexBuffer).toBeDefined();
    if (!geometry.triangleOffsets || !geometry.indexBuffer) return;
    const triangles = geometry.triangleOffsets.at(-1) ?? 0;
    expect(geometry.indexBuffer).toHaveLength(triangles * 3);
    const vertexCount = geometry.vertices.length / geometry.dimension;
    expect(geometry.indexBuffer.every((i) => i < vertexCount)).toBe(true);
  });

  it("passes on the triangles of a layer without outlines", () => {
    const { geometry } = onlyLayer("0x02/z_poly_hole_tri");
    expect(geometry.dimension).toBe(3);
    expect(geometry.partOffsets).toBeUndefined();
    expect(geometry.indexBuffer).toBeDefined();
  });
});

describe("the layers option", () => {
  // A tile is a sequence of layer frames, so fixtures concatenate into a multi-layer tile.
  const tile = concat(
    fixture("0x01/fpf_align_1"),
    fixture("0x02/z_point"),
    fixture("0x01/fpf_align_2"),
    fixture("0x02/z_point"),
  );
  const names = (options?: { layers: string[] }) =>
    decodeTileColumns(tile, options).layers.map((layer) => layer.name);

  it("decodes every layer when left out", () => {
    expect(names()).toEqual(["a", "layer1", "aa", "layer1"]);
  });

  it("keeps the named layers in wire order, each one with a listed name", () => {
    expect(names({ layers: ["aa", "layer1"] })).toEqual([
      "layer1",
      "aa",
      "layer1",
    ]);
  });

  it("decodes nothing for an empty list or a name the tile lacks", () => {
    expect(names({ layers: [] })).toEqual([]);
    expect(names({ layers: ["missing"] })).toEqual([]);
  });

  it("decodes a kept layer as it does alone", () => {
    const [layer] = decodeTileColumns(tile, { layers: ["aa"] }).layers;
    expect(layer).toEqual(onlyLayer("0x01/fpf_align_2"));
  });
});

function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
