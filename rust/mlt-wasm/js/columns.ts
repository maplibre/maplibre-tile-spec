import { wasmDecodeTileColumns } from "./wasm";

/** The type a property or m-value column was stored as. */
export type MltColumnType =
  | "bool"
  | "i8"
  | "u8"
  | "i32"
  | "u32"
  | "i64"
  | "u64"
  | "f32"
  | "f64"
  | "string";

/**
 * One value per feature (per vertex for an m-value column).
 *
 * `bool` comes as `0`/`1` in a `Uint8Array`. `i64` and `u64` come as `Float64Array`, so values
 * above `Number.MAX_SAFE_INTEGER` lose precision.
 */
export type MltColumnValues =
  | Uint8Array
  | Int8Array
  | Int32Array
  | Uint32Array
  | Float32Array
  | Float64Array
  | string[];

/**
 * A slot whose feature has no value holds `0` or `""`; `present` says which slots those are.
 * `present` is one bit per feature, LSB-first, and is left out when every feature has a value.
 */
export interface MltColumn<V extends MltColumnValues = MltColumnValues> {
  readonly values: V;
  readonly present?: Uint8Array;
}

/** A property or m-value column. */
export interface MltNamedColumn extends MltColumn {
  readonly name: string;
  readonly type: MltColumnType;
}

/**
 * A layer's geometry as decoded, 2D or 3D by `dimension`.
 *
 * Offset arrays are cumulative: entry `i` starts item `i` and entry `i + 1` ends it.
 * Each level indexes into the next, and the last one present indexes vertices.
 * An array a layer does not need is left out.
 */
export type MltGeometryColumns = MltGeometryColumns2D | MltGeometryColumns3D;

interface MltGeometryColumnsBase {
  /** The MLT geometry type of each feature, see `MltGeometryType`. */
  readonly types: Uint8Array;
  readonly geometryOffsets?: Uint32Array;
  readonly partOffsets?: Uint32Array;
  readonly ringOffsets?: Uint32Array;
  /**
   * `dimension` words per vertex, in tile coordinates, interleaved as the wire stores them.
   * A polygon ring's closing vertex is not stored.
   */
  readonly vertices: Int32Array;
  /** Cumulative triangle counts per polygon feature, for a tessellated layer. */
  readonly triangleOffsets?: Uint32Array;
  /** Three vertex indices per triangle, counted from the layer's first vertex. */
  readonly indexBuffer?: Uint32Array;
}

/** `(x, y)` vertices. */
export interface MltGeometryColumns2D extends MltGeometryColumnsBase {
  readonly dimension: 2;
  readonly zStep?: undefined;
}

/** `(x, y, z)` vertices. */
export interface MltGeometryColumns3D extends MltGeometryColumnsBase {
  readonly dimension: 3;
  /** The z grid as the power of ten of its step in metres: `-10000 + z * 10 ** zStep` metres. */
  readonly zStep: number;
}

export interface MltColumnLayer {
  readonly name: string;
  readonly extent: number;
  /** The wire version the layer was stored in. */
  readonly version: 1 | 2;
  readonly featureCount: number;
  readonly geometry: MltGeometryColumns;
  /** Left out when the layer has no id column. */
  readonly ids?: MltColumn<Float64Array>;
  /**
   * In wire order, a shared dictionary's children named by its prefix and their own names.
   * v2 nested columns are not passed on.
   */
  readonly properties: readonly MltNamedColumn[];
  /** v2 vertex-scoped columns, one value per vertex in `geometry.vertices` order. */
  readonly mValues: readonly MltNamedColumn[];
}

export interface MltColumnTile {
  /** In wire order; layer names need not be unique. */
  readonly layers: readonly MltColumnLayer[];
}

export interface DecodeTileColumnsOptions {
  /**
   * Decode only the layers with one of these names, every one that has it. The other
   * layers are skipped without being decoded. Every layer is decoded when left out.
   */
  readonly layers?: readonly string[];
}

/** Decode the layers of a tile into typed arrays. */
export function decodeTileColumns(
  data: Uint8Array,
  options: DecodeTileColumnsOptions = {},
): MltColumnTile {
  const names = options.layers === undefined ? undefined : [...options.layers];
  return { layers: wasmDecodeTileColumns(data, names) as MltColumnLayer[] };
}

/** Whether entry `index` of `column` holds a value. */
export function isPresent(column: MltColumn, index: number): boolean {
  const { present } = column;
  return (
    present === undefined || ((present[index >> 3] >> (index & 7)) & 1) === 1
  );
}
