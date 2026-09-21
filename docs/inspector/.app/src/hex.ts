/** Hex-map arithmetic, which is view state the wasm never sees. */

import type { DumpTree, Region } from "./annotate.ts";

/** Label of the synthetic leaf covering the bytes a bailed-out walk never reached. */
export const UNANNOTATED = "<unannotated>";

/** How much of a data blob the map draws brightly, and what the detail pane decodes. */
export type DataMode = "both" | "blob" | "decoded" | "hidden";

/** Bytes of a data blob the map draws brightly, which is enough to find the blob and not read it. */
export const MAX_BLOB = 1;

/** The render knobs plus the layer filter, which is the only one the wasm sees. */
export interface ViewState {
  /** Hex columns per row, fitted to the pane rather than chosen. */
  width: number;
  dataMode: DataMode;
  layer: number | null;
}

/** The CLI's defaults, with `width` replaced by the fitted column count as soon as the pane is measured. */
export function defaultView(): ViewState {
  return {
    width: 16,
    dataMode: "both",
    layer: null,
  };
}

export function hex2(byte: number): string {
  return byte.toString(16).padStart(2, "0");
}

export function hex8(offset: number): string {
  return offset.toString(16).padStart(8, "0");
}

export function printable(byte: number): string {
  return byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : ".";
}

/**
 * Hex columns that fill a pane of `paneWidth` pixels, fitted in #15 at 976, 1300 and 1700 px.
 * A terminal is 80 columns wide, a browser is not, so the row follows the pane instead.
 */
export function fitColumns(paneWidth: number): number {
  const cols = (4 * (paneWidth - 800)) / 100 + 12;
  return Math.min(64, Math.max(8, Math.floor(cols / 4) * 4));
}

/** Leaf region index per byte, or -1 where the tree annotates nothing. */
export function byteOwners(tree: DumpTree): Int32Array {
  const owners = new Int32Array(tree.bufLen).fill(-1);
  tree.regions.forEach((region, index) => {
    if (region.container) return;
    owners.fill(index, region.offset, region.offset + region.len);
  });
  return owners;
}

/** Enclosing container labels, outermost first, found by walking back up the depths. */
export function regionPath(regions: Region[], index: number): string[] {
  const path: string[] = [];
  let depth = regions[index].depth;
  for (let at = index - 1; at >= 0 && depth > 0; at--) {
    const region = regions[at];
    if (region.container && region.depth < depth) {
      path.unshift(region.label);
      depth = region.depth;
    }
  }
  return path;
}

/** Indices of the containers `index` sits in, outermost first. */
export function ancestors(regions: Region[], index: number): number[] {
  const out: number[] = [];
  let depth = regions[index].depth;
  for (let at = index - 1; at >= 0 && depth > 0; at--) {
    const region = regions[at];
    if (region.container && region.depth < depth) {
      out.unshift(at);
      depth = region.depth;
    }
  }
  return out;
}

export function layerLabels(tree: DumpTree): string[] {
  return tree.regions
    .filter((r) => r.depth === 0 && r.container)
    .map((r) => r.label);
}

/** Offset from which a blob's bytes are drawn faded, which is what `data` means to a map that never drops a row. */
export function fadedFrom(region: Region, view: ViewState): number {
  if (region.kind !== "dataBlob") return Number.POSITIVE_INFINITY;
  if (view.dataMode === "hidden" || view.dataMode === "decoded")
    return region.offset;
  return region.offset + MAX_BLOB;
}

/** Whether the detail pane asks for decoded values at all. */
export function showsDecoded(view: ViewState): boolean {
  return view.dataMode === "both" || view.dataMode === "decoded";
}

/**
 * Index of each region of a filtered tree in the whole tree, which is what `decodeBlob` counts.
 * A filtered tree keeps pre-order, so one walk in step is enough and the filter itself stays in Rust.
 */
export function wholeIndices(whole: Region[], part: Region[]): Int32Array {
  const map = new Int32Array(part.length).fill(-1);
  let at = 0;
  part.forEach((region, index) => {
    while (at < whole.length && !sameRegion(whole[at], region)) at++;
    if (at < whole.length) map[index] = at++;
  });
  return map;
}

function sameRegion(a: Region, b: Region): boolean {
  return a.offset === b.offset && a.len === b.len && a.label === b.label;
}

/** The next leaf `step` away from `from`, since containers own no bytes of their own. */
export function leafStep(
  regions: Region[],
  from: number,
  step: number,
): number | null {
  for (let at = from + step; at >= 0 && at < regions.length; at += step) {
    if (!regions[at].container) return at;
  }
  return null;
}
