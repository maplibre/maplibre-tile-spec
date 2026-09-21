import { wasmAnnotateTile } from "./wasm";

/** A tile annotated as far as the walk got, plus the error that stopped it. */
export interface AnnotatedTile {
  /** Regions of the whole tile, or of one top-level layer. Memoized per argument. */
  tree(layer?: number): DumpTree;
  /** Walker failure message, or null. When set, `tree()` is partial. */
  readonly error: string | null;
  /** Decoded values of the data blob at `regionIndex`, at most `maxValues` of them (0 = all). */
  decodeBlob(regionIndex: number, maxValues: number): DecodedBlob;
  /** Release the tile bytes and the region list. */
  free(): void;
}

export interface DumpTree {
  bufLen: number;
  /** Pre-order: containers before their children. Leaves partition the buffer exactly. */
  regions: Region[];
}

export interface Region {
  offset: number;
  len: number;
  depth: number;
  label: string;
  value: string | null;
  bits: BitField[];
  kind: "meta" | "dataBlob";
  container: boolean;
  blob: BlobInfo | null;
}

export interface BitField {
  /** Inclusive high bit index, MSB first (7..=0). */
  hi: number;
  lo: number;
  raw: number;
  meaning: string;
}

export interface BlobInfo {
  streamType: string;
  logical: string;
  physical: string;
  numValues: number;
  hint: DecodeHint;
}

export type DecodeHint =
  | { kind: "presence" }
  | { kind: "bool" }
  | { kind: "i32" }
  | { kind: "u32" }
  | { kind: "i64" }
  | { kind: "u64" }
  | { kind: "f32" }
  | { kind: "f64" }
  | { kind: "bytes" }
  | { kind: "packedBits" }
  | { kind: "alp"; e: number; f: number; base: bigint };

export type DecodedBlob =
  | { kind: "numbers"; values: number[]; truncatedFrom: number | null }
  | { kind: "bigints"; values: bigint[]; truncatedFrom: number | null }
  | { kind: "bools"; values: boolean[]; truncatedFrom: number | null }
  | { kind: "text"; value: string }
  | { kind: "binary"; len: number }
  | { kind: "error"; message: string };

/**
 * Annotate a raw MLT tile blob, region by region.
 *
 * Never throws for a malformed tile: the walk hands back what it annotated, `error` carries the
 * failure, and the tree ends in an `<unannotated>` leaf covering the bytes it never reached.
 */
export function annotateTile(data: Uint8Array): AnnotatedTile {
  return wasmAnnotateTile(data) as unknown as AnnotatedTile;
}
