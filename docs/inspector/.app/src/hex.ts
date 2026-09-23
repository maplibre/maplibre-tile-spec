/** Hex-map arithmetic, which is view state the wasm never sees. */

import type { DumpTree, Region } from "./annotate.ts";

/** Label of the synthetic leaf covering the bytes a bailed-out walk never reached. */
export const UNANNOTATED = "<unannotated>";

/** Bytes of a data blob the map draws brightly, which is enough to find the blob and not read it. */
export const MAX_BLOB = 1;

/** The render knobs plus the layer filter, which is the only one the wasm sees. */
export interface ViewState {
  /** Hex columns per row, fitted to the pane rather than chosen. */
  width: number;
  /** Whether the map and the tree tint each section. */
  colorful: boolean;
  layer: number | null;
}

/** The CLI's defaults, with `width` replaced by the fitted column count as soon as the pane is measured. */
export function defaultView(): ViewState {
  return {
    width: 16,
    colorful: false,
    layer: null,
  };
}

export function hex2(byte: number): string {
  return byte.toString(16).padStart(2, "0");
}

/** Offset in hex, padded to four digits or to as many as the last byte of a larger tile needs. */
export function hexOffset(offset: number, bufLen: number): string {
  const digits = Math.max(4, Math.max(0, bufLen - 1).toString(16).length);
  return offset.toString(16).padStart(digits, "0");
}

export function printable(byte: number): string {
  return byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : ".";
}

/** Row height in pixels, which HexMap's stylesheet pins and its virtualizer counts in. */
export const ROW = 26;

/** Width of one hex cell, and of one ascii glyph, at the map's 0.74rem mono. */
const CELL = 23.2;
const GLYPH = 7.1;

/** The offset gutter, the two column gaps and the map's own padding, measured at 8 columns.
 * Only what sits beside the bytes inside the left pane - the sidebar is not this pane's. */
const CHROME = 84;

/**
 * Hex columns that fill the map pane's `paneWidth` pixels.
 * A terminal is 80 columns wide, a browser is not, so the row follows the pane instead.
 */
export function fitColumns(paneWidth: number): number {
  const cols = (paneWidth - CHROME) / (CELL + GLYPH);
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

/** Block tints the map and the tree cycle over, which is enough that neighbouring bands differ. */
export const BLOCKS = 6;

/**
 * Band per region, counted over the containers in pre-order.
 * A leaf takes the band of the container holding it, so a block and its scalars paint as one.
 * The count does not wrap, so two bands that touch stay apart once the tint has cycled.
 */
export function regionBands(tree: DumpTree): Int32Array {
  const bands = new Int32Array(tree.regions.length).fill(-1);
  const open: number[] = [];
  let next = 0;
  tree.regions.forEach((region, index) => {
    if (region.container) {
      open[region.depth] = next++;
      bands[index] = open[region.depth];
    } else if (region.depth > 0) {
      bands[index] = open[region.depth - 1] ?? -1;
    }
  });
  return bands;
}

/** Palette slot of a band, or -1 for a region no container holds. */
export function bandTint(band: number): number {
  return band < 0 ? -1 : band % BLOCKS;
}

/** Containers that only group indexed siblings, whose own label a path would repeat. */
const GROUPINGS = new Set(["column data", "columns", "m_values", "header"]);

/** Enclosing container labels, outermost first, without the pure groupings. */
export function regionPath(regions: Region[], index: number): string[] {
  return ancestors(regions, index)
    .map((at) => regions[at].label)
    .filter((label) => !GROUPINGS.has(label));
}

/** The same path as one name, which is how the hover tip and the detail pane title a region. */
export function regionDotPath(regions: Region[], index: number): string {
  return [...regionPath(regions, index), regions[index].label].join(".");
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
export function fadedFrom(region: Region): number {
  if (region.kind !== "dataBlob") return Number.POSITIVE_INFINITY;
  return region.offset + MAX_BLOB;
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

/** Where the pointer is in the viewport, which is what places the hover tip. */
export interface Pointer {
  x: number;
  y: number;
}

/** Gap the tip keeps from the pointer and from every edge. */
const TIP_GAP = 14;

/**
 * Top-left corner of the hover tip, kept inside the viewport and clear of the pointer.
 * A tip that would hang off the bottom sits above the pointer instead, since sliding it up would cover the byte.
 */
export function tipPlacement(
  pointer: Pointer,
  tip: { width: number; height: number },
  viewport: { width: number; height: number },
): Pointer {
  const right = viewport.width - tip.width - TIP_GAP;
  const below = pointer.y + TIP_GAP;
  return {
    x: Math.max(TIP_GAP, Math.min(pointer.x + TIP_GAP, right)),
    y:
      below + tip.height > viewport.height
        ? Math.max(TIP_GAP, pointer.y - TIP_GAP - tip.height)
        : below,
  };
}
