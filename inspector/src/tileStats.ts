/** What a tile spends its bytes on, read off the annotated regions and the decoded features. */

import type { FeatureCollection } from "geojson";
import type { DumpTree, Region } from "./annotate.ts";
import { layerOf, vertexCount } from "./geometry.ts";
import { layerSummary } from "./layerSummary.ts";

export const CATEGORIES = [
  "geometry",
  "properties",
  "ids",
  "metadata",
] as const;
export type Category = (typeof CATEGORIES)[number];

export interface ColumnStat {
  name: string;
  type: string;
  len: number;
  category: Category;
}

export interface LayerStat {
  /** Null for a layer whose label spells no name. */
  name: string | null;
  len: number;
  features: number | null;
  extent: string | null;
  /** Metadata is what is left of the layer once every column is taken out. */
  bytes: Record<Category, number>;
  columns: ColumnStat[];
}

export interface TileStat {
  /** Size of the whole buffer, which the layers add up to unless the walk stopped early. */
  total: number;
  /** Bytes no layer claims, which only a walk that bailed out leaves behind. */
  unannotated: number;
  layers: LayerStat[];
  bytes: Record<Category, number>;
}

const NAMED = /^layer\[\d+] "(.*)"$/;
const COLUMN = /^(?:column|m_value)\[\d+] (\S+)(?: "(.*)")?$/;
const ID = /^(?:Opt)?(?:Long)?Id$/;

function zero(): Record<Category, number> {
  return { geometry: 0, properties: 0, ids: 0, metadata: 0 };
}

/** The size of every layer, split by what the bytes are for. */
export function tileStat(tree: DumpTree): TileStat {
  const { regions } = tree;
  const layers: LayerStat[] = [];
  regions.forEach((layer, at) => {
    if (layer.depth !== 0 || !layer.container) return;
    if (!layer.label.startsWith("layer[")) return;
    layers.push(layerStat(regions, at));
  });

  const bytes = zero();
  let claimed = 0;
  for (const layer of layers) {
    claimed += layer.len;
    for (const category of CATEGORIES) bytes[category] += layer.bytes[category];
  }
  return {
    total: tree.bufLen,
    unannotated: Math.max(0, tree.bufLen - claimed),
    layers,
    bytes,
  };
}

function layerStat(regions: Region[], at: number): LayerStat {
  const layer = regions[at];
  const summary = layerSummary(regions, at);
  const bytes = zero();
  const columns: ColumnStat[] = [];
  for (const kid of childrenOf(regions, at)) {
    const column = classify(regions[kid]);
    if (column === null) continue;
    columns.push(column);
    bytes[column.category] += column.len;
  }
  bytes.metadata = layer.len - bytes.geometry - bytes.properties - bytes.ids;
  return {
    name: NAMED.exec(layer.label)?.[1] ?? null,
    len: layer.len,
    features: summary?.features ?? null,
    extent: summary?.extent ?? null,
    bytes,
    columns,
  };
}

/** A container directly under a layer that holds one column's bytes, or null for framing. */
function classify(region: Region): ColumnStat | null {
  if (!region.container) return null;
  if (region.label === "geometry")
    return {
      name: "",
      type: "Geometry",
      len: region.len,
      category: "geometry",
    };
  const parts = COLUMN.exec(region.label);
  if (parts === null) return null;
  const [, type, name = ""] = parts;
  const category: Category =
    type === "Geometry" ? "geometry" : ID.test(type) ? "ids" : "properties";
  return { name, type, len: region.len, category };
}

function childrenOf(regions: Region[], index: number): number[] {
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

export interface GeoStat {
  features: number;
  vertices: number;
  /** Feature count per GeoJSON geometry type, in the order each first appears. */
  types: Map<string, number>;
}

/** Feature, vertex and geometry-type counts per layer name, plus the tile as a whole. */
export function geoStat(tile: FeatureCollection): {
  whole: GeoStat;
  layers: Map<string, GeoStat>;
} {
  const fresh = (): GeoStat => ({ features: 0, vertices: 0, types: new Map() });
  const whole = fresh();
  const layers = new Map<string, GeoStat>();
  for (const feature of tile.features) {
    const name = layerOf(feature);
    let layer = layers.get(name);
    if (layer === undefined) {
      layer = fresh();
      layers.set(name, layer);
    }
    const vertices = vertexCount(feature.geometry);
    const type = feature.geometry.type;
    for (const into of [whole, layer]) {
      into.features += 1;
      into.vertices += vertices;
      into.types.set(type, (into.types.get(type) ?? 0) + 1);
    }
  }
  return { whole, layers };
}
