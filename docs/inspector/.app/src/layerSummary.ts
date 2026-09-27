/** What a layer says about itself, read back off the regions the walker annotated. */

import type { Region } from "./annotate.ts";

export interface LayerColumn {
  /** Empty for a type that carries no name on the wire, such as `Geometry`. */
  name: string;
  type: string;
}

export interface LayerSummary {
  /** Size of the whole layer, framing included. */
  len: number;
  tag: string;
  extent: string | null;
  /** Null when a partial walk never reached what counts the features. */
  features: number | null;
  columns: LayerColumn[];
}

const COLUMN = /^column\[\d+] (\S+)(?: "(.*)")?$/;
const GEOMETRY = /^column\[\d+] Geometry$/;

/** The summary of the layer at `index`, or null for a region that is not a layer. */
export function layerSummary(
  regions: Region[],
  index: number,
): LayerSummary | null {
  const layer = regions[index];
  if (!layer?.container || !layer.label.startsWith("layer[")) return null;
  const kids = children(regions, index);
  const value = (label: string) =>
    regions[kids.find((at) => regions[at].label === label) ?? -1]?.value ??
    null;

  const columns: LayerColumn[] = [];
  for (const at of kids) {
    const parts = COLUMN.exec(regions[at].label);
    if (parts) columns.push({ name: parts[2] ?? "", type: parts[1] });
  }

  return {
    len: layer.len,
    tag: (value("tag") ?? "").split("-> ")[1] ?? "unknown",
    extent: value("extent") ?? extentOf(value("header")),
    features: featureCount(regions, kids, value("feature_count")),
    columns,
  };
}

/** Indices of the regions directly under `index`, skipping whatever they hold. */
function children(regions: Region[], index: number): number[] {
  const depth = regions[index].depth;
  const out: number[] = [];
  for (
    let at = index + 1;
    at < regions.length && regions[at].depth > depth;
    at++
  ) {
    if (regions[at].depth === depth + 1) out.push(at);
  }
  return out;
}

/** v2 packs the extent into the header byte rather than spelling it out. */
function extentOf(header: string | null): string | null {
  return /extent = (\d+)/.exec(header ?? "")?.[1] ?? null;
}

/**
 * v2 states the count; v1 leaves it to the geometry types, one per feature, so the
 * stream that carries them is what has to be asked.
 */
function featureCount(
  regions: Region[],
  kids: number[],
  stated: string | null,
): number | null {
  if (stated !== null) return Number(stated);
  const geometry = kids.find((at) => GEOMETRY.test(regions[at].label));
  if (geometry === undefined) return null;
  const meta = children(regions, geometry).find(
    (at) => regions[at].label === "meta",
  );
  return meta === undefined ? null : streamValues(regions, meta);
}

/** How many values a stream decodes to: an RLE header counts its runs, not its values. */
function streamValues(regions: Region[], stream: number): number | null {
  let count: number | null = null;
  for (const at of children(regions, stream)) {
    const { label, value } = regions[at];
    if (value === null) continue;
    if (label === "num_rle_values") return Number(value);
    if (label === "num_values") count = Number(value);
  }
  return count;
}
