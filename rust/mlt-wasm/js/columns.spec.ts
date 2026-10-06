import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  compareWithTolerance,
  getTestCases,
} from "../../../test/synthetic/synthetic-test-utils";
import {
  decodeTileColumns,
  isPresent,
  type MltColumnLayer,
  type MltNamedColumn,
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
    expect(layer.mValues).toEqual([]);
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
    expect([0, 1, 2, 3, 4].map((i) => isPresent(ids, i))).toEqual([
      true,
      true,
      false,
      true,
      true,
    ]);
  });

  it("marks absent property values in a bitmap and leaves their slots at 0", () => {
    const [val] = onlyLayer("0x02/presence_bitmap").properties;
    expect(val.name).toBe("val");
    expect(Array.from(val.values)).toEqual([0, 0, 2, 0, 4, 0, 6, 0]);
    expect(Array.from(val.present ?? [])).toEqual([0b0101_0101]);
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

type Expected = {
  features: { properties: Record<string, unknown> }[];
};

/** Each m-value fixture's expected `m:` arrays, one value per vertex the feature stores. */
describe("m-values line up with vertices", () => {
  expect.addEqualityTesters([compareWithTolerance]);
  const cases = getTestCases([]).active.filter(({ name }) =>
    /^0x02\/(z_)?mvalues/.test(name),
  );
  it("covers the m-value fixtures", () => {
    expect(cases.length).toBeGreaterThan(30);
  });
  for (const { name, content, fileName } of cases) {
    it(name, () => {
      const expected = (content as Expected).features;
      const { layers } = decodeTileColumns(
        new Uint8Array(readFileSync(fileName)),
      );
      let next = 0;
      for (const layer of layers) {
        for (let f = 0; f < layer.featureCount; f++) {
          const { properties } = expected[next++];
          const [start, end] = vertexRange(layer, f);
          const expectedKeys = Object.keys(properties).filter((k) =>
            k.startsWith("m:"),
          );
          const presentKeys = layer.mValues
            .filter((column) => isPresent(column, f))
            .map((column) => `m:${column.name}`);
          expect(presentKeys.sort()).toEqual(expectedKeys.sort());
          for (const column of layer.mValues) {
            const key = `m:${column.name}`;
            if (!isPresent(column, f)) {
              expect(properties[key]).toBeUndefined();
              continue;
            }
            const actual = mValues(column, start, end);
            expect(actual).toEqual(properties[key]);
          }
        }
      }
      expect(next).toBe(expected.length);
      expect(layers.some((layer) => layer.mValues.length > 0)).toBe(true);
    });
  }
});

/** The vertices feature `f` owns, by descending every offset level the layer has. */
function vertexRange(layer: MltColumnLayer, f: number): [number, number] {
  const { geometryOffsets, partOffsets, ringOffsets } = layer.geometry;
  let [start, end] = [f, f + 1];
  for (const offsets of [geometryOffsets, partOffsets, ringOffsets]) {
    if (offsets !== undefined) [start, end] = [offsets[start], offsets[end]];
  }
  return [start, end];
}

/**
 * The values of `column` over vertices `start..end`. `compareWithTolerance` matches the
 * fixtures' NaN and infinity spellings, so only bools need converting.
 */
function mValues(
  column: MltNamedColumn,
  start: number,
  end: number,
): unknown[] {
  return Array.from(column.values.slice(start, end), (value) =>
    column.type === "bool" ? value === 1 : value,
  );
}
