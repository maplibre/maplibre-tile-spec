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

/**
 * One hue per triangle of each `Meshes` layer with a triangle column, from its first such column.
 * Values are numbered tile-wide in first-seen order, so a value keeps its hue across meshes.
 * A triangle of a mesh without values in the column gets `PALETTE_SIZE`, the hue past the palette.
 */
export function triangleHues(
  tile: MltColumnTile,
): Map<MltColumnLayer, number[]> {
  return categorise(tile).hues;
}

/** `triangleHues`, plus every triangle value in the order it was numbered. */
function categorise(tile: MltColumnTile): {
  hues: Map<MltColumnLayer, number[]>;
  values: unknown[];
} {
  const categories = new Map<unknown, number>();
  const hues = new Map<MltColumnLayer, number[]>();
  for (const layer of tile.layers) {
    const { layout, triangleOffsets } = layer.geometry;
    const column = layer.triangleValues[0];
    if (layout !== "meshes" || triangleOffsets === undefined || !column)
      continue;
    const own: number[] = [];
    for (let f = 0; f < layer.featureCount; f++) {
      const count = triangleOffsets[f + 1] - triangleOffsets[f];
      const start = column.offsets[f];
      const covered = column.offsets[f + 1] - start === count;
      for (let t = 0; t < count; t++) {
        if (!covered) {
          own.push(PALETTE_SIZE);
          continue;
        }
        const value = valueAt(column, start + t);
        let category = categories.get(value);
        if (category === undefined) {
          category = categories.size;
          categories.set(value, category);
        }
        own.push(category % PALETTE_SIZE);
      }
    }
    hues.set(layer, own);
  }
  return { hues, values: [...categories.keys()] };
}

/** Whether the tile has a `Meshes` layer, named `layer` when one is given. */
export function hasMeshes(tile: MltColumnTile, layer: string | null): boolean {
  return tile.layers.some(
    (each) =>
      each.geometry.layout === "meshes" &&
      (layer === null || each.name === layer),
  );
}

/** Positions and per-triangle hues of the meshes a 3D view draws. */
export interface MeshScene {
  /** Three corners per triangle, each `x, height, y` in tile units, so `y` grows south as in the tile. */
  positions: Float32Array;
  /** One per triangle, from `triangleHues`, or `PALETTE_SIZE` for a triangle without a triangle column. */
  hues: Uint8Array;
  /** Each triangle value of the tile and the hue it is drawn in. */
  legend: { value: string; hue: number }[];
}

/** The triangles of every `Meshes` layer, or only of `layer` when one is scoped. */
export function meshScene(
  tile: MltColumnTile,
  layer: string | null,
  metresPerUnit: number,
): MeshScene {
  const { hues, values } = categorise(tile);
  const positions: number[] = [];
  const triangleHue: number[] = [];
  for (const each of tile.layers) {
    if (layer !== null && each.name !== layer) continue;
    const geometry = each.geometry;
    if (geometry.layout !== "meshes" || geometry.dimension !== 3) continue;
    const { vertices, indexBuffer = new Uint32Array(), zStep } = geometry;
    const step = 10 ** zStep;
    const own = hues.get(each);
    for (let t = 0; t < indexBuffer.length / 3; t++) {
      for (let corner = 0; corner < 3; corner++) {
        const at = indexBuffer[t * 3 + corner] * 3;
        positions.push(
          vertices[at],
          (vertices[at + 2] * step) / metresPerUnit,
          vertices[at + 1],
        );
      }
      triangleHue.push(own?.[t] ?? PALETTE_SIZE);
    }
  }
  return {
    positions: Float32Array.from(positions),
    hues: Uint8Array.from(triangleHue),
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
  const match = address?.match(/(\d+)\/\d+\/(\d+)\.mlt$/);
  if (!match) return 1;
  const tiles = 2 ** Number(match[1]);
  const north = Math.PI * (1 - (2 * (Number(match[2]) + 0.5)) / tiles);
  const latitude = Math.atan(Math.sinh(north));
  return (EARTH_CIRCUMFERENCE * Math.cos(latitude)) / tiles / extent;
}
