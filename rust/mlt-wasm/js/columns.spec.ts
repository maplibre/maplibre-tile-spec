import { readFileSync } from "node:fs";
import { describe, expect, expectTypeOf, it } from "vitest";
import {
  columnValue,
  type DecodeTileColumnsOptions,
  decodeTileColumns,
  type MltColumnLayer,
  type MltNestedColumn,
  toElevation,
} from "./columns";
import { featureGeometry } from "./featureGeometry";

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
  });

  it("marks absent property values in a bitmap and leaves their slots at 0", () => {
    const [val] = onlyLayer("0x02/presence_bitmap").properties;
    if (val.type !== "u32") throw new Error("expected a u32 column");
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

describe("exact 64-bit integers", () => {
  for (const version of ["0x01", "0x02"]) {
    for (const [name, kind, value] of [
      ["prop_i64_min", "i64", -9223372036854775808n],
      ["prop_i64_max", "i64", 9223372036854775807n],
      ["prop_u64_max", "u64", 18446744073709551615n],
    ] as const) {
      it(`preserves ${version}/${name} without changing numeric defaults`, () => {
        const data = fixture(`${version}/${name}`);
        const [exact] = decodeTileColumns(data, { int64: "bigint" }).layers;
        const [numeric] = decodeTileColumns(data).layers;
        const column = exact.properties[0];
        const rounded = numeric.properties[0];
        expect(column.type).toBe(kind);
        if (column.type !== "i64" && column.type !== "u64")
          throw new Error("expected a 64-bit integer column");
        if (rounded.type !== "i64" && rounded.type !== "u64")
          throw new Error("expected a 64-bit integer column");
        expectTypeOf(rounded.values).toEqualTypeOf<Float64Array>();
        expect(column.values).toBeInstanceOf(
          kind === "i64" ? BigInt64Array : BigUint64Array,
        );
        expect(Array.from(column.values)).toEqual([value]);
        expect(columnValue(column, 0)).toBe(value);
        expect(rounded.values).toBeInstanceOf(Float64Array);
        expect(columnValue(rounded, 0)).toBe(Number(value));
        expect(featureGeometry(exact, 0)).toEqual(featureGeometry(numeric, 0));
      });
    }

    it(`keeps nulls separate from the 0n placeholder in ${version}`, () => {
      const [layer] = decodeTileColumns(
        fixture(`${version}/prop_u64_null_val`),
        {
          int64: "bigint",
        },
      ).layers;
      const column = layer.properties[0];
      if (column.type !== "u64") throw new Error("expected a u64 column");
      expectTypeOf(column.values).toEqualTypeOf<BigUint64Array>();
      expect(Array.from(column.values)).toEqual([0n, 1234567890123456789n]);
      expect(Array.from(column.present ?? [])).toEqual([0b10]);
      expect(columnValue(column, 0)).toBeUndefined();
      expect(columnValue(column, 1)).toBe(1234567890123456789n);
    });

    it(`preserves unsigned id extremes and optional ids in ${version}`, () => {
      const [extremes] = decodeTileColumns(fixture(`${version}/ids64_minmax`), {
        int64: "bigint",
      }).layers;
      if (!extremes.ids) throw new Error("expected ids");
      expectTypeOf(extremes.ids.values).toEqualTypeOf<BigUint64Array>();
      expect(Array.from(extremes.ids.values)).toEqual([
        0n,
        18446744073709551615n,
        0n,
        18446744073709551615n,
      ]);
      const [optional] = decodeTileColumns(fixture(`${version}/ids64_opt`), {
        int64: "bigint",
      }).layers;
      if (!optional.ids) throw new Error("expected ids");
      expect(Array.from(optional.ids.values)).toEqual([
        0n,
        9234567890n,
        101n,
        105n,
        106n,
      ]);
      expect(columnValue(optional.ids, 0)).toBeUndefined();
      expect(columnValue(optional.ids, 1)).toBe(9234567890n);
      expect(columnValue(extremes.ids, 1)).toBe(18446744073709551615n);
    });
  }

  it("types a runtime option as either representation", () => {
    const options: DecodeTileColumnsOptions = { int64: "bigint" };
    const [layer] = decodeTileColumns(
      fixture("0x02/ids64_minmax"),
      options,
    ).layers;
    if (!layer.ids) throw new Error("expected ids");
    expectTypeOf(layer.ids.values).toEqualTypeOf<
      Float64Array | BigUint64Array
    >();
    expect(layer.ids.values).toBeInstanceOf(BigUint64Array);
  });

  it("accepts forwarded optional options and keeps omitted options numeric", () => {
    const data = fixture("0x02/ids64_minmax");
    const forward = (options?: DecodeTileColumnsOptions) =>
      decodeTileColumns(data, options);
    const [numeric] = decodeTileColumns(data).layers;
    const [explicitUndefined] = decodeTileColumns(data, undefined).layers;
    if (!numeric.ids || !explicitUndefined.ids) throw new Error("expected ids");
    expectTypeOf(numeric.ids.values).toEqualTypeOf<Float64Array>();
    expectTypeOf(explicitUndefined.ids.values).toEqualTypeOf<Float64Array>();
    expect(forward().layers[0].ids?.values).toBeInstanceOf(Float64Array);
    expect(forward({ int64: "bigint" }).layers[0].ids?.values).toBeInstanceOf(
      BigUint64Array,
    );
  });
});

function nestedColumn(name: string): MltNestedColumn {
  const [column] = onlyLayer(`0x02/${name}`).properties;
  if (column.type !== "nested") throw new Error("expected a nested column");
  return column;
}

describe("nested columns", () => {
  it("exports a struct schema and restores its declared null field", () => {
    const column = nestedColumn("nested_struct");
    expect(column.name).toBe("obj");
    expect(column.schema).toEqual({
      kind: "struct",
      fields: [
        { name: "name", schema: { kind: "leaf", type: "string" } },
        { name: "rank", schema: { kind: "leaf", type: "i32" } },
      ],
    });
    expect(column.values).toEqual([
      { name: "ab", rank: 7 },
      { name: "cd", rank: null },
      { name: "ef", rank: 9 },
    ]);
    expect(column.present).toBeUndefined();
    expect(columnValue(column, 1)).toEqual({ name: "cd", rank: null });
    expect(Object.getPrototypeOf(column.values[0])).toBeNull();
  });

  it("restores null fields for every row of a shape-coded struct", () => {
    const column = nestedColumn("nested_struct_shapes");
    expect(column.schema).toEqual({
      kind: "struct",
      fields: [
        ...["a", "b", "c"].map((name) => ({
          name,
          schema: { kind: "leaf", type: "i32" },
        })),
        ...["x", "y", "z"].map((name) => ({
          name,
          schema: { kind: "leaf", type: "string" },
        })),
      ],
    });
    expect(column.values).toEqual([
      ...Array.from({ length: 8 }, (_, i) => ({
        a: i + 1,
        b: i + 2,
        c: i + 3,
        x: null,
        y: null,
        z: null,
      })),
      ...Array.from({ length: 8 }, (_, i) => ({
        a: null,
        b: null,
        c: null,
        x: i % 2 ? "q" : "p",
        y: i % 2 ? "s" : "r",
        z: i % 2 ? "u" : "t",
      })),
    ]);
  });

  it("preserves lists and the integer leaf type inside their structs", () => {
    const column = nestedColumn("nested_list_struct");
    expect(column.schema).toEqual({
      kind: "list",
      element: {
        kind: "struct",
        fields: [
          { name: "id", schema: { kind: "leaf", type: "i32" } },
          { name: "tag", schema: { kind: "leaf", type: "string" } },
        ],
      },
    });
    expect(column.values).toEqual([
      [
        { id: 1, tag: "hi" },
        { id: 2, tag: "lo" },
      ],
      [{ id: 3, tag: "hi" }],
    ]);
    const [exact] = decodeTileColumns(fixture("0x02/nested_list_struct"), {
      int64: "bigint",
    }).layers;
    expect(exact.properties[0].values).toEqual(column.values);
  });

  it("exports maps without inventing keys from another row", () => {
    const plain = nestedColumn("nested_map_str");
    expect(plain.schema).toEqual({
      kind: "map",
      value: { kind: "leaf", type: "string" },
    });
    expect(plain.values).toEqual([
      { en: "sea", de: "meer" },
      { en: "hill", de: "berg" },
    ]);
    const shaped = nestedColumn("nested_map_shapes");
    expect(shaped.schema).toEqual(plain.schema);
    expect(shaped.values).toEqual([
      { de: "berg", en: "hill", fr: "mont" },
      { de: "see", fr: "lac" },
      { en: "sea", fr: "mer" },
      { de: "fluss", en: "river", fr: "riviere" },
    ]);
    expect(Object.hasOwn(shaped.values[1] as object, "en")).toBe(false);
  });

  it("owns values independently of later decodes and other rows", () => {
    const column = nestedColumn("nested_list_struct");
    const first = column.values[0];
    if (!Array.isArray(first)) throw new Error("expected a list");
    const item = first[0];
    if (!item || typeof item !== "object" || Array.isArray(item))
      throw new Error("expected a struct");
    item.tag = "changed";
    first.push(null);
    const again = nestedColumn("nested_list_struct");
    expect(again.values[0]).toEqual([
      { id: 1, tag: "hi" },
      { id: 2, tag: "lo" },
    ]);
    expect(column.values[1]).toEqual([{ id: 3, tag: "hi" }]);
    expect(item.tag).toBe("changed");
  });

  it("preserves null containers, null elements, map keys and exact nested integers", () => {
    const data = fixture("0x02/nested_nulls");
    const [exact] = decodeTileColumns(data, { int64: "bigint" }).layers;
    const [numeric] = decodeTileColumns(data).layers;
    const [obj, tags] = exact.properties;
    if (obj.type !== "nested" || tags.type !== "nested")
      throw new Error("expected nested columns");
    expect([obj.name, tags.name]).toEqual(["obj", "tags"]);
    expect(obj.schema).toEqual({
      kind: "struct",
      fields: [
        { name: "fixed", schema: { kind: "leaf", type: "string" } },
        { name: "__proto__", schema: { kind: "leaf", type: "string" } },
        { name: "signed", schema: { kind: "leaf", type: "i64" } },
        { name: "unsigned", schema: { kind: "leaf", type: "u64" } },
        {
          name: "items",
          schema: { kind: "list", element: { kind: "leaf", type: "i32" } },
        },
      ],
    });
    expect(obj.values).toEqual([
      {
        fixed: null,
        ["__proto__"]: "first",
        signed: -9223372036854775808n,
        unsigned: 18446744073709551615n,
        items: [7, null, 9],
      },
      null,
      {
        fixed: null,
        ["__proto__"]: "empty",
        signed: 0n,
        unsigned: 0n,
        items: [],
      },
      {
        fixed: null,
        ["__proto__"]: "last",
        signed: 9223372036854775807n,
        unsigned: 1n,
        items: null,
      },
    ]);
    expect(columnValue(obj, 1)).toBeNull();
    expect(tags.schema).toEqual({
      kind: "map",
      value: { kind: "leaf", type: "string" },
    });
    expect(tags.values).toEqual([
      { present: "value", nil: null },
      { ["__proto__"]: "safe" },
      {},
      null,
    ]);
    expect(Object.hasOwn(tags.values[0] as object, "nil")).toBe(true);
    expect(Object.hasOwn(tags.values[1] as object, "nil")).toBe(false);
    for (const value of [obj.values[0], tags.values[1]]) {
      expect(Object.getPrototypeOf(value)).toBeNull();
      expect(Object.hasOwn(value as object, "__proto__")).toBe(true);
    }
    expect(numeric.properties[0].values).toEqual([
      {
        fixed: null,
        ["__proto__"]: "first",
        signed: Number(-9223372036854775808n),
        unsigned: Number(18446744073709551615n),
        items: [7, null, 9],
      },
      null,
      {
        fixed: null,
        ["__proto__"]: "empty",
        signed: 0,
        unsigned: 0,
        items: [],
      },
      {
        fixed: null,
        ["__proto__"]: "last",
        signed: Number(9223372036854775807n),
        unsigned: 1,
        items: null,
      },
    ]);
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

describe("columnValue", () => {
  it("reads a feature's value, and undefined where it has none", () => {
    const [val] = onlyLayer("0x02/presence_bitmap").properties;
    // A stored 0 and an absent value differ only by the bitmap.
    expect([0, 1, 2, 3].map((i) => columnValue(val, i))).toEqual([
      0,
      undefined,
      2,
      undefined,
    ]);
  });

  it("rejects an index that is not a feature, rather than reading it as no value", () => {
    const [val] = onlyLayer("0x02/prop_i32").properties;
    expect(columnValue(val, 0)).toBe(42);
    for (const index of [-1, 1, 0.5, Number.NaN]) {
      expect(() => columnValue(val, index)).toThrow(RangeError);
    }
  });
});

describe("toElevation", () => {
  it("offsets by -10000 m and scales by the step", () => {
    expect(toElevation(10000, 0)).toBe(0);
    expect(toElevation(21000, 0)).toBe(11000);
    expect(toElevation(1012, 1)).toBe(120);
  });

  it("keeps a fine grid's decimals exact", () => {
    expect(toElevation(1001234, -2)).toBe(12.34);
    expect(toElevation(10012345, -3)).toBe(12.345);
  });
});
