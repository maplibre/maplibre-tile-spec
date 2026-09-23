import type { FeatureCollection } from "geojson";
import { wasmTileGeoJson } from "./wasm";

/**
 * Decode a raw MLT tile blob into `GeoJSON`.
 *
 * Every feature carries `_layer` and `_extent`, and a v2 tile's vertex-scoped columns ride
 * along as `m:`-prefixed arrays, one value per vertex.
 */
export function tileGeoJson(data: Uint8Array): FeatureCollection {
  return JSON.parse(wasmTileGeoJson(data)) as FeatureCollection;
}
