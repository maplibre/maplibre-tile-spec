/**
 * The one module that imports the wasm package.
 *
 * Safari 15-26 cannot have two modules import the same top-level-await module,
 * so every other module imports the already-initialised bindings from here.
 * Optional Cargo features (`annotate`, `geojson`) are resolved dynamically so
 * stripped decoder-only WASM builds still type-check and link cleanly.
 */
import * as wasm from "../pkg/mlt_wasm.js";

export const wasmDecodeTileColumns = wasm.decodeTileColumns;

interface OptionalWasm {
  annotateTile?: (data: Uint8Array) => unknown;
  tileGeoJson?: (data: Uint8Array) => string;
}

export function wasmAnnotateTile(data: Uint8Array): unknown {
  const fn = (wasm as OptionalWasm).annotateTile;
  if (typeof fn !== "function") {
    throw new Error(
      "annotateTile requires mlt-wasm built with the `annotate` Cargo feature",
    );
  }
  return fn(data);
}

export function wasmTileGeoJson(data: Uint8Array): string {
  const fn = (wasm as OptionalWasm).tileGeoJson;
  if (typeof fn !== "function") {
    throw new Error(
      "tileGeoJson requires mlt-wasm built with the `geojson` Cargo feature",
    );
  }
  return fn(data);
}
