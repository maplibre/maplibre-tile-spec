/** What the geometry panel reads out of the GeoJSON `tileGeoJson` hands back. */

import type { Feature, FeatureCollection, Geometry } from "geojson";
import type {
  MltColumnLayer,
  MltColumnTile,
  MltMValueColumn,
} from "./annotate.ts";

/** `from_layers` tags every feature with the layer it came from and that layer's extent. */
const LAYER = "_layer";
const EXTENT = "_extent";
/** Prefix `from_layers` gives a vertex-scoped column, whose value is one entry per vertex. */
const M_PREFIX = "m:";

export function layerOf(feature: Feature): string {
  return String(feature.properties?.[LAYER] ?? "");
}

/** The tile's coordinate space, which is the widest extent any layer declares. */
export function extentOf(tile: FeatureCollection): number {
  const extents = tile.features
    .map((f) => Number(f.properties?.[EXTENT]))
    .filter((extent) => Number.isFinite(extent) && extent > 0);
  return extents.length === 0 ? 4096 : Math.max(...extents);
}

/** Layer names in the order the tile carries them, which is the order the wire has them. */
export function layersOf(tile: FeatureCollection): string[] {
  return [...new Set(tile.features.map(layerOf))];
}

/** Every vertex of a geometry, however deeply its coordinates are nested. */
export function vertexCount(geometry: Geometry): number {
  const walk = (node: unknown): number => {
    if (!Array.isArray(node)) return 0;
    if (typeof node[0] === "number") return 1;
    return node.reduce<number>((total, child) => total + walk(child), 0);
  };
  return "coordinates" in geometry ? walk(geometry.coordinates) : 0;
}

/** One stretch of vertices an m-value column holds the same value over. */
export interface Run {
  from: number;
  /** Inclusive, so a single-vertex run has `from === to`. */
  to: number;
  value: unknown;
}

/** The runs of equal values in `values`, which is what makes a per-vertex column readable. */
export function runsOf(values: unknown[]): Run[] {
  const runs: Run[] = [];
  values.forEach((value, at) => {
    const last = runs.at(-1);
    if (last !== undefined && Object.is(last.value, value)) last.to = at;
    else runs.push({ from: at, to: at, value });
  });
  return runs;
}

/** One vertex-scoped column of one feature, as the popup lists it. */
export interface MColumn {
  name: string;
  runs: Run[];
}

/** Everything the hover popup says about a feature. */
export interface FeatureFacts {
  layer: string;
  type: string;
  /** Left out for a plain Point, whose count is always one. */
  vertices: number | null;
  properties: [name: string, value: unknown][];
  mValues: MColumn[];
}

export function factsOf(feature: Feature): FeatureFacts {
  const properties: [string, unknown][] = [];
  const mValues: MColumn[] = [];
  for (const [name, value] of Object.entries(feature.properties ?? {})) {
    if (name === LAYER || name === EXTENT) continue;
    if (name.startsWith(M_PREFIX) && Array.isArray(value))
      mValues.push({ name: name.slice(M_PREFIX.length), runs: runsOf(value) });
    else properties.push([name, value]);
  }
  const type = feature.geometry.type;
  return {
    layer: layerOf(feature),
    type,
    vertices: type === "Point" ? null : vertexCount(feature.geometry),
    properties,
    mValues,
  };
}

/** The app's block palette, reused so a geometry type is tinted like a hex container. */
const HUES: Record<string, number> = {
  Point: 0,
  MultiPoint: 1,
  LineString: 2,
  MultiLineString: 3,
  Polygon: 4,
  MultiPolygon: 5,
};

export function hueOf(geometry: Geometry): number {
  return HUES[geometry.type] ?? 0;
}

/** The number of hues in the block palette, which category hues cycle through. */
const PALETTE_SIZE = 6;

/** Value `at` of an m-value column, a string column's resolved through its dictionary. */
function valueAt(column: MltMValueColumn, at: number): unknown {
  return column.type === "string"
    ? column.dictionary[column.codes[at]]
    : column.values[at];
}

/** Whether an m-value column holds one value per vertex or one per triangle of a mesh. */
export type MValueScope = "vertex" | "triangle";

/** An m-value column a 3D view can colour its meshes by. */
export interface MeshColumn {
  name: string;
  scope: MValueScope;
}

/** The `Meshes` layers of the tile, or only those named `layer` when one is scoped. */
function meshLayers(tile: MltColumnTile, layer: string | null) {
  return tile.layers.filter(
    (each) =>
      each.geometry.layout === "meshes" &&
      (layer === null || each.name === layer),
  );
}

/** The m-value columns of the meshes in scope, triangle columns first, each name and scope once. */
export function meshColumns(
  tile: MltColumnTile,
  layer: string | null,
): MeshColumn[] {
  const columns: MeshColumn[] = [];
  const layers = meshLayers(tile, layer);
  for (const scope of ["triangle", "vertex"] as const)
    for (const each of layers)
      for (const { name } of columnsOf(each, scope))
        if (!columns.some((c) => c.name === name && c.scope === scope))
          columns.push({ name, scope });
  return columns;
}

function columnsOf(layer: MltColumnLayer, scope: MValueScope) {
  return scope === "triangle" ? layer.triangleValues : layer.vertexValues;
}

/**
 * One hue per triangle corner of each `Meshes` layer holding `column`.
 * Values are numbered tile-wide in first-seen order, so a value keeps its hue across meshes.
 * A corner of a mesh without values in the column gets `PALETTE_SIZE`, the hue past the palette.
 */
function categorise(
  tile: MltColumnTile,
  column: MeshColumn | null,
): {
  hues: Map<MltColumnLayer, number[]>;
  values: unknown[];
} {
  const categories = new Map<unknown, number>();
  const hues = new Map<MltColumnLayer, number[]>();
  if (column === null) return { hues, values: [] };
  const hueOf = (value: unknown) => {
    let category = categories.get(value);
    if (category === undefined) {
      category = categories.size;
      categories.set(value, category);
    }
    return category % PALETTE_SIZE;
  };
  for (const layer of tile.layers) {
    const { layout, triangleOffsets, geometryOffsets, indexBuffer } =
      layer.geometry;
    const values = columnsOf(layer, column.scope).find(
      (c) => c.name === column.name,
    );
    if (layout !== "meshes" || !triangleOffsets || !indexBuffer || !values)
      continue;
    const own: number[] = [];
    for (let f = 0; f < layer.featureCount; f++) {
      const first = triangleOffsets[f];
      const count = triangleOffsets[f + 1] - first;
      const start = values.offsets[f];
      const held = values.offsets[f + 1] - start;
      if (column.scope === "triangle") {
        for (let t = 0; t < count; t++) {
          const hue =
            held === count ? hueOf(valueAt(values, start + t)) : PALETTE_SIZE;
          own.push(hue, hue, hue);
        }
        continue;
      }
      const base = geometryOffsets?.[f] ?? 0;
      const vertices = (geometryOffsets?.[f + 1] ?? 0) - base;
      for (let at = first * 3; at < (first + count) * 3; at++)
        own.push(
          held === vertices
            ? hueOf(valueAt(values, start + indexBuffer[at] - base))
            : PALETTE_SIZE,
        );
    }
    hues.set(layer, own);
  }
  return { hues, values: [...categories.keys()] };
}

/** Whether the tile has a `Meshes` layer, named `layer` when one is given. */
export function hasMeshes(tile: MltColumnTile, layer: string | null): boolean {
  return meshLayers(tile, layer).length > 0;
}

/** Positions and per-corner hues of the meshes a 3D view draws. */
export interface MeshScene {
  /** Three corners per triangle, each `x, height, y` in tile units, so `y` grows south as in the tile. */
  positions: Float32Array;
  /** One per corner, by its value in the picked column, or `PALETTE_SIZE` for a corner without one. */
  hues: Uint8Array;
  /** Each value of the picked column and the hue it is drawn in. */
  legend: { value: string; hue: number }[];
}

/** The tile's coordinate space, which is the widest extent any of its layers declares. */
export function widestExtent(tile: MltColumnTile): number {
  return (
    tile.layers.reduce((widest, l) => Math.max(widest, l.extent), 0) || 4096
  );
}

/**
 * The triangles of every `Meshes` layer, or only of `layer` when one is scoped, coloured by `column`.
 * Every layer is brought onto `widestExtent`, whose unit `metresPerUnit` measures.
 */
export function meshScene(
  tile: MltColumnTile,
  layer: string | null,
  metresPerUnit: number,
  column: MeshColumn | null,
): MeshScene {
  const { hues, values } = categorise(tile, column);
  const extent = widestExtent(tile);
  const positions: number[] = [];
  const cornerHue: number[] = [];
  for (const each of meshLayers(tile, layer)) {
    const geometry = each.geometry;
    if (geometry.dimension !== 3) continue;
    const { vertices, indexBuffer = new Uint32Array(), zStep } = geometry;
    const step = 10 ** zStep;
    const scale = extent / each.extent;
    const own = hues.get(each);
    indexBuffer.forEach((vertex, corner) => {
      const at = vertex * 3;
      positions.push(
        vertices[at] * scale,
        (vertices[at + 2] * step) / metresPerUnit,
        vertices[at + 1] * scale,
      );
      cornerHue.push(own?.[corner] ?? PALETTE_SIZE);
    });
  }
  return {
    positions: Float32Array.from(positions),
    hues: Uint8Array.from(cornerHue),
    legend: values.map((value, at) => ({
      value: String(value),
      hue: at % PALETTE_SIZE,
    })),
  };
}

/** The Earth's circumference at the equator in metres, which Web Mercator tiles divide. */
const EARTH_CIRCUMFERENCE = 40_075_016.686;

/**
 * Ground metres per tile unit, read from a `z/x/y.mlt` address at the tile's centre latitude.
 * An address naming no tile, such as a synthetic fixture, takes a unit as a metre.
 */
export function metresPerUnit(address: string | null, extent: number): number {
  if (address === null) return 1;
  let path = address;
  try {
    path = new URL(address, "http://localhost/").pathname;
  } catch {
    // Not a URL, so the address is taken as a path.
  }
  const match = path.match(/(\d+)\/\d+\/(\d+)\.mlt$/);
  if (!match) return 1;
  const tiles = 2 ** Number(match[1]);
  const north = Math.PI * (1 - (2 * (Number(match[2]) + 0.5)) / tiles);
  const latitude = Math.atan(Math.sinh(north));
  return (EARTH_CIRCUMFERENCE * Math.cos(latitude)) / tiles / extent;
}
