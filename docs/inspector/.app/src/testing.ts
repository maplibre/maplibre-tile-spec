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
