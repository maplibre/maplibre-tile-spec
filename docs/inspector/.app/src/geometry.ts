/** What the geometry panel reads out of the GeoJSON `tileGeoJson` hands back. */

import type { Feature, FeatureCollection, Geometry } from "geojson";

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
