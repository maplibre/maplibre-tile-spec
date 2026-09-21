import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  annotateTile,
  type DumpTree,
  type Region,
} from "./annotate";

const SYNTHETIC = new URL("../../../test/synthetic/", import.meta.url);

describe("annotateTile", () => {
  it("annotates a v1 tile", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    expect(tile.error).toBeNull();
    expect(tile.tree()).toMatchSnapshot();
  });

  it("annotates a v2 tile", async () => {
    const tile = annotateTile(await fixture("0x02/id.mlt"));
    expect(tile.error).toBeNull();
    expect(tile.tree()).toMatchSnapshot();
  });

  it.each([
    "0x01/id.mlt",
    "0x01/poly_hole.mlt",
    "0x01/props_shared_dict.mlt",
    "0x01/props_str.mlt",
    "0x02/id.mlt",
    "0x02/mvalues.mlt",
    "0x02/nested_list_struct.mlt",
    "0x02/nested_map_str.mlt",
    "0x02/nested_struct.mlt",
    "0x02/poly_hole.mlt",
    "0x02/prop_f64_alp.mlt",
    "0x02/props_sp.mlt",
    "0x02/props_str_dict.mlt",
  ])("leaves of %s partition the whole buffer", async (name) => {
    const tree = annotateTile(await fixture(name)).tree();
    expect(coveredBytes(tree)).toBe(tree.bufLen);
    expect(tree.bufLen).toBeGreaterThan(0);
  });
});

describe("a tile truncated mid-layer", () => {
  const truncated = async () => {
    const whole = await fixture("0x01/id.mlt");
    const cut = await fixture("0x01/props_str.mlt");
    return annotateTile(concat(whole, cut.subarray(0, cut.length - 1)));
  };

  it("carries the walker failure on the handle", async () => {
    expect((await truncated()).error).toMatchSnapshot();
  });

  it("keeps the regions the walk did reach", async () => {
    const tile = await truncated();
    expect(tile.tree().regions.map((r) => r.label)).toMatchSnapshot();
  });

  it("ends in an <unannotated> leaf that partitions the buffer", async () => {
    const tree = (await truncated()).tree();
    const last = tree.regions.at(-1) as Region;
    expect(last.label).toBe("<unannotated>");
    expect(last.container).toBe(false);
    expect(coveredBytes(tree)).toBe(tree.bufLen);
  });

  it("still filters to the layer that bailed", async () => {
    const tile = await truncated();
    expect(tile.tree(0).regions).toHaveLength(
      annotateTile(await fixture("0x01/id.mlt")).tree().regions.length,
    );
    expect(tile.tree(1).regions.map((r) => r.label)).toEqual([
      "layer[1]",
      "size",
      "tag",
    ]);
  });
});

describe("decodeBlob", () => {
  it("hands a u64 id across as a BigInt", async () => {
    const tile = annotateTile(await fixture("0x01-rust/id64_max_delta.mlt"));
    const decoded = tile.decodeBlob(blobIndex(tile.tree(), "u64"), 0);
    expect(decoded).toEqual({
      kind: "bigints",
      values: [18446744073709551615n],
      truncatedFrom: null,
    });
  });

  it("hands ALP parameters across as a BigInt", async () => {
    const tree = annotateTile(await fixture("0x02/prop_f64_alp.mlt")).tree();
    const hint = tree.regions[blobIndex(tree, "alp")].blob?.hint;
    expect(hint).toMatchSnapshot();
    expect(typeof (hint as { base: bigint }).base).toBe("bigint");
  });

  it("decodes a string payload as text", async () => {
    const tile = annotateTile(await fixture("0x01/props_str.mlt"));
    expect(
      tile.decodeBlob(blobIndex(tile.tree(), "bytes"), 0),
    ).toMatchSnapshot();
  });

  it("caps the values at maxValues and reports the full count", async () => {
    const tile = annotateTile(await fixture("0x01/poly_hole.mlt"));
    const index = blobIndex(tile.tree(), "i32");
    const all = tile.decodeBlob(index, 0);
    const capped = tile.decodeBlob(index, 2);
    expect(all).toMatchObject({ truncatedFrom: null });
    expect(capped).toMatchObject({
      values: (all as { values: number[] }).values.slice(0, 2),
      truncatedFrom: (all as { values: number[] }).values.length,
    });
  });

  it("reports a region that carries no stream metadata", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    expect(tile.decodeBlob(0, 0)).toEqual({
      kind: "error",
      message: "region 0 carries no stream metadata",
    });
  });

  it("throws for a region the tree does not have", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    expect(() => tile.decodeBlob(9999, 0)).toThrow(
      "the tree has no region 9999",
    );
  });
});

describe("tree", () => {
  it("filters to one layer", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    const layer = tile.tree(0);
    expect(layer.regions[0].label).toBe("layer[0]");
    expect(layer.regions.length).toBeLessThanOrEqual(
      tile.tree().regions.length,
    );
  });

  it("memoizes per layer argument", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    expect(tile.tree()).toBe(tile.tree());
    expect(tile.tree(0)).toBe(tile.tree(0));
    expect(tile.tree()).not.toBe(tile.tree(0));
  });

  it("throws for a layer the tile does not have", async () => {
    const tile = annotateTile(await fixture("0x01/id.mlt"));
    expect(() => tile.tree(1)).toThrow("the tile has no layer 1");
  });
});

function concat(head: Uint8Array, tail: Uint8Array): Uint8Array {
  const buf = new Uint8Array(head.length + tail.length);
  buf.set(head);
  buf.set(tail, head.length);
  return buf;
}

async function fixture(name: string): Promise<Uint8Array> {
  return new Uint8Array(await readFile(new URL(name, SYNTHETIC)));
}

function coveredBytes(tree: DumpTree): number {
  let next = 0;
  for (const region of tree.regions) {
    if (region.container) continue;
    expect(region.offset).toBe(next);
    next += region.len;
  }
  return next;
}

function blobIndex(tree: DumpTree, hint: string): number {
  const index = tree.regions.findIndex((r) => r.blob?.hint.kind === hint);
  expect(index, `a ${hint} blob`).toBeGreaterThanOrEqual(0);
  return index;
}
