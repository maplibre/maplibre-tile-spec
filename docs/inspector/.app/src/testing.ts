/** Hand-built trees for the component tests, in the shape the wasm hands over. */

import type { DumpTree, Region } from "./annotate.ts";

/** jsdom 30 ships `<dialog>` without `showModal` and `close`, so the fixture sheet needs them. */
export function stubDialog(): void {
  HTMLDialogElement.prototype.showModal = function showModal(
    this: HTMLDialogElement,
  ) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function close(this: HTMLDialogElement) {
    this.open = false;
  };
}

export function region(
  fields: Pick<Region, "offset" | "len" | "label"> & Partial<Region>,
): Region {
  return {
    depth: 0,
    value: null,
    bits: [],
    kind: "meta",
    container: false,
    blob: null,
    ...fields,
  };
}

/** One layer of eight bytes, three leaves deep, with a data blob at the end. */
export function tinyTree(last = "data"): DumpTree {
  return {
    bufLen: 8,
    regions: [
      region({ offset: 0, len: 8, label: "layer[0]", container: true }),
      region({ offset: 0, len: 2, label: "name", depth: 1, value: "water" }),
      region({
        offset: 2,
        len: 6,
        label: "geometry",
        depth: 1,
        container: true,
      }),
      region({ offset: 2, len: 1, label: "encoding", depth: 2 }),
      region({ offset: 3, len: 5, label: last, depth: 2, kind: "dataBlob" }),
    ],
  };
}

export const TINY_BYTES = new Uint8Array([
  0x77, 0x61, 0x20, 0x01, 0x02, 0x03, 0x04, 0x05,
]);

/** Six sibling containers under one layer, which is one more than the six-slot tint has. */
export function wrappedTree(): DumpTree {
  return {
    bufLen: 7,
    regions: [
      region({ offset: 0, len: 7, label: "layer[0]", container: true }),
      ...Array.from({ length: 6 }, (_, at) => [
        region({
          offset: at,
          len: 1,
          label: `column[${at}]`,
          depth: 1,
          container: true,
        }),
        region({ offset: at, len: 1, label: "present", depth: 2 }),
      ]).flat(),
      region({ offset: 6, len: 1, label: "trailer", depth: 1 }),
    ],
  };
}

export const WRAPPED_BYTES = new Uint8Array(7).fill(0x41);
