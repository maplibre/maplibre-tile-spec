import type { FeatureCollection } from "geojson";
import { wasmTileGeoJson } from "./wasm";

/**
 * Decode a raw MLT tile blob into `GeoJSON`.
 *
 * Every feature carries `_layer` and `_extent`; a v2 tile's m-value columns
 * ride along as `m:`-prefixed arrays (one value per vertex or per mesh
 * triangle), and mesh features carry a top-level `triangles` member.
 */
export function tileGeoJson(data: Uint8Array): FeatureCollection {
  return JSON.parse(wasmTileGeoJson(data)) as FeatureCollection;
}
