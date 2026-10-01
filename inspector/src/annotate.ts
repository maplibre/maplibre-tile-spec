/** The one module that reaches for the wasm, so a test can stand in for it. */

export type {
  AnnotatedTile,
  BitField,
  BlobInfo,
  DecodedBlob,
  DecodeHint,
  DumpTree,
  Region,
} from "@maplibre/mlt-wasm";
export { annotateTile, tileGeoJson } from "@maplibre/mlt-wasm";
