/**
 * The one module that imports the wasm package.
 *
 * Safari 15-26 cannot have two modules import the same top-level-await module, so every
 * other module imports the already-initialised bindings from here.
 */
export {
  annotateTile as wasmAnnotateTile,
  decode_tile as wasmDecodeTile,
  tileGeoJson as wasmTileGeoJson,
} from "../pkg/mlt_wasm.js";
