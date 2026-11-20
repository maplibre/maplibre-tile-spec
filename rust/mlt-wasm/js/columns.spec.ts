import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  columnValue,
  decodeTileColumns,
  type MltColumnLayer,
  toElevation,
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
    expect(layer.geometry.heightMode).toBeUndefined();
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
    expect(geometry.heightMode).toBe("absolute");
    expect(Array.from(geometry.vertices)).toEqual([13, 42, 120]);
  });

  it("reads bit 3 of the z axis byte as relativeToGround", () => {
    const { geometry } = onlyLayer("0x02/z_height_relative_to_ground");
    if (geometry.dimension !== 3) throw new Error("expected 3D vertices");
    expect(geometry.heightMode).toBe("relativeToGround");
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
  it("scales by the step from sea level", () => {
    expect(toElevation(0, 0)).toBe(0);
    expect(toElevation(-994, 0)).toBe(-994);
    expect(toElevation(12, 1)).toBe(120);
  });

  it("keeps a fine grid's decimals exact", () => {
    expect(toElevation(1234, -2)).toBe(12.34);
    expect(toElevation(12345, -3)).toBe(12.345);
  });
});

function varint(value: number): Uint8Array {
  const bytes: number[] = [];
  let n = value >>> 0;
  while (n >= 0x80) {
    bytes.push((n & 0x7f) | 0x80);
    n >>>= 7;
  }
  bytes.push(n);
  return Uint8Array.from(bytes);
}

/**
 * A ~68 KB v2 Point layer of `features` points at (0, 0) referencing one `entryBytes`-byte
 * dictionary entry in every feature, as a `Str` property, a `SharedDict` property, or an m-value.
 */
function repeatedDictTile(
  kind: "property" | "sharedDict" | "mValue",
  features: number,
  entryBytes: number,
): Uint8Array {
  const name = new TextEncoder().encode("layer1");
  const vertices = new Uint8Array(features * 2);
  const entryLen = varint(entryBytes);
  const dictLengths = concat(
    Uint8Array.of(0x88, 0x01),
    varint(entryLen.length),
    entryLen,
  );
  const dictValues = concat(
    Uint8Array.of(0x04),
    varint(entryBytes),
    new Uint8Array(entryBytes).fill(0x41),
  );
  const rlePayload = concat(varint(features), Uint8Array.of(0x00));
  const codes = concat(
    Uint8Array.of(0x21),
    varint(rlePayload.length),
    rlePayload,
  );

  let columns: Uint8Array;
  if (kind === "property") {
    columns = concat(
      Uint8Array.of(0x01, 0x0b, 0x03, 0x76, 0x61, 0x6c),
      codes,
      dictLengths,
      dictValues,
    );
  } else if (kind === "sharedDict") {
    columns = concat(
      Uint8Array.of(0x01, 0x0f, 0x02, 0x70, 0x3a, 0x01),
      dictLengths,
      dictValues,
      Uint8Array.of(0x0b, 0x01, 0x61),
      codes,
    );
  } else {
    // 1 LineString of `features` vertices so the m-value column carries `features` codes.
    const partLen = varint(features);
    const geom = concat(
      Uint8Array.of(0x04, 0x08),
      varint(partLen.length),
      partLen,
      Uint8Array.of(0xa8),
      varint(features),
      varint(vertices.length),
      vertices,
    );
    const mCodes = concat(
      Uint8Array.of(0xa1),
      varint(features),
      varint(rlePayload.length),
      rlePayload,
    );
    const body = concat(
      Uint8Array.of(0x02, name.length),
      name,
      Uint8Array.of(0xa0, 0x01),
      geom,
      Uint8Array.of(0x02, 0x0b, 0x03, 0x76, 0x61, 0x6c),
      mCodes,
      dictLengths,
      dictValues,
    );
    return concat(varint(body.length), body);
  }

  const body = concat(
    Uint8Array.of(0x02, name.length),
    name,
    Uint8Array.of(0x10),
    varint(features),
    Uint8Array.of(0x00, 0x28),
    varint(vertices.length),
    vertices,
    columns,
  );
  return concat(varint(body.length), body);
}

describe("memory budget", () => {
  // 1,100 * 64 KiB = 70.4 MiB expanded (> 64 MiB Decoder default), in a ~68 KB tile.
  const features = 1100;
  const entryBytes = 65_536;

  it("rejects repeated dictionary property strings whose expanded size exceeds 64 MiB", () => {
    const tile = repeatedDictTile("property", features, entryBytes);
    expect(() => decodeTileColumns(tile)).toThrow(/memory limit exceeded/i);
  });

  it("rejects repeated shared-dictionary property strings whose expanded size exceeds 64 MiB", () => {
    const tile = repeatedDictTile("sharedDict", features, entryBytes);
    expect(() => decodeTileColumns(tile)).toThrow(/memory limit exceeded/i);
  });

  it("keeps repeated dictionary string m-values coded within the budget", () => {
    const tile = repeatedDictTile("mValue", features, entryBytes);
    const [layer] = decodeTileColumns(tile).layers;
    const [col] = layer.vertexValues ?? [];
    expect(col?.type).toBe("string");
    if (col?.type !== "string") return;
    expect(col.dictionary).toHaveLength(1);
    expect(col.dictionary[0]).toHaveLength(entryBytes);
    expect(col.codes).toHaveLength(features);
  });
});
