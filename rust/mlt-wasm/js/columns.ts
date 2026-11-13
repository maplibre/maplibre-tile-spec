import { wasmDecodeTileColumns } from "./wasm";

/** Mirrors `GeometryType` in mlt-core - preserves the single vs multi distinction that MVT collapses. */
export enum MltGeometryType {
  Point = 0,
  LineString = 1,
  Polygon = 2,
  MultiPoint = 3,
  MultiLineString = 4,
  MultiPolygon = 5,
  /** `(x, y, z)` vertices and the triangles between them, only in a v2 `Meshes` layer. */
  Mesh = 6,
}

/**
 * The array each column type comes as, one value per feature.
 *
 * `bool` comes as `0`/`1` in a `Uint8Array`. `i64` and `u64` come as `Float64Array`, so values
 * above `Number.MAX_SAFE_INTEGER` lose precision.
 */
interface MltColumnValuesByType {
  bool: Uint8Array;
  i8: Int8Array;
  u8: Uint8Array;
  i32: Int32Array;
  u32: Uint32Array;
  i64: Float64Array;
  u64: Float64Array;
  f32: Float32Array;
  f64: Float64Array;
  string: string[];
}

/** The type a property column was stored as. */
export type MltColumnType = keyof MltColumnValuesByType;

export type MltColumnValues = MltColumnValuesByType[MltColumnType];

/**
 * A slot whose feature has no value holds `0` or `""`; `present` says which slots those are.
 * `present` is one bit per feature, LSB-first, and is left out when every feature has a value.
 */
export interface MltColumn<V extends MltColumnValues = MltColumnValues> {
  readonly values: V;
  readonly present?: Uint8Array;
}

/** A property column. Checking `type` narrows `values` to its array. */
export type MltNamedColumn = {
  [T in MltColumnType]: MltColumn<MltColumnValuesByType[T]> & {
    readonly name: string;
    readonly type: T;
  };
}[MltColumnType];

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
  /**
   * Cumulative triangle counts per polygon feature, for a tessellated layer, or per feature
   * of a `Meshes` layer.
   */
  readonly triangleOffsets?: Uint32Array;
  /** Three vertex indices per triangle, counted from the layer's first vertex. */
  readonly indexBuffer?: Uint32Array;
  /** Set on a `Meshes` layer, which says so even when it holds no feature. */
  readonly meshes?: true;
}

/**
 * What a z of `0` stands for: mean sea level, or the terrain under each vertex. A `Meshes`
 * layer's terrain is the one at the centre of each mesh's `(x, y)` bounding box.
 */
export type HeightMode = "absolute" | "relativeToGround";

/** `(x, y)` vertices, which sit on the ground. */
export interface MltGeometryColumns2D extends MltGeometryColumnsBase {
  readonly dimension: 2;
  readonly zStep?: undefined;
  readonly heightMode?: undefined;
}

/** `(x, y, z)` vertices. */
export interface MltGeometryColumns3D extends MltGeometryColumnsBase {
  readonly dimension: 3;
  /** The z grid as the power of ten of its step in metres; `toElevation` gives a z in metres. */
  readonly zStep: number;
  readonly heightMode: HeightMode;
}

/**
 * What an m-value column runs over: the vertices, or, in a `Meshes` layer, the triangles.
 */
export type MltMValueScope = "vertex" | "triangle";

/**
 * An m-value column of a v2 layer. A `"vertex"`-scoped column's values run over the vertices of
 * the features that carry it, in feature order: when `present` is left out, that is one value
 * per vertex of the layer, aligned with `geometry.vertices`. A `"triangle"`-scoped column of a
 * `Meshes` layer runs over the triangles the same way, aligned with `geometry.indexBuffer`
 * three indices per value. `present` has one bit per feature, LSB-first; a feature whose bit is
 * clear carries no values at all.
 *
 * A string column comes as one dictionary code per value, and `dictionary[code]` is the string.
 */
export type MltMValueColumn =
  | {
      [T in Exclude<MltColumnType, "string">]: {
        readonly name: string;
        readonly scope: MltMValueScope;
        readonly type: T;
        readonly values: MltColumnValuesByType[T];
        readonly present?: Uint8Array;
      };
    }[Exclude<MltColumnType, "string">]
  | {
      readonly name: string;
      readonly scope: MltMValueScope;
      readonly type: "string";
      readonly codes: Uint32Array;
      readonly dictionary: readonly string[];
      readonly present?: Uint8Array;
    };

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
  /** A v2 layer's m-value columns, in wire order; left out for a v1 layer. */
  readonly mValues?: readonly MltMValueColumn[];
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
  // wasm-bindgen copies the array into Rust, so it never sees the caller's readonly one.
  const names = options.layers as string[] | undefined;
  return { layers: wasmDecodeTileColumns(data, names) as MltColumnLayer[] };
}

/**
 * Feature `index`'s value in `column`, or `undefined` when the feature has none.
 *
 * Throws when `index` is not a feature, so that `undefined` only ever means no value.
 */
export function columnValue<C extends MltColumn>(
  column: C,
  index: number,
): C["values"][number] | undefined {
  const { values, present } = column;
  if (!Number.isInteger(index) || index < 0 || index >= values.length) {
    throw new RangeError(`the column has no feature ${index}`);
  }
  const has =
    present === undefined || ((present[index >> 3] >> (index & 7)) & 1) === 1;
  return has ? values[index] : undefined;
}

/**
 * A z of a 3D layer in metres above the layer's `heightMode` base, `z * 10 ** zStep`, as mlt-core's `ZStep::elevation`:
 * a fine grid divides by an exact integer, so `1234` on the `-2` grid is exactly `12.34`.
 */
export function toElevation(z: number, zStep: number): number {
  const power = 10 ** Math.abs(zStep);
  return zStep < 0 ? z / power : z * power;
}
